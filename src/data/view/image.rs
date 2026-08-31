//! View DTOs for the image domain.

//! Shared image-upload request and response data.

use std::collections::BTreeMap;

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Presigned capability for one pending image upload.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImageUploadSlotView {
    //
    /// Presigned PUT URL.
    pub put_url: String,
    /// Monotonic image identity version.
    #[serde(rename = "image_version")]
    pub image_ver: u32,
    /// Headers bound into the PUT signature.
    pub headers: BTreeMap<String, String>,
}
