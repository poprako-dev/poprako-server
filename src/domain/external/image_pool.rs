use async_trait::async_trait;
use url::Url;

use crate::domain::result::DomainResult;
use crate::util::ForwardRef;

/// Forwarding marker for [`ImageGet`].
pub struct ImageGetForward;

/// Forwarding marker for [`ImagePut`].
pub struct ImagePutForward;

/// Forwarding marker for [`ImageDelete`].
pub struct ImageDeleteForward;

/// Generates a pre-signed download URL for an object in the image pool.
#[async_trait]
pub trait ImageGet {
    /// Returns a signed GET URL valid for a limited time window.
    async fn get_signed(&self, key: &str) -> DomainResult<Url>;
}

#[async_trait]
impl<T> ImageGet for T
where
    T: ForwardRef<ImageGetForward> + Sync,
    T::Target: ImageGet + Sync,
{
    async fn get_signed(&self, key: &str) -> DomainResult<Url> {
        self.forward_ref().get_signed(key).await
    }
}

/// Generates a pre-signed upload URL for an object in the image pool.
#[async_trait]
pub trait ImagePut {
    /// Returns a signed PUT URL that clients can use to upload content.
    async fn put_signed(&self, key: &str) -> DomainResult<Url>;
}

#[async_trait]
impl<T> ImagePut for T
where
    T: ForwardRef<ImagePutForward> + Sync,
    T::Target: ImagePut + Sync,
{
    async fn put_signed(&self, key: &str) -> DomainResult<Url> {
        self.forward_ref().put_signed(key).await
    }
}

/// Deletes one or more objects from the image pool in a single batch operation.
#[async_trait]
pub trait ImageDelete {
    /// Deletes every object whose key appears in `keys`. No-op if none exist.
    async fn delete_batch(&self, keys: &[&str]) -> DomainResult<()>;
}

#[async_trait]
impl<T> ImageDelete for T
where
    T: ForwardRef<ImageDeleteForward> + Sync,
    T::Target: ImageDelete + Sync,
{
    async fn delete_batch(&self, keys: &[&str]) -> DomainResult<()> {
        self.forward_ref().delete_batch(keys).await
    }
}

/// Composite of all image-pool capabilities.
pub trait ImagePool: ImageGet + ImagePut + ImageDelete {}

impl<T> ImagePool for T where T: ImageGet + ImagePut + ImageDelete {}

#[cfg(test)]
mod tests {
    // get_signed_forwards_to_target(ForwardRef<ImageGetForward>)(positive): wrapper should delegate signed get URL creation to image pool target
    // put_signed_forwards_to_target(ForwardRef<ImagePutForward>)(positive): wrapper should delegate signed put URL creation to image pool target
    // delete_batch_forwards_to_target(ForwardRef<ImageDeleteForward>)(positive): wrapper should delegate batch deletion to image pool target

    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use url::Url;

    use crate::domain::external::image_pool::{
        ImageDelete, ImageDeleteForward, ImageGet, ImageGetForward, ImagePut, ImagePutForward,
    };
    use crate::domain::result::DomainResult;
    use crate::impl_forward_ref;

    #[derive(Clone, Default)]
    struct FakeImagePool {
        calls: Arc<Mutex<Vec<String>>>,
    }

    struct FakeHarness {
        image_pool: FakeImagePool,
    }

    fn harn() -> FakeHarness {
        FakeHarness {
            image_pool: FakeImagePool::default(),
        }
    }

    impl_forward_ref!(
        FakeHarness => FakeImagePool,
        image_pool,
        ImageGetForward,
        ImagePutForward,
        ImageDeleteForward,
    );

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

    #[tokio::test]
    async fn get_signed_forwards_to_target() {
        let harn = harn();

        let url = harn.get_signed("page-1.png").await.unwrap();

        assert_eq!(url.as_str(), "https://example.test/get");
        assert_eq!(
            harn.image_pool.calls.lock().unwrap().as_slice(),
            ["get:page-1.png"]
        );
    }

    #[tokio::test]
    async fn put_signed_forwards_to_target() {
        let harn = harn();

        let url = harn.put_signed("page-2.png").await.unwrap();

        assert_eq!(url.as_str(), "https://example.test/put");
        assert_eq!(
            harn.image_pool.calls.lock().unwrap().as_slice(),
            ["put:page-2.png"]
        );
    }

    #[tokio::test]
    async fn delete_batch_forwards_to_target() {
        let harn = harn();

        harn.delete_batch(&["page-1.png", "page-2.png"])
            .await
            .unwrap();

        assert_eq!(
            harn.image_pool.calls.lock().unwrap().as_slice(),
            ["page-1.png,page-2.png"]
        );
    }
}
