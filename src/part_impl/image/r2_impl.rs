//! Cloudflare R2-backed image URL signer.

use std::collections::BTreeMap;
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

use poprako_util::i18n::trl;

use crate::part::image::{
    ImageManager, ImagePool, ImageUploadSlot, ImageUploadSpec,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

#[cfg(test)]
// Executes lightweight unit tests for URL generation and upload content handling.
mod tests;

// Expiration duration for presigned upload URLs (10 minutes).
const PUT_SIGNED_EXPIRATION: Duration = Duration::from_secs(600);

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
    pub fn new(client: Client, bucket: String, domain: String) -> Self {
        Self {
            client,
            bucket,
            domain,
        }
    }

    /// Reads Cloudflare R2 settings from environment variables.
    pub fn from_env() -> anyhow::Result<Self> {
        //
        // Internal implementation detail.
        let (account_id, access_key_id) = (
            std::env::var("R2_ACCOUNT_ID").with_context(
                || "[R2ImagePool::from_env] R2_ACCOUNT_ID is not set",
            )?,
            std::env::var("R2_ACCESS_KEY_ID").with_context(
                || "[R2ImagePool::from_env] R2_ACCESS_KEY_ID is not set",
            )?,
        );

        let (secret_access_key, region) = (
            std::env::var("R2_SECRET_ACCESS_KEY").with_context(
                || "[R2ImagePool::from_env] R2_SECRET_ACCESS_KEY is not set",
            )?,
            std::env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string()),
        );

        let (bucket, domain) = (
            std::env::var("R2_BUCKET_NAME").with_context(
                || "[R2ImagePool::from_env] R2_BUCKET_NAME is not set",
            )?,
            std::env::var("R2_CUSTOM_DOMAIN").with_context(
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
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `gen_download_url`.
    async fn gen_download_url(&self, key: &str) -> BaseRest<Url> {
        build_public_url(&self.domain, key, "gen_download_url")
    }

    #[instrument(level = "info", err(Debug), skip_all)]
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

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `get_upload_url`.
    async fn get_upload_url(&self, key: &str) -> BaseRest<Url> {
        //
        // Internal implementation detail.
        let (content_type, presigning_config) = (
            detect_content_type(key).ok_or_else(|| BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-unsupported-file-type"),
            })?,
            PresigningConfig::expires_in(PUT_SIGNED_EXPIRATION).map_err(
                |err| BaseError::Unrecoverable {
                    message: format!(
                        "[R2ImagePool::get_upload_url] failed to build presigning config: {}",
                        err
                    ),
                },
            )?,
        );

        let presigned_request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning_config)
            .await
            .map_err(|err| BaseError::Unrecoverable {
                message: format!(
                    "[R2ImagePool::get_upload_url] failed to generate presigned put URL: {}",
                    err
                ),
            })?;

        Url::parse(presigned_request.uri()).map_err(|err| BaseError::Unrecoverable {
            message: format!(
                "[R2ImagePool::get_upload_url] failed to parse presigned URI: {}",
                err
            ),
        })
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `get_upload_slot`.
    async fn get_upload_slot(
        &self,
        spec: ImageUploadSpec<'_>,
    ) -> BaseRest<ImageUploadSlot> {
        //
        // Internal implementation detail.
        let (content_length, presigning_config) = (
            i64::try_from(spec.content_length).map_err(|_| {
                BaseError::Unrecoverable {
                    message:
                        "[R2ImagePool::get_upload_slot] content length exceeds i64"
                            .into(),
                }
            })?,
            PresigningConfig::expires_in(PUT_SIGNED_EXPIRATION).map_err(
                |err| BaseError::Unrecoverable {
                    message: format!(
                        "[R2ImagePool::get_upload_slot] failed to build presigning config: {}",
                        err
                    ),
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
            .map_err(|err| BaseError::Unrecoverable {
                message: format!(
                    "[R2ImagePool::get_upload_slot] failed to generate presigned put URL: {}",
                    err
                ),
            })?;

        let url = Url::parse(presigned_request.uri()).map_err(|err| {
            BaseError::Unrecoverable {
                message: format!(
                    "[R2ImagePool::get_upload_slot] failed to parse presigned URI: {}",
                    err
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
    #[instrument(level = "info", err(Debug), skip_all)]
    // Removes a previously uploaded object from the R2 bucket.
    async fn delete_object(&self, key: &str) -> BaseRest<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| BaseError::Unrecoverable {
                message: format!(
                    "[R2ImagePool::delete_object] failed to delete '{}': {}",
                    key, e
                ),
            })
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    // Performs a HEAD request to determine whether an object exists in R2.
    async fn object_exists(&self, key: &str) -> BaseRest<bool> {
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

            Err(e) => Err(BaseError::Unrecoverable {
                message: format!(
                    "[R2ImagePool::object_exists] failed to check '{}': {}",
                    key, e
                ),
            }),
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
        return Err(BaseError::Unrecoverable {
            message: format!(
                "[R2ImagePool::{}] custom domain is not configured",
                operation
            ),
        });
    }

    let domain = domain.trim_end_matches('/');

    let url_string =
        match domain.starts_with("http://") || domain.starts_with("https://") {
            //
            // Internal implementation detail.
            true => {
                format!("{}/{}", domain, path)
            }

            false => {
                format!("https://{}/{}", domain, path)
            }
        };

    Url::parse(&url_string).map_err(|err| BaseError::Unrecoverable {
        message: format!(
            "[R2ImagePool::{}] failed to parse URL '{}': {}",
            operation, url_string, err
        ),
    })
}

// Maps a file extension to its MIME content type for upload requests.
fn detect_content_type(key: &str) -> Option<&'static str> {
    //
    // Internal implementation detail.
    let ext = key.rsplit('.').next()?.to_lowercase();

    match ext.as_str() {
        //
        // Internal implementation detail.
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
