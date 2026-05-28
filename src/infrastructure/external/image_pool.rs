use std::time::Duration;

use anyhow::Context;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_s3::operation::delete_objects::DeleteObjectsError;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use aws_sdk_s3::{Client, Config};
use url::Url;

use crate::domain::external::image_pool::{ImageDelete, ImageGet, ImagePut};
use crate::domain::result::{DomainErr, DomainResl};
use crate::util::err::ErrorTrace as _;
use tracing::Level;

// ---------------------------------------------------------------------------
// R2OssClient
// ---------------------------------------------------------------------------

/// Cloudflare R2-backed image pool client.
///
/// Reads configuration from environment variables:
/// - `R2_ACCOUNT_ID`     — required
/// - `R2_ACCESS_KEY_ID`  — required
/// - `R2_SECRET_ACCESS_KEY` — required
/// - `R2_REGION`         — optional, defaults to `"auto"`
/// - `R2_BUCKET_NAME`    — required
/// - `R2_CUSTOM_DOMAIN`  — required for GET URL generation
#[derive(Clone)]
pub struct R2ImagePool {
    client: Client,
    bucket: String,
    domain: String,
}

impl R2ImagePool {
    pub fn from_env() -> anyhow::Result<Self> {
        let account_id = std::env::var("R2_ACCOUNT_ID")
            .context("[R2OssClient::new] R2_ACCOUNT_ID is not set")?;
        let access_key_id = std::env::var("R2_ACCESS_KEY_ID")
            .context("[R2OssClient::new] R2_ACCESS_KEY_ID is not set")?;
        let secret_access_key = std::env::var("R2_SECRET_ACCESS_KEY")
            .context("[R2OssClient::new] R2_SECRET_ACCESS_KEY is not set")?;
        let region = std::env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string());
        let bucket = std::env::var("R2_BUCKET_NAME")
            .context("[R2OssClient::new] R2_BUCKET_NAME is not set")?;
        let domain = std::env::var("R2_CUSTOM_DOMAIN")
            .context("[R2OssClient::new] R2_CUSTOM_DOMAIN is not set")?;

        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);

        let credentials = Credentials::new(access_key_id, secret_access_key, None, None, "r2");

        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .endpoint_url(&endpoint)
            .credentials_provider(credentials)
            .build();

        tracing::debug!(
            bucket = %bucket,
            domain = %domain,
            endpoint = %endpoint,
            "[R2OssClient::new] configured",
        );

        Ok(Self {
            client: Client::from_conf(config),
            bucket,
            domain,
        })
    }
}

// ---------------------------------------------------------------------------
// ImageGet
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl ImageGet for R2ImagePool {
    #[tracing::instrument(skip(self), level = Level::DEBUG)]
    async fn get_signed(&self, key: &str) -> DomainResl<Url> {
        if self.domain.is_empty() {
            return Err(DomainErr::unrecoverable(
                "[R2OssClient::get_signed] custom domain is not configured".into(),
            ));
        }

        let url_str = format!("{}/{}", self.domain.trim_end_matches('/'), key);
        Url::parse(&url_str)
            .map_err(|e| {
                DomainErr::unrecoverable(format!(
                    "[R2OssClient::get_signed] failed to parse URL '{}': {}",
                    url_str, e
                ))
            })
            .trace_error()
    }
}

// ---------------------------------------------------------------------------
// ImagePut
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl ImagePut for R2ImagePool {
    #[tracing::instrument(skip(self), level = Level::DEBUG)]
    async fn put_signed(&self, key: &str) -> DomainResl<Url> {
        const EXPIRATION: Duration = Duration::from_secs(600); // 10 minutes

        let content_type = detect_content_type(key).ok_or_else(|| {
            DomainErr::expected_argument(format!(
                "[R2OssClient::put_signed] unsupported file type for key: {}",
                key
            ))
        })?;

        let presigned_config = PresigningConfig::expires_in(EXPIRATION).map_err(|e| {
            DomainErr::unrecoverable(format!(
                "[R2OssClient::put_signed] failed to build presigning config: {}",
                e
            ))
        })?;

        let presigned_request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigned_config)
            .await
            .map_err(|e| {
                DomainErr::unrecoverable(format!(
                    "[R2OssClient::put_signed] failed to generate presigned put URL: {}",
                    e
                ))
            })
            .trace_error()?;

        Url::parse(presigned_request.uri()).map_err(|e| {
            DomainErr::unrecoverable(format!(
                "[R2OssClient::put_signed] failed to parse presigned URI: {}",
                e
            ))
        })
    }
}

// ---------------------------------------------------------------------------
// ImageDelete
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl ImageDelete for R2ImagePool {
    #[tracing::instrument(skip(self), level = Level::DEBUG)]
    async fn delete_batch(&self, keys: &[&str]) -> DomainResl<()> {
        const MAX_RETRY: usize = 3;
        const RETRY_DELAY: Duration = Duration::from_secs(1);

        // At current stage, we do not use an exponential backoff strategy for
        // retrying failed deletions, as the number of keys in a batch is
        // expected to be small (usually less than 10), and the likelihood of
        // transient errors is relatively low.

        let obj_ids: Vec<ObjectIdentifier> = keys
            .iter()
            .map(|k| {
                ObjectIdentifier::builder()
                    .key(*k)
                    .build()
                    .expect("ObjectIdentifier build should never fail")
            })
            .collect();

        let mut last_err: Option<String> = None;

        for attempt in 0..MAX_RETRY {
            if attempt > 0 {
                tokio::time::sleep(RETRY_DELAY).await;
            }

            let delete = Delete::builder()
                .set_objects(Some(obj_ids.clone()))
                .quiet(true)
                .build()
                .expect("Delete build should never fail");

            let result = self
                .client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(delete)
                .send()
                .await;

            match result {
                Ok(output) => {
                    // Collect errors that are *not* NoSuchKey.
                    let non_not_found: Vec<_> = output
                        .errors()
                        .iter()
                        .filter(|e| e.code() != Some("NoSuchKey"))
                        .collect();

                    if non_not_found.is_empty() {
                        return Ok(());
                    }

                    last_err = Some(format!(
                        "[R2OssClient::delete_batch] partial failure: {:?}",
                        non_not_found
                    ));
                    // fall through to retry
                }
                Err(e) => {
                    // If the entire batch returned NoSuchKey, treat as success.
                    if is_no_such_key_error(&e) {
                        return Ok(());
                    }

                    last_err = Some(format!("[R2OssClient::delete_batch] {}", e));
                    // fall through to retry
                }
            }
        }

        Err(DomainErr::unrecoverable(
            last_err.unwrap_or_else(|| "unknown error".into()),
        ))
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Detects the MIME content type from a file extension.
fn detect_content_type(key: &str) -> Option<&'static str> {
    let ext = key.rsplit('.').next()?.to_lowercase();

    match ext.as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        "avif" => Some("image/avif"),
        "bmp" => Some("image/bmp"),
        "tif" | "tiff" => Some("image/tiff"),
        _ => None,
    }
}

/// Checks whether the top-level S3 error is `NoSuchKey`.
fn is_no_such_key_error(err: &SdkError<DeleteObjectsError>) -> bool {
    if let SdkError::ServiceError(service_err) = err {
        return service_err.err().code() == Some("NoSuchKey");
    }
    false
}
