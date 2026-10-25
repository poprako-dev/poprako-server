//! Composite image physical identities.

use serde::{Deserialize, Serialize};

use crate::value::business_id::{ChapterId, PageArtworkId};
use crate::value::image::ImageExt;

/// Identity required to locate a composite image generation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageArtworkImageKey {
    /// Owning Chapter.
    pub chapter_id: ChapterId,
    /// Stable composite page identity.
    pub page_artwork_id: PageArtworkId,
    /// Image format.
    pub ext: ImageExt,
}
