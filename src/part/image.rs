//! Object-storage port for resolving signed URLs.

use async_trait::async_trait;
use url::Url;

use crate::result::RegularResult;

/// Abstraction over an image pool.
///
/// Provides signed URLs for direct client-to-storage uploads and downloads,
/// avoiding the need to proxy image bytes through the application server.
#[async_trait]
pub trait ImagePool {
    // FIXME: bad names.

    /// Returns a signed download URL for the object at `key`.
    async fn get_signed(&self, key: &str) -> RegularResult<Url>;

    /// Returns a signed upload URL for writing an object at `key`.
    async fn put_signed(&self, key: &str) -> RegularResult<Url>;

    /// Check whether an object exists in storage.
    async fn head_object(&self, key: &str) -> RegularResult<bool>;

    /// Delete an object from storage. Idempotent — succeeds if the
    /// object does not exist.
    async fn delete_object(&self, key: &str) -> RegularResult<()>;
}
