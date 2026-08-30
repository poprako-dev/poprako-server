//! Cloudflare R2 support for the `ObjDept` implementation.

#[cfg(test)]
// Executes lightweight unit tests for URL generation and upload content handling.
mod tests;

use std::collections::BTreeMap;
use std::env::var;
use std::time::{Duration, SystemTime};

use anyhow::Context as _;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::{Client, Config};
use time::OffsetDateTime;
use tracing::instrument;
use url::Url;

use poprako_obj_dept::model::slot::ObjPoolSlot;
use poprako_obj_dept::pool::{ObjPool, ObjPoolView};
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};

// Expiration duration for presigned upload URLs (10 minutes).
const PUT_SIGNED_EXPIRATION: Duration = Duration::from_mins(10);

/// Cloudflare R2-backed physical object pool.
#[derive(Clone)]
pub struct R2ObjPool {
    //
    // Internal state field `client`.
    /// HTTP client configured for Cloudflare R2 API requests.
    client: Client,
    /// Name of the R2 bucket used for image storage.
    bucket: String,
    /// Public domain serving image URLs.
    domain: String,
}

impl R2ObjPool {
    /// Creates an object pool from an already configured S3-compatible client.
    #[must_use]
    pub const fn new(client: Client, bucket: String, domain: String) -> Self {
        //
        Self {
            client,
            bucket,
            domain,
        }
    }

    /// Reads Cloudflare R2 settings from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error when required settings are missing or the client
    /// cannot be constructed.
    pub fn from_env() -> anyhow::Result<Self> {
        //
        // Internal implementation detail.
        let (account_id, access_key_id) = (
            var("R2_ACCOUNT_ID").with_context(
                || "[R2ObjPool::from_env] R2_ACCOUNT_ID is not set",
            )?,
            var("R2_ACCESS_KEY_ID").with_context(
                || "[R2ObjPool::from_env] R2_ACCESS_KEY_ID is not set",
            )?,
        );

        let (secret_access_key, region) = (
            var("R2_SECRET_ACCESS_KEY").with_context(
                || "[R2ObjPool::from_env] R2_SECRET_ACCESS_KEY is not set",
            )?,
            var("R2_REGION").unwrap_or_else(|_| "auto".to_string()),
        );

        let (bucket, domain) = (
            var("R2_BUCKET_NAME").with_context(
                || "[R2ObjPool::from_env] R2_BUCKET_NAME is not set",
            )?,
            var("R2_CUSTOM_DOMAIN").with_context(
                || "[R2ObjPool::from_env] R2_CUSTOM_DOMAIN is not set",
            )?,
        );

        let endpoint =
            format!("https://{}.r2.cloudflarestorage.com", account_id);

        let credentials = Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "r2",
        );

        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .endpoint_url(endpoint)
            .credentials_provider(credentials)
            .build();

        Ok(Self::new(Client::from_conf(config), bucket, domain))
    }
}

impl ObjPoolView for R2ObjPool {
    #[instrument(level = "info", skip_all)]
    // Generates one public object URL.
    async fn gen_url(&self, key: &str) -> ObjDeptRest<Url> {
        build_public_url(&self.domain, key)
    }

    #[instrument(level = "info", skip_all)]
    // Checks whether one object exists in R2.
    async fn has(&self, key: &str) -> ObjDeptRest<bool> {
        //
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),

            Err(SdkError::ServiceError(e))
                if matches!(e.err(), HeadObjectError::NotFound(_)) =>
            {
                Ok(false)
            }

            Err(err) => {
                //
                tracing::error!(
                    operation = "object_exists",
                    sdk_err = ?err,
                    "R2 SDK request error",
                );

                Err(ObjDeptError::Retryable {
                    message: "failed to check physical object".into(),
                })
            }
        }
    }
}

impl ObjPool for R2ObjPool {
    #[instrument(level = "info", skip_all)]
    // Generates one signed upload capability.
    async fn gen_slot(
        &self,
        key: &str,
        content_type: &str,
        byte_len: u64,
    ) -> ObjDeptRest<ObjPoolSlot> {
        //
        // Internal implementation detail.
        let signed_at = SystemTime::now();

        let expires_at =
            OffsetDateTime::from(signed_at + PUT_SIGNED_EXPIRATION);

        let (content_length, presigning_config) = (
            i64::try_from(byte_len).map_err(|_| {
                //
                ObjDeptError::Invalid {
                    message: "object byte length exceeds i64".into(),
                }
            })?,
            PresigningConfig::builder()
                .expires_in(PUT_SIGNED_EXPIRATION)
                .build()
                .map_err(|err| {
                    //
                    tracing::error!(
                        operation = "gen_obj_slot",
                        sdk_err = ?err,
                        "R2 SDK presigning configuration error",
                    );

                    ObjDeptError::Unrecoverable {
                        message: "failed to build object presigning config"
                            .into(),
                    }
                })?,
        );

        let presigned_request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .content_length(content_length)
            .presigned(presigning_config)
            .await
            .map_err(|err| {
                //
                tracing::error!(
                    operation = "gen_obj_slot",
                    sdk_err = ?err,
                    "R2 SDK presigning error",
                );

                ObjDeptError::Retryable {
                    message: "failed to generate object upload URL".into(),
                }
            })?;

        let url = Url::parse(presigned_request.uri()).map_err(|err| {
            //
            tracing::error!(
                operation = "gen_obj_slot",
                sdk_err = ?err,
                "URL SDK parsing error",
            );

            ObjDeptError::Unrecoverable {
                message: "failed to parse generated object upload URL".into(),
            }
        })?;

        let mut headers = BTreeMap::new();

        headers.insert("content-length".into(), content_length.to_string());

        headers.insert("content-type".into(), content_type.into());

        Ok(ObjPoolSlot {
            url,
            headers,
            expires_at,
        })
    }

    #[instrument(level = "info", skip_all)]
    // Deletes one object idempotently.
    async fn del(&self, key: &str) -> ObjDeptRest<()> {
        //
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map(|_| ())
            .map_err(|err| {
                //
                tracing::error!(
                    operation = "delete_object",
                    sdk_err = ?err,
                    "R2 SDK request error",
                );

                ObjDeptError::Retryable {
                    message: "failed to delete physical object".into(),
                }
            })
    }
}

// Parses one assembled public object URL.
fn parse_public_url(url_string: &str) -> ObjDeptRest<Url> {
    //
    Url::parse(url_string).map_err(|err| {
        //
        tracing::error!(
            operation = "gen_obj_url",
            sdk_err = ?err,
            "URL SDK parsing error",
        );

        ObjDeptError::Unrecoverable {
            message: "failed to parse physical object URL".into(),
        }
    })
}

// Builds a URL under the configured public object domain.
fn build_public_url(domain: &str, path: &str) -> ObjDeptRest<Url> {
    //
    if domain.is_empty() {
        //
        return Err(ObjDeptError::Unrecoverable {
            message: "object public domain is not configured".into(),
        });
    }

    let domain = domain.trim_end_matches('/');

    let has_scheme = domain.strip_prefix("http://").is_some()
        || domain.strip_prefix("https://").is_some();

    if has_scheme {
        return parse_public_url(&format!("{}/{}", domain, path));
    }

    parse_public_url(&format!("https://{}/{}", domain, path))
}
