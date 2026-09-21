//! Val DTOs for the page domain.

//! Data transfer objects for page use cases.

#[cfg(test)]
mod tests;

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::image::ImageUploadSlotView;
use crate::model::read::proj::page::{PageUnitDiffStats, PageUnitFlaggedStats};
use crate::value::image::{ImageExt, ImageHash};

/// Visible Unit text statistics for a Page with revision differences.
/// Counts are independent of revision approval and cover all visible Units.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(test, derive(Debug))]
pub struct PageUnitDiffStatsVal {
    /// Permanent Page identifier.
    pub page_id: String,
    /// Original zero-based Chapter position, preserved after filtering.
    pub index: usize,

    /// Units with non-whitespace translation text; revision-only Units are excluded.
    pub translated_unit_count: usize,
    /// Units with non-whitespace translation and revision texts that differ exactly.
    pub editted_unit_count: usize,
    /// Units with non-whitespace revision text and absent or whitespace-only translation.
    pub proofreader_append_unit_count: usize,
}

impl From<PageUnitDiffStats> for PageUnitDiffStatsVal {
    // Converts the read projection into the serialized response value.
    fn from(model: PageUnitDiffStats) -> Self {
        //
        Self {
            page_id: model.page_id,
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
pub struct AllocChapterPagesVal {
    /// Allocated pages with upload targets.
    pub pages: Vec<AllocatedPageVal>,
}

/// One allocated page upload target.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocatedPageVal {
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

/// Visible flagged Unit statistics for one matching Page.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(test, derive(Debug))]
pub struct PageUnitFlaggedStatsVal {
    /// Permanent Page identifier.
    pub page_id: String,
    /// Original zero-based Chapter position, preserved after filtering.
    pub index: usize,

    /// Number of visible Units flagged for later review.
    pub flagged_unit_count: usize,
}

impl From<PageUnitFlaggedStats> for PageUnitFlaggedStatsVal {
    // Converts the persisted aggregate into its response value.
    fn from(model: PageUnitFlaggedStats) -> Self {
        //
        Self {
            page_id: model.page_id,
            index: model.index,
            flagged_unit_count: model.flagged_unit_count,
        }
    }
}
