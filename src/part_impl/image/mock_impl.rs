//! Mock implementation of [ImagePool] and [ImageManager] for testing
//! signed URL resolution with deterministic output.

use std::collections::BTreeMap;

use url::Url;

use crate::part::image::{
    ImageManager, ImagePool, ImageUploadSlot, ImageUploadSpec,
};
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseError, BaseRest, accept};

/// Mock implementation of [`ImagePool`].
///
/// Returns deterministic test URLs (`https://test.local/get/{key}` /
/// `https://test.local/put/{key}`). Configure
/// [`Mock::with_image_get_failure`] or [`Mock::with_image_put_failure`] to test
/// error paths.
impl ImagePool for Mock {
    // Internal implementation of `gen_download_url`.
    async fn gen_download_url(&self, key: &str) -> BaseRest<Url> {
        //
        // Internal implementation detail.
        if self.flags.lock().unwrap().image_get_failure {
            return Err(BaseError::Unrecoverable {
                message: "mock image download URL generation failed".into(),
            });
        }

        accept(Url::parse(&format!("https://test.local/get/{}", key)).unwrap())
    }

    // Internal implementation of `gen_thumbnail_download_url`.
    async fn gen_thumbnail_download_url(
        &self,
        original_key: &str,
    ) -> BaseRest<Url> {
        //
        // Internal implementation detail.
        if self.flags.lock().unwrap().image_get_failure {
            return Err(BaseError::Unrecoverable {
                message: "mock image thumbnail URL generation failed".into(),
            });
        }

        accept(
            Url::parse(&format!(
                "https://test.local/cdn-cgi/image/width=300,fit=scale-down,quality=80,format=auto,metadata=none/{}",
                original_key,
            ))
            .unwrap(),
        )
    }

    // Internal implementation of `get_upload_slot`.
    async fn get_upload_slot(
        &self,
        spec: ImageUploadSpec<'_>,
    ) -> BaseRest<ImageUploadSlot> {
        //
        // Internal implementation detail.
        if self.flags.lock().unwrap().image_put_failure {
            return Err(BaseError::Unrecoverable {
                message: "mock image upload URL generation failed".into(),
            });
        }

        let url =
            Url::parse(&format!("https://test.local/put/{}", spec.object_key))
                .unwrap();

        let mut headers = BTreeMap::new();

        headers
            .insert("content-length".into(), spec.content_length.to_string());

        headers.insert("content-type".into(), spec.content_type.into());

        accept(ImageUploadSlot { url, headers })
    }
}

/// Mock implementation of [`ImageManager`].
impl ImageManager for Mock {
    // Internal implementation of `object_exists`.
    async fn object_exists(&self, _: &str) -> BaseRest<bool> {
        //
        // Internal implementation detail.
        if self.flags.lock().unwrap().image_head_failure {
            return Err(BaseError::Unrecoverable {
                message: "mock image object lookup failed".into(),
            });
        }

        if self.flags.lock().unwrap().image_head_absent {
            return accept(false);
        }

        accept(true)
    }

    // Internal implementation of `delete_object`.
    async fn delete_object(&self, key: &str) -> BaseRest<()> {
        //
        // Internal implementation detail.
        if self.flags.lock().unwrap().image_delete_failure {
            return Err(BaseError::Unrecoverable {
                message: "mock image object deletion failed".into(),
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
// gen_download_url_failure_returns_unrecoverable_err(ImagePool::gen_download_url)(negative): configured get failures should return an unrecoverable error.

/// Mock helper that returns an unrecoverable error when download failure is configured.
#[tokio::test]
async fn gen_download_url_failure_returns_unrecoverable_err() {
    //
    // Internal implementation detail.
    let mock = Mock::new().with_image_get_failure();

    let err_download = ImagePool::gen_download_url(&mock, "avatar.png")
        .await
        .err()
        .unwrap();

    assert!(matches!(err_download, BaseError::Unrecoverable { .. }));
}
