//! Mock implementation of [ImagePool] and [ImageManager] for testing
//! signed URL resolution with deterministic output.

use std::collections::BTreeMap;

use url::Url;

use poprako_util::i18n::trl;

use crate::part::image::{ImageManager, ImageObjectInfo, ImagePool, ImageUploadSlot, ImageUploadSpec};
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::value::image::ImageHash;

/// Mock implementation of [ImagePool].
///
/// Returns deterministic test URLs (`https://test.local/get/{key}` /
/// `https://test.local/put/{key}`). Configure
/// [Mock::with_image_get_failure] or [Mock::with_image_put_failure] to test
/// error paths.
impl ImagePool for Mock {
    async fn gen_download_url(&self, key: &str) -> BaseResult<Url> {
        //
        if self.flags.lock().unwrap().image_get_failure {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-image-get-failed"),
            });
        }

        accept(Url::parse(&format!("https://test.local/get/{}", key)).unwrap())
    }

    async fn gen_thumbnail_download_url(
        &self,
        original_key: &str,
    ) -> BaseResult<Url> {
        //
        if self.flags.lock().unwrap().image_get_failure {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-image-get-failed"),
            });
        }

        accept(
            Url::parse(&format!(
                "https://test.local/cdn-cgi/image/width=300,fit=scale-down,quality=80,format=auto,metadata=none/{}",
                original_key
            ))
            .unwrap(),
        )
    }

    async fn get_upload_url(&self, key: &str) -> BaseResult<Url> {
        //
        if self.flags.lock().unwrap().image_put_failure {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-image-put-failed"),
            });
        }

        accept(Url::parse(&format!("https://test.local/put/{}", key)).unwrap())
    }

    async fn get_upload_slot(
        &self,
        spec: ImageUploadSpec<'_>,
    ) -> BaseResult<ImageUploadSlot> {
        //
        if self.flags.lock().unwrap().image_put_failure {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-image-put-failed"),
            });
        }

        let url =
            Url::parse(&format!("https://test.local/put/{}", spec.object_key))
                .unwrap();

        let mut headers = BTreeMap::new();

        headers
            .insert("content-length".into(), spec.content_length.to_string());

        headers.insert("content-type".into(), spec.content_type.into());

        headers.insert(
            "x-amz-checksum-sha256".into(),
            spec.checksum_sha256.to_base64(),
        );

        accept(ImageUploadSlot { url, headers })
    }
}

/// Mock implementation of [ImageManager].
impl ImageManager for Mock {
    async fn head_object(
        &self,
        key: &str,
    ) -> BaseResult<Option<ImageObjectInfo>> {
        //
        if self.flags.lock().unwrap().image_head_failure {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-image-head-failed"),
            });
        }

        if self.flags.lock().unwrap().image_head_absent {
            return accept(None);
        }

        let flags = self.flags.lock().unwrap().clone();

        let state = self.state.lock().unwrap();

        let expected_checksum_sha256 = state
            .pages
            .iter()
            .find(|page_info| page_info.image_key.as_deref() == Some(key))
            .map(|page_info| page_info.image_hash.clone())
            .or_else(|| {
                state
                    .users
                    .iter()
                    .find(|user_info| {
                        user_info.avatar_key.as_deref() == Some(key)
                    })
                    .map(|user_info| user_info.avatar_hash.clone())
            })
            .or_else(|| {
                state
                    .teams
                    .iter()
                    .find(|team_info| {
                        team_info.avatar_key.as_deref() == Some(key)
                    })
                    .map(|team_info| team_info.avatar_hash.clone())
            })
            .or_else(|| {
                state
                    .comics
                    .iter()
                    .find(|comic_info| {
                        comic_info.cover_key.as_deref() == Some(key)
                    })
                    .map(|comic_info| comic_info.cover_hash.clone())
            })
            .unwrap_or_else(|| ImageHash::new([0; 32]));

        let byte_length = match flags.image_head_length_mismatch {
            //
            true => 1,

            false => 4096,
        };

        let checksum_sha256 = match flags.image_head_hash_mismatch {
            //
            true => ImageHash::new([255; 32]),

            false => expected_checksum_sha256,
        };

        accept(Some(ImageObjectInfo {
            byte_length,
            checksum_sha256,
        }))
    }

    async fn delete_object(&self, key: &str) -> BaseResult<()> {
        //
        if self.flags.lock().unwrap().image_delete_failure {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-image-delete-failed"),
            });
        }

        self.state
            .lock()
            .unwrap()
            .deleted_image_keys
            .push(key.to_string());

        accept(())
    }
}

// gen_download_url_returns_stable_url(ImagePool::gen_download_url)(positive): download URLs should be deterministic for assertions.
// get_upload_url_returns_stable_url(ImagePool::get_upload_url)(positive): upload URLs should be deterministic for assertions.
// gen_download_url_failure_returns_expected_err(ImagePool::gen_download_url)(negative): configured get failures should return an expected error.

/// Mock helper that returns a stable deterministic upload URL.
#[tokio::test]
async fn get_upload_url_returns_stable_url() {
    //
    let mock = Mock::new();

    let url = ImagePool::get_upload_url(&mock, "avatar.png").await;

    assert!(url.is_ok());

    let url = url.ok().unwrap();

    assert_eq!(url.as_str(), "https://test.local/put/avatar.png");
}

/// Mock helper that returns an expected error when download failure is configured.
#[tokio::test]
async fn gen_download_url_failure_returns_expected_err() {
    //
    let mock = Mock::new().with_image_get_failure();

    let err_download = ImagePool::gen_download_url(&mock, "avatar.png")
        .await
        .err()
        .unwrap();

    assert!(matches!(
        err_download,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));
}
