//! Composite image physical identities.

use crate::value::image::ImageExt;

/// Identity required to locate a composite image generation.
#[derive(Clone, PartialEq, Eq)]
pub struct PageArtworkImageKey {
    /// Owning Chapter.
    pub chapter_id: String,
    /// Stable composite page identity.
    pub page_artwork_id: String,
    /// Image format.
    pub ext: ImageExt,
}
