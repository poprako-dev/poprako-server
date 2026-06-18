use std::time::Duration;

use anyhow::Context as _;
use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use aws_sdk_s3::{Client, Config};
use tracing::{Level, instrument};
use url::Url;

use poprako_util::i18n::trl;

use crate::domain::external::image_pool::{ImageDelete, ImageGet, ImageInspect, ImagePut};
use crate::domain::result::{DomainError, DomainResult};

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
pub struct OssImagePool {
    client: Client,
    bucket: String,
    domain: String,
}

impl OssImagePool {
    pub fn from_env_r2() -> anyhow::Result<Self> {
        let account_id = std::env::var("R2_ACCOUNT_ID")
            .with_context(|| "[R2OssClient::from_env] R2_ACCOUNT_ID is not set")?;
        let access_key_id = std::env::var("R2_ACCESS_KEY_ID")
            .with_context(|| "[R2OssClient::from_env] R2_ACCESS_KEY_ID is not set")?;
        let secret_access_key = std::env::var("R2_SECRET_ACCESS_KEY")
            .with_context(|| "[R2OssClient::from_env] R2_SECRET_ACCESS_KEY is not set")?;
        let region = std::env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string());
        let bucket = std::env::var("R2_BUCKET_NAME")
            .with_context(|| "[R2OssClient::from_env] R2_BUCKET_NAME is not set")?;
        let domain = std::env::var("R2_CUSTOM_DOMAIN")
            .with_context(|| "[R2OssClient::from_env] R2_CUSTOM_DOMAIN is not set")?;

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
            "[R2ImagePool::from_env] configured",
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

#[async_trait]
impl ImageGet for OssImagePool {
    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn get_signed(&self, key: &str) -> DomainResult<Url> {
        if self.domain.is_empty() {
            return Err(DomainError::unrecoverable(
                "[R2OssClient::get_signed] custom domain is not configured".into(),
            ));
        }

        let url_str = format!("{}/{}", self.domain.trim_end_matches('/'), key);
        Url::parse(&url_str).map_err(|e| {
            DomainError::unrecoverable(format!(
                "[R2OssClient::get_signed] failed to parse URL '{}': {}",
                url_str, e
            ))
        })
    }
}

// ---------------------------------------------------------------------------
// ImagePut
// ---------------------------------------------------------------------------

#[async_trait]
impl ImagePut for OssImagePool {
    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn put_signed(&self, key: &str) -> DomainResult<Url> {
        const EXPIRATION: Duration = Duration::from_secs(600); // 10 minutes

        let content_type = detect_content_type(key)
            .ok_or_else(|| DomainError::expected_argument(trl("error-unsupported-file-type")))?;

        let presigned_config = PresigningConfig::expires_in(EXPIRATION).map_err(|e| {
            DomainError::unrecoverable(format!(
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
                DomainError::unrecoverable(format!(
                    "[R2OssClient::put_signed] failed to generate presigned put URL: {}",
                    e
                ))
            })?;

        Url::parse(presigned_request.uri()).map_err(|e| {
            DomainError::unrecoverable(format!(
                "[R2OssClient::put_signed] failed to parse presigned URI: {}",
                e
            ))
        })
    }
}

// ---------------------------------------------------------------------------
// ImageDelete
// ---------------------------------------------------------------------------

#[async_trait]
impl ImageDelete for OssImagePool {
    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn delete_batch(&self, keys: &[&str]) -> DomainResult<()> {
        const MAX_DELETE_OBJECTS: usize = 1000;

        if keys.is_empty() {
            return Ok(());
        }

        for chunk in keys.chunks(MAX_DELETE_OBJECTS) {
            self.delete_chunk(chunk).await?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ImageInspect
// ---------------------------------------------------------------------------

#[async_trait]
impl ImageInspect for OssImagePool {
    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn exists(&self, key: &str) -> DomainResult<bool> {
        let result = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(e) if is_missing_object_error(&e) => Ok(false),
            Err(e) => Err(DomainError::unrecoverable(format!(
                "[R2OssClient::exists] failed to inspect object: {}",
                e
            ))),
        }
    }
}

impl OssImagePool {
    async fn delete_chunk(&self, keys: &[&str]) -> DomainResult<()> {
        const MAX_RETRY: usize = 3;
        const RETRY_DELAY: Duration = Duration::from_secs(1);

        let mut pending: Vec<_> = keys.iter().map(|key| key.to_string()).collect();
        let mut last_err: Option<String> = None;

        for attempt in 0..MAX_RETRY {
            if attempt > 0 {
                tokio::time::sleep(RETRY_DELAY).await;
            }

            let delete = build_delete_payload(&pending)?;

            let result = self
                .client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(delete)
                .send()
                .await;

            match result {
                Ok(output) => {
                    let mut failed_keys = Vec::new();
                    for e in output
                        .errors()
                        .iter()
                        .filter(|e| !is_already_deleted_error(e))
                    {
                        let key = e.key().map(|key| key.to_string()).ok_or_else(|| {
                            DomainError::unrecoverable(format!(
                                "[R2OssClient::delete_batch] missing key in delete error: {:?}",
                                e
                            ))
                        })?;
                        failed_keys.push(key);
                    }

                    if failed_keys.is_empty() {
                        return Ok(());
                    }

                    last_err = Some(format!(
                        "[R2OssClient::delete_batch] partial failure: {:?}",
                        output.errors()
                    ));

                    pending = failed_keys;
                }
                Err(e) => {
                    last_err = Some(format!("[R2OssClient::delete_batch] {}", e));
                }
            }
        }

        Err(DomainError::unrecoverable(last_err.unwrap_or_else(|| {
            "[R2OssClient::delete_batch] unknown error".into()
        })))
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

fn build_delete_payload(keys: &[String]) -> DomainResult<Delete> {
    let mut objects = Vec::new();
    for k in keys.iter() {
        let obj = ObjectIdentifier::builder().key(k).build().map_err(|e| {
            DomainError::unrecoverable(format!(
                "[R2OssClient::delete_batch] failed to build object identifier: {}",
                e
            ))
        })?;
        objects.push(obj);
    }

    Delete::builder()
        .set_objects(Some(objects))
        .quiet(true)
        .build()
        .map_err(|e| {
            DomainError::unrecoverable(format!(
                "[R2OssClient::delete_batch] failed to build delete payload: {}",
                e
            ))
        })
}

fn is_already_deleted_error(err: &aws_sdk_s3::types::Error) -> bool {
    err.code() == Some("NoSuchKey")
}

fn is_missing_object_error<E>(err: &E) -> bool
where
    E: aws_sdk_s3::error::ProvideErrorMetadata,
{
    matches!(err.code(), Some("NoSuchKey" | "NotFound" | "404"))
}
