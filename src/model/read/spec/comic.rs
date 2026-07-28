//! Domain models for comics inside worksets — core metadata, cover-storage
//! tracking, and denormalised chapter counters.
//!
//! Convert to [`ComicInfoVal`] for presentation outside the domain layer.
//!
//! [`ComicInfoVal`]: crate::data::comic::ComicInfoVal

use crate::value::chapter::StageMask;
use crate::value::comic::ComicInclOpt;

/// Filtering and pagination parameters for listing comics within a workset.
pub struct ComicListSpec {
    //
    /// The workset whose comics should be listed.
    pub workset_id: String,

    /// Optional fuzzy title search to narrow the results.
    pub fuzzy_title: Option<String>,
    /// Workflow-stage filter controlling which comics are returned.
    pub kind: ComicListKind,

    /// Additional data to include in each result, such as the workset or creator.
    pub incl_opt: Vec<ComicInclOpt>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}

/// Workflow-stage filtering mode for listing comics.
pub enum ComicListKind {
    //
    /// Include all comics regardless of workflow stage.
    All,

    /// Include only comics whose chapters have any of the specified stages.
    Stages(StageMask),
}
