//! View DTOs for the image domain.
//! Shared image-upload request and response data.
use crate::data::view::upload::UploadHeaderView;

/// Presigned capability for one pending image upload.
pub struct ImageUploadSlotView {
    /// Presigned PUT URL.
    pub put_url: String,
    /// Monotonic image identity version.
    pub image_ver: u32,
    /// Headers bound into the PUT signature.
    pub headers: Vec<UploadHeaderView>,
}
