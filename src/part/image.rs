//! Object-storage port for signed URL generation and object lifecycle
//! management.

use std::collections::BTreeMap;
use std::future::Future;

use url::Url;

use crate::result::BaseRest;
/// Constraints bound into a presigned image upload request.
pub struct ImageUploadSpec<'a> {
    //
    /// Storage key identifying the target object path.
    pub object_key: &'a str,
    /// MIME type the client must declare when uploading.
    pub content_type: &'static str,
    /// Exact byte length of the upload, enforced at the storage layer.
    pub content_length: u64,
}

/// A presigned upload URL and the headers the client must send unchanged.
pub struct ImageUploadSlot {
    //
    /// Presigned URL the client uses for the direct upload.
    pub url: Url,
    /// HTTP headers the client must include verbatim with the upload request.
    pub headers: BTreeMap<String, String>,
}

/// Abstraction over an image pool — signed URL generation.
///
/// Provides signed URLs for direct client-to-storage uploads and downloads,
/// avoiding the need to proxy image bytes through the application server.
pub trait ImagePool {
    /// Returns a signed download URL for the object at `key`.
    fn gen_download_url(
        &self,
        key: &str,
    ) -> impl Future<Output = BaseRest<Url>> + Send;

    /// Returns a thumbnail download URL for the object at `original_key`.
    fn gen_thumbnail_download_url(
        &self,
        original_key: &str,
    ) -> impl Future<Output = BaseRest<Url>> + Send;

    /// Returns a signed upload URL for writing an object at `key`.
    fn get_upload_url(
        &self,
        key: &str,
    ) -> impl Future<Output = BaseRest<Url>> + Send;

    /// Returns an upload target whose signature binds content identity.
    fn get_upload_slot(
        &self,
        spec: ImageUploadSpec<'_>,
    ) -> impl Future<Output = BaseRest<ImageUploadSlot>> + Send;
}

/// Abstraction over image object lifecycle — existence checks and deletion.
///
/// Handles existence checks and object removal from the storage backend.
/// Used by the prom (background task) layer for deferred image cleanup
/// and upload verification.
///
/// Methods return `impl Future + Send` so the futures can be spawned on
/// the prom worker's async runtime.
pub trait ImageManager {
    /// Returns whether an object exists at `key`.
    fn object_exists(
        &self,
        key: &str,
    ) -> impl Future<Output = BaseRest<bool>> + Send;

    /// Delete an object from storage. Idempotent — succeeds if the
    /// object does not exist.
    fn delete_object(
        &self,
        key: &str,
    ) -> impl Future<Output = BaseRest<()>> + Send;
}
