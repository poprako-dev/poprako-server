//! Val DTOs for the page domain.

//! Data transfer objects for page use cases.

#[cfg(test)]
mod tests;

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::image::ImageUploadSlotView;
use crate::model::read::proj::page_source::{
    PageSourceUnitDiffStats, PageSourceUnitFlaggedStats,
};
use crate::value::image::{ImageExt, ImageHash};

/// Visible Unit text statistics for a source page with revision differences.
/// Counts are independent of revision approval and cover all visible Units.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(test, derive(Debug))]
pub struct PageSourceUnitDiffStatsVal {
    /// Permanent source page identifier.
    pub page_source_id: String,
    /// Original zero-based Chapter position, preserved after filtering.
    pub index: usize,

    /// Units with translation text or typesetting annotations; revision-only Units are excluded.
    pub translated_unit_count: usize,
    /// Units with non-whitespace translation and revision texts that differ exactly.
    pub editted_unit_count: usize,
    /// Units with non-whitespace revision text and absent or whitespace-only translation.
    pub proofreader_append_unit_count: usize,
}

impl From<PageSourceUnitDiffStats> for PageSourceUnitDiffStatsVal {
    // Converts the read projection into the serialized response value.
    fn from(model: PageSourceUnitDiffStats) -> Self {
        Self {
            page_source_id: model.page_source_id,
            index: model.index,
            translated_unit_count: model.translated_unit_count,
            editted_unit_count: model.editted_unit_count,
            proofreader_append_unit_count: model.proofreader_append_unit_count,
        }
    }
}

/// Return value from successful chapter page allocations.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocChapterPageSourcesVal {
    /// Allocated pages with upload targets.
    pub pages: Vec<AllocatedPageSourceVal>,
}

/// One allocated page upload target.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocatedPageSourceVal {
    /// Allocated page identifier.
    pub page_source_id: String,

    /// Ordinal position within the chapter.
    pub index: u32,

    /// Content hash of the page image.
    pub image_hash: ImageHash,
    /// File format.
    pub ext: ImageExt,

    /// Current allocated or reused image generation.
    #[serde(rename = "image_version")]
    pub image_ver: u32,

    /// Presigned upload slot, if a new image must be uploaded.
    pub slot: Option<ImageUploadSlotView>,
}

/// Visible flagged Unit statistics for one matching source page.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(test, derive(Debug))]
pub struct PageSourceUnitFlaggedStatsVal {
    /// Permanent source page identifier.
    pub page_source_id: String,
    /// Original zero-based Chapter position, preserved after filtering.
    pub index: usize,

    /// Number of visible Units flagged for later review.
    pub flagged_unit_count: usize,
}

impl From<PageSourceUnitFlaggedStats> for PageSourceUnitFlaggedStatsVal {
    // Converts the persisted aggregate into its response value.
    fn from(model: PageSourceUnitFlaggedStats) -> Self {
        Self {
            page_source_id: model.page_source_id,
            index: model.index,
            flagged_unit_count: model.flagged_unit_count,
        }
    }
}
