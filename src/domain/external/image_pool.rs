use async_trait::async_trait;
use url::Url;

use poprako_macro::{forward_ref, forward_ref_super};

use crate::domain::result::DomainResult;

/// Generates a pre-signed download URL for an object in the image pool.
#[forward_ref]
#[async_trait]
pub trait ImageGet {
    /// Returns a signed GET URL valid for a limited time window.
    async fn get_signed(&self, key: &str) -> DomainResult<Url>;
}

/// Generates a pre-signed upload URL for an object in the image pool.
#[forward_ref]
#[async_trait]
pub trait ImagePut {
    /// Returns a signed PUT URL that clients can use to upload content.
    async fn put_signed(&self, key: &str) -> DomainResult<Url>;
}

/// Deletes one or more objects from the image pool in a single batch operation.
#[forward_ref]
#[async_trait]
pub trait ImageDelete {
    /// Deletes every object whose key appears in `keys`. No-op if none exist.
    async fn delete_batch(&self, keys: &[&str]) -> DomainResult<()>;
}

/// Inspects object metadata in the image pool.
#[forward_ref]
#[async_trait]
pub trait ImageInspect {
    /// Returns whether an object exists at the given key.
    async fn exists(&self, key: &str) -> DomainResult<bool>;
}

/// Composite of all image-pool capabilities.
#[forward_ref_super]
pub trait ImagePool: ImageGet + ImagePut + ImageDelete + ImageInspect {}

impl<T> ImagePool for T where T: ImageGet + ImagePut + ImageDelete + ImageInspect {}

#[cfg(test)]
mod tests {
    // get_signed_forwards_to_target(ForwardRef<ImageGetForward>)(positive): wrapper should delegate signed get URL creation to image pool target
    // put_signed_forwards_to_target(ForwardRef<ImagePutForward>)(positive): wrapper should delegate signed put URL creation to image pool target
    // delete_batch_forwards_to_target(ForwardRef<ImageDeleteForward>)(positive): wrapper should delegate batch deletion to image pool target.
    // exists_forwards_to_target(ForwardRef<ImageInspectForward>)(positive): wrapper should delegate object inspection to image pool target.

    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use url::Url;

    use poprako_macro::ForwardRefs;

    use crate::domain::external::image_pool::{
        ImageDelete, ImageDeleteForward, ImageGet, ImageGetForward, ImageInspect,
        ImageInspectForward, ImagePut, ImagePutForward,
    };
    use crate::domain::result::DomainResult;

    #[derive(Clone, Default)]
    struct FakeImagePool {
        calls: Arc<Mutex<Vec<String>>>,
    }

    #[derive(ForwardRefs)]
    struct FakeHarness {
        #[forward_ref(ImageGet, ImagePut, ImageDelete, ImageInspect)]
        image_pool: FakeImagePool,
    }

    fn harn() -> FakeHarness {
        FakeHarness {
            image_pool: FakeImagePool::default(),
        }
    }

    #[async_trait]
    impl ImageGet for FakeImagePool {
        async fn get_signed(&self, key: &str) -> DomainResult<Url> {
            self.calls.lock().unwrap().push(format!("get:{}", key));
            Ok(Url::parse("https://example.test/get").unwrap())
        }
    }

    #[async_trait]
    impl ImagePut for FakeImagePool {
        async fn put_signed(&self, key: &str) -> DomainResult<Url> {
            self.calls.lock().unwrap().push(format!("put:{}", key));
            Ok(Url::parse("https://example.test/put").unwrap())
        }
    }

    #[async_trait]
    impl ImageDelete for FakeImagePool {
        async fn delete_batch(&self, keys: &[&str]) -> DomainResult<()> {
            self.calls.lock().unwrap().push(keys.join(","));
            Ok(())
        }
    }

    #[async_trait]
    impl ImageInspect for FakeImagePool {
        async fn exists(&self, key: &str) -> DomainResult<bool> {
            self.calls.lock().unwrap().push(format!("exists:{}", key));
            Ok(true)
        }
    }

    #[tokio::test]
    async fn get_signed_forwards_to_target() {
        let harn = harn();

        let url = ImageGet::get_signed(&harn, "page-1.png").await.unwrap();

        assert_eq!(url.as_str(), "https://example.test/get");
        assert_eq!(
            harn.image_pool.calls.lock().unwrap().as_slice(),
            ["get:page-1.png"]
        );
    }

    #[tokio::test]
    async fn put_signed_forwards_to_target() {
        let harn = harn();

        let url = ImagePut::put_signed(&harn, "page-2.png").await.unwrap();

        assert_eq!(url.as_str(), "https://example.test/put");
        assert_eq!(
            harn.image_pool.calls.lock().unwrap().as_slice(),
            ["put:page-2.png"]
        );
    }

    #[tokio::test]
    async fn delete_batch_forwards_to_target() {
        let harn = harn();

        ImageDelete::delete_batch(&harn, &["page-1.png", "page-2.png"])
            .await
            .unwrap();

        assert_eq!(
            harn.image_pool.calls.lock().unwrap().as_slice(),
            ["page-1.png,page-2.png"]
        );
    }

    #[tokio::test]
    async fn exists_forwards_to_target() {
        let harn = harn();

        let exists = ImageInspect::exists(&harn, "page-1.png").await.unwrap();

        assert!(exists);
        assert_eq!(
            harn.image_pool.calls.lock().unwrap().as_slice(),
            ["exists:page-1.png"]
        );
    }
}
