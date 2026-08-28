//! Cloudflare R2-backed image URL signer.

#[cfg(test)]
// Executes lightweight unit tests for URL generation and upload content handling.
mod tests;

use std::collections::BTreeMap;
use std::env::var;
use std::time::Duration;

use anyhow::Context as _;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::{Client, Config};
use tracing::instrument;
use url::Url;

use crate::part::image::{
    ImageManager, ImagePool, ImageUploadSlot, ImageUploadSpec,
};
use crate::result::{BaseError, BaseRest, accept};

// Expiration duration for presigned upload URLs (10 minutes).
const PUT_SIGNED_EXPIRATION: Duration = Duration::from_mins(10);

// Cloudflare Image Resizing options for public thumbnail URLs.
const THUMBNAIL_TRANSFORM: &str =
    "width=300,fit=scale-down,quality=80,format=auto,metadata=none";

/// Cloudflare R2-backed image pool.
#[derive(Clone)]
pub struct R2ImagePool {
    //
    // Internal state field `client`.
    /// HTTP client configured for Cloudflare R2 API requests.
    client: Client,
    /// Name of the R2 bucket used for image storage.
    bucket: String,
    /// Public domain serving image URLs.
    domain: String,
}

impl R2ImagePool {
    /// Creates an image pool from an already configured S3-compatible client.
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
                || "[R2ImagePool::from_env] R2_ACCOUNT_ID is not set",
            )?,
            var("R2_ACCESS_KEY_ID").with_context(
                || "[R2ImagePool::from_env] R2_ACCESS_KEY_ID is not set",
            )?,
        );

        let (secret_access_key, region) = (
            var("R2_SECRET_ACCESS_KEY").with_context(
                || "[R2ImagePool::from_env] R2_SECRET_ACCESS_KEY is not set",
            )?,
            var("R2_REGION").unwrap_or_else(|_| "auto".to_string()),
        );

        let (bucket, domain) = (
            var("R2_BUCKET_NAME").with_context(
                || "[R2ImagePool::from_env] R2_BUCKET_NAME is not set",
            )?,
            var("R2_CUSTOM_DOMAIN").with_context(
                || "[R2ImagePool::from_env] R2_CUSTOM_DOMAIN is not set",
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

impl ImagePool for R2ImagePool {
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `gen_download_url`.
    async fn gen_download_url(&self, key: &str) -> BaseRest<Url> {
        build_public_url(&self.domain, key, "gen_download_url")
    }

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `gen_thumbnail_download_url`.
    async fn gen_thumbnail_download_url(
        &self,
        original_key: &str,
    ) -> BaseRest<Url> {
        //
        // Internal implementation detail.
        let thumbnail_path =
            format!("cdn-cgi/image/{}/{}", THUMBNAIL_TRANSFORM, original_key);

        build_public_url(
            &self.domain,
            &thumbnail_path,
            "gen_thumbnail_download_url",
        )
    }

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `get_upload_slot`.
    async fn get_upload_slot(
        &self,
        spec: ImageUploadSpec<'_>,
    ) -> BaseRest<ImageUploadSlot> {
        //
        // Internal implementation detail.
        let (content_length, presigning_config) = (
            i64::try_from(spec.content_length).map_err(|_| {
                //
                BaseError::Unrecoverable {
                    message:
                        "[R2ImagePool::get_upload_slot] content length exceeds i64"
                            .into(),
                }
            })?,
            PresigningConfig::expires_in(PUT_SIGNED_EXPIRATION).map_err(
                |err| {
                    //
                    tracing::error!(
                        operation = "get_upload_slot",
                        sdk_err = ?err,
                        "R2 SDK presigning configuration error",
                    );

                    BaseError::Unrecoverable {
                        message: format!(
                            "[R2ImagePool::get_upload_slot] failed to build presigning config: {}",
                            err,
                        ),
                    }
                },
            )?,
        );

        let presigned_request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(spec.object_key)
            .content_type(spec.content_type)
            .content_length(content_length)
            .presigned(presigning_config)
            .await
            .map_err(|err| {
                //
                tracing::error!(
                    operation = "get_upload_slot",
                    sdk_err = ?err,
                    "R2 SDK presigning error",
                );

                BaseError::Unrecoverable {
                        message: format!(
                            "[R2ImagePool::get_upload_slot] failed to generate presigned put URL: {}",
                            err,
                        ),
                }
            })?;

        let url = Url::parse(presigned_request.uri()).map_err(|err| {
            //
            tracing::error!(
                operation = "get_upload_slot",
                sdk_err = ?err,
                "URL SDK parsing error",
            );

            BaseError::Unrecoverable {
                message: format!(
                    "[R2ImagePool::get_upload_slot] failed to parse presigned URI: {}",
                    err,
                ),
            }
        })?;

        let mut headers = BTreeMap::new();

        headers.insert("content-length".into(), content_length.to_string());

        headers.insert("content-type".into(), spec.content_type.into());

        accept(ImageUploadSlot { url, headers })
    }
}

impl ImageManager for R2ImagePool {
    #[instrument(level = "info", skip_all)]
    // Removes a previously uploaded object from the R2 bucket.
    async fn delete_object(&self, key: &str) -> BaseRest<()> {
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

                BaseError::Unrecoverable {
                    message: format!(
                        "[R2ImagePool::delete_object] failed to delete '{}': {}",
                        key,
                        err,
                    ),
                }
            })
    }

    #[instrument(level = "info", skip_all)]
    // Performs a HEAD request to determine whether an object exists in R2.
    async fn object_exists(&self, key: &str) -> BaseRest<bool> {
        //
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => accept(true),

            Err(SdkError::ServiceError(e))
                if matches!(e.err(), HeadObjectError::NotFound(_)) =>
            {
                accept(false)
            }

            Err(err) => {
                //
                tracing::error!(
                    operation = "object_exists",
                    sdk_err = ?err,
                    "R2 SDK request error",
                );

                Err(BaseError::Unrecoverable {
                    message: format!(
                        "[R2ImagePool::object_exists] failed to check '{}': {}",
                        key, err,
                    ),
                })
            }
        }
    }
}

// Builds a URL under the configured public image domain.
fn build_public_url(
    domain: &str,
    path: &str,
    operation: &str,
) -> BaseRest<Url> {
    //
    // Internal implementation detail.
    if domain.is_empty() {
        //
        return Err(BaseError::Unrecoverable {
            message: format!(
                "[R2ImagePool::{}] custom domain is not configured",
                operation,
            ),
        });
    }

    let domain = domain.trim_end_matches('/');

    let url_string =
        if domain.starts_with("http://") || domain.starts_with("https://") {
            format!("{}/{}", domain, path)
        } else {
            format!("https://{}/{}", domain, path)
        };

    Url::parse(&url_string).map_err(|err| {
        //
        tracing::error!(
            operation,
            sdk_err = ?err,
            "URL SDK parsing error",
        );

        BaseError::Unrecoverable {
            message: format!(
                "[R2ImagePool::{}] failed to parse URL '{}': {}",
                operation, url_string, err,
            ),
        }
    })
}
