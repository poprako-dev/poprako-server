//! Object-storage port for resolving signed URLs.

use async_trait::async_trait;
use url::Url;

use crate::result::RootResult;

/// Abstraction over an image pool.
///
/// Provides signed URLs for direct client-to-storage uploads and downloads,
/// avoiding the need to proxy image bytes through the application server.
#[async_trait]
pub trait ImagePool {
    /// Returns a signed download URL for the object at `key`.
    async fn get_signed(&self, key: &str) -> RootResult<Url>;

    /// Returns a signed upload URL for writing an object at `key`.
    async fn put_signed(&self, key: &str) -> RootResult<Url>;
}
