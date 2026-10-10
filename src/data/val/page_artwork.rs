//! Composite page allocation results.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::image::ImageUploadSlotView;
use crate::value::image::{ImageExt, ImageHash};

/// One resolved composite page and its current image generation.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocatedPageArtworkVal {
    /// Stable composite page identity.
    pub page_artwork_id: String,

    /// Final page position.
    pub index: usize,

    /// Composite image hash.
    pub image_hash: ImageHash,
    /// Image format.
    pub ext: ImageExt,
    /// Allocated or reused image generation.
    #[serde(rename = "image_version")]
    pub image_ver: u32,

    /// Upload capability; absent when the exact content is already available.
    pub slot: Option<ImageUploadSlotView>,
}

/// Allocations aligned with the complete submitted manifest.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocChapterPageArtworksVal {
    /// Resolved pages in request order.
    pub pages: Vec<AllocatedPageArtworkVal>,
}
