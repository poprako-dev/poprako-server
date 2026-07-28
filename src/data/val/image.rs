//! Val DTOs for the image domain.

//! Shared image-upload request and response data.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::image::ImageUploadSlotView;

/// Single-resource image reservation response.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveImageVal {
    /// Upload capability, absent when this content is already uploaded.
    pub slot: Option<ImageUploadSlotView>,
}
