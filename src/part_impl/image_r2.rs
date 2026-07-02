//! Cloudflare R2-backed image URL signer.

use std::time::Duration;

use anyhow::Context as _;
use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::{Client, Config};
use tracing::{Level, instrument};
use url::Url;

use poprako_util::i18n::trl;

use crate::part::image::ImagePool;
use crate::result::{ExpectedVariant, RegularError, RegularResult};

const PUT_SIGNED_EXPIRATION: Duration = Duration::from_secs(600);

/// Cloudflare R2-backed image pool.
#[derive(Clone)]
pub struct R2ImagePool {
    client: Client,
    bucket: String,
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
        let account_id = std::env::var("R2_ACCOUNT_ID")
            .with_context(|| "[R2ImagePool::from_env] R2_ACCOUNT_ID is not set")?;

        let access_key_id = std::env::var("R2_ACCESS_KEY_ID")
            .with_context(|| "[R2ImagePool::from_env] R2_ACCESS_KEY_ID is not set")?;

        let secret_access_key = std::env::var("R2_SECRET_ACCESS_KEY")
            .with_context(|| "[R2ImagePool::from_env] R2_SECRET_ACCESS_KEY is not set")?;

        let region = std::env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string());

        let bucket = std::env::var("R2_BUCKET_NAME")
            .with_context(|| "[R2ImagePool::from_env] R2_BUCKET_NAME is not set")?;

        let domain = std::env::var("R2_CUSTOM_DOMAIN")
            .with_context(|| "[R2ImagePool::from_env] R2_CUSTOM_DOMAIN is not set")?;

        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);

        let credentials = Credentials::new(access_key_id, secret_access_key, None, None, "r2");

        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .endpoint_url(endpoint)
            .credentials_provider(credentials)
            .build();

        Ok(Self::new(Client::from_conf(config), bucket, domain))
    }
}

#[async_trait]
impl ImagePool for R2ImagePool {
    #[instrument(err(Debug), skip(self), level = Level::DEBUG)]
    async fn get_signed(&self, key: &str) -> RegularResult<Url> {
        if self.domain.is_empty() {
            return Err(RegularError::Unrecoverable {
                message: "[R2ImagePool::get_signed] custom domain is not configured".to_string(),
            });
        }

        let url_string = format!("{}/{}", self.domain.trim_end_matches('/'), key);

        Url::parse(&url_string).map_err(|err| RegularError::Unrecoverable {
            message: format!(
                "[R2ImagePool::get_signed] failed to parse URL '{}': {}",
                url_string, err
            ),
        })
    }

    #[instrument(err(Debug), skip(self), level = Level::DEBUG)]
    async fn put_signed(&self, key: &str) -> RegularResult<Url> {
        let content_type = detect_content_type(key).ok_or_else(|| RegularError::Expected {
            variant: ExpectedVariant::ArgsInvalid,
            message: trl("error-unsupported-file-type"),
        })?;

        let presigning_config =
            PresigningConfig::expires_in(PUT_SIGNED_EXPIRATION).map_err(|err| {
                RegularError::Unrecoverable {
                    message: format!(
                        "[R2ImagePool::put_signed] failed to build presigning config: {}",
                        err
                    ),
                }
            })?;

        let presigned_request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning_config)
            .await
            .map_err(|err| RegularError::Unrecoverable {
                message: format!(
                    "[R2ImagePool::put_signed] failed to generate presigned put URL: {}",
                    err
                ),
            })?;

        Url::parse(presigned_request.uri()).map_err(|err| RegularError::Unrecoverable {
            message: format!(
                "[R2ImagePool::put_signed] failed to parse presigned URI: {}",
                err
            ),
        })
    }
}

fn detect_content_type(key: &str) -> Option<&'static str> {
    let extension = key.rsplit('.').next()?.to_lowercase();

    match extension.as_str() {
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

#[cfg(test)]
mod tests {
    // detect_content_type(detect_content_type)(positive): supported image extensions should map to MIME types.
    // detect_content_type_rejects_unknown_extension(detect_content_type)(negative): unsupported extensions should be rejected.
    // get_signed_uses_custom_domain(ImagePool::get_signed)(positive): download URLs should be built from the configured public domain.

    use super::*;

    fn image_pool() -> R2ImagePool {
        let credentials = Credentials::new("access-key", "secret-key", None, None, "test");

        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url("https://example.invalid")
            .credentials_provider(credentials)
            .build();

        R2ImagePool::new(
            Client::from_conf(config),
            "bucket".to_string(),
            "https://images.example.test/root/".to_string(),
        )
    }

    #[test]
    fn detect_content_type_maps_supported_extensions() {
        assert_eq!(detect_content_type("avatar.PNG"), Some("image/png"));

        assert_eq!(detect_content_type("avatar.webp"), Some("image/webp"));
    }

    #[test]
    fn detect_content_type_rejects_unknown_extension() {
        assert_eq!(detect_content_type("avatar.txt"), None);
    }

    #[tokio::test]
    async fn get_signed_uses_custom_domain() {
        let image_pool = image_pool();

        let url = ImagePool::get_signed(&image_pool, "avatars/user-1.png").await;
        assert!(url.is_ok());

        let url = url.ok().unwrap();

        assert_eq!(
            url.as_str(),
            "https://images.example.test/root/avatars/user-1.png"
        );
    }
}
