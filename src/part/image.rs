//! Object-storage port for signed URL generation and object lifecycle
//! management.

use std::collections::BTreeMap;
use std::future::Future;

use url::Url;

use crate::result::BaseResult;
use crate::value::image::ImageHash;

/// Constraints bound into a presigned image upload request.
pub struct ImageUploadSpec<'a> {
    /// Storage key identifying the target object path.
    pub object_key: &'a str,
    /// MIME type the client must declare when uploading.
    pub content_type: &'static str,
    /// Expected SHA-256 hash the presigned URL is bound to.
    pub checksum_sha256: &'a ImageHash,
    /// Exact byte length of the upload, enforced at the storage layer.
    pub content_length: u64,
}

/// A presigned upload URL and the headers the client must send unchanged.
pub struct ImageUploadSlot {
    /// Presigned URL the client uses for the direct upload.
    pub url: Url,
    /// HTTP headers the client must include verbatim with the upload request.
    pub headers: BTreeMap<String, String>,
}

/// Verified object identity returned from storage metadata.
pub struct ImageObjectInfo {
    /// Size of the stored object in bytes.
    pub byte_length: u64,
    /// SHA-256 hash of the stored object content.
    pub checksum_sha256: ImageHash,
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
    ) -> impl Future<Output = BaseResult<Url>> + Send;

    /// Returns a thumbnail download URL for the object at `original_key`.
    fn gen_thumbnail_download_url(
        &self,
        original_key: &str,
    ) -> impl Future<Output = BaseResult<Url>> + Send;

    /// Returns a signed upload URL for writing an object at `key`.
    fn get_upload_url(
        &self,
        key: &str,
    ) -> impl Future<Output = BaseResult<Url>> + Send;

    /// Returns an upload target whose signature binds content identity.
    fn get_upload_slot(
        &self,
        spec: ImageUploadSpec<'_>,
    ) -> impl Future<Output = BaseResult<ImageUploadSlot>> + Send;
}

/// Abstraction over image object lifecycle — metadata and deletion.
///
/// Handles existence checks and object removal from the storage backend.
/// Used by the prom (background task) layer for deferred image cleanup
/// and upload verification.
///
/// Methods return `impl Future + Send` so the futures can be spawned on
/// the prom worker's async runtime.
pub trait ImageManager {
    /// Returns verified object metadata, or `None` when the object is absent.
    fn head_object(
        &self,
        key: &str,
    ) -> impl Future<Output = BaseResult<Option<ImageObjectInfo>>> + Send;

    /// Delete an object from storage. Idempotent — succeeds if the
    /// object does not exist.
    fn delete_object(
        &self,
        key: &str,
    ) -> impl Future<Output = BaseResult<()>> + Send;
}
