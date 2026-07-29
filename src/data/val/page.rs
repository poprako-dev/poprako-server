//! Val DTOs for the page domain.

//! Data transfer objects for page use cases.

use serde::Serialize;

use crate::data::view::image::ImageUploadSlotView;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::image::{ImageExt, ImageHash};

#[cfg(test)]
mod tests;

/// Return value from successful chapter page reservations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveChapterPagesVal {
    /// Reserved pages with upload targets.
    pub pages: Vec<ReservedPageVal>,
}

/// One reserved page upload target.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReservedPageVal {
    //
    /// Reserved page identifier.
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
