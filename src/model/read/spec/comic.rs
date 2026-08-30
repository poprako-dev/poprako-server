//! Domain models for comics inside worksets — core metadata, cover-storage
//! tracking, and denormalised chapter counters.
//!
//! Convert to [`ComicInfoView`] for presentation outside the domain layer.
//!
//! [`ComicInfoView`]: crate::data::view::comic::ComicInfoView

use crate::value::chapter::mask::StageMask;
use crate::value::comic::{ComicInclOpt, ComicStatus};

/// Filtering and pagination parameters for listing comics within a workset.
pub struct ComicListSpec {
    //
    /// The workset whose comics should be listed.
    pub workset_id: String,

    /// Optional fuzzy title search to narrow the results.
    pub fuzzy_title: Option<String>,
    /// Optional workflow-stage mask filter.
    pub stages: Option<StageMask>,
    /// Optional lifecycle state filter.
    pub status: Option<ComicStatus>,

    /// Additional data to include in each result, such as the workset or creator.
    pub incl_opt: Vec<ComicInclOpt>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}
