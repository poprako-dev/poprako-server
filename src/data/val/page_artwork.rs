//! Composite page allocation results.
use crate::data::view::image::ImageUploadSlotView;
use crate::value::image::{ImageExt, ImageHash};

/// One resolved composite page and its current image generation.
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
    pub image_ver: u32,

    /// Upload capability; absent when the exact content is already available.
    pub slot: Option<ImageUploadSlotView>,
}

/// Allocations aligned with the complete submitted manifest.
pub struct AllocChapterPageArtworksVal {
    /// Resolved pages in request order.
    pub pages: Vec<AllocatedPageArtworkVal>,
}
