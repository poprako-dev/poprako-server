//! Val DTOs for the page domain.

//! Data transfer objects for page use cases.

#[cfg(test)]
mod tests;

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::image::ImageUploadSlotView;
use crate::value::image::{ImageExt, ImageHash};

/// Return value from successful chapter page allocations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocChapterPagesVal {
    /// Allocated pages with upload targets.
    pub pages: Vec<AllocatedPageVal>,
}

/// One allocated page upload target.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocatedPageVal {
    //
    /// Allocated page identifier.
    pub page_id: String,

    /// Ordinal position within the chapter.
    pub index: u32,
    /// Content hash of the page image.
    pub image_hash: ImageHash,
    /// File format.
    pub ext: ImageExt,

    /// Presigned upload slot, if a new image must be uploaded.
    pub slot: Option<ImageUploadSlotView>,
}
