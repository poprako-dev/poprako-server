use async_trait::async_trait;
use url::Url;

use crate::domain::result::DomainResult;

/// Generates a pre-signed download URL for an object in the image pool.
#[async_trait]
pub trait ImageGet {
    /// Returns a signed GET URL valid for a limited time window.
    async fn get_signed(&self, key: &str) -> DomainResult<Url>;
}

/// Generates a pre-signed upload URL for an object in the image pool.
#[async_trait]
pub trait ImagePut {
    /// Returns a signed PUT URL that clients can use to upload content.
    async fn put_signed(&self, key: &str) -> DomainResult<Url>;
}

/// Deletes one or more objects from the image pool in a single batch operation.
#[async_trait]
pub trait ImageDelete {
    /// Deletes every object whose key appears in `keys`. No-op if none exist.
    async fn delete_batch(&self, keys: &[&str]) -> DomainResult<()>;
}

/// Composite of all image-pool capabilities.
/// Implemented by [`super::ImageGet`] + [`super::ImagePut`] + [`super::ImageDelete`] blanket.
pub trait ImagePool: ImageGet + ImagePut + ImageDelete {}
