//! Mock implementation of [ImagePool] for testing signed URL resolution with deterministic output.

use async_trait::async_trait;
use url::Url;

use poprako_util::i18n::trl;

use crate::part::image::ImagePool;
use crate::part_impl::repo_mock::Mock;
use crate::result::{ExpectedVariant, RegularError, RegularResult};

/// Mock implementation of [ImagePool].
///
/// Returns deterministic test URLs (`https://test.local/get/{key}` /
/// `https://test.local/put/{key}`). Configure
/// [Mock::with_image_get_failure] or [Mock::with_image_put_failure] to test
/// error paths.
#[async_trait]
impl ImagePool for Mock {
    async fn get_signed(&self, key: &str) -> RegularResult<Url> {
        if self.flags.lock().unwrap().image_get_failure {
            return Err(RegularError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-image-get-failed"),
            });
        }

        Ok(Url::parse(&format!("https://test.local/get/{}", key)).unwrap())
    }

    async fn put_signed(&self, key: &str) -> RegularResult<Url> {
        if self.flags.lock().unwrap().image_put_failure {
            return Err(RegularError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-image-put-failed"),
            });
        }

        Ok(Url::parse(&format!("https://test.local/put/{}", key)).unwrap())
    }
}

// put_signed_returns_stable_url(ImagePool::put_signed)(positive): put URLs should be deterministic for assertions.
// get_signed_failure_returns_expected_error(ImagePool::get_signed)(negative): configured get failures should return an expected error.

#[tokio::test]
async fn put_signed_returns_stable_url() {
    let mock = Mock::new();

    let url = ImagePool::put_signed(&mock, "avatar.png").await;
    assert!(url.is_ok());
    let url = url.ok().unwrap();

    assert_eq!(url.as_str(), "https://test.local/put/avatar.png");
}

#[tokio::test]
async fn get_signed_failure_returns_expected_error() {
    let mock = Mock::new().with_image_get_failure();

    let err = ImagePool::get_signed(&mock, "avatar.png")
        .await
        .err()
        .unwrap();

    assert!(matches!(
        err,
        RegularError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));
}
