//! Domain models for chapters inside comics — workflow state, progress counters,
//! and display metadata tracked per chapter.
//!
//! See [`StagePhase`] and [`WorkflowStage`](crate::value::chapter::WorkflowStage)
//! for the six production stages and their phase transitions.
//! Convert to [`ChapterInfoVal`] for presentation.
//!
//! [`ChapterInfoVal`]: crate::data::chapter::ChapterInfoVal
//! [`StagePhase`]: crate::value::chapter::StagePhase

use crate::value::chapter::ChapterInclOpt;

/// Filtering, pagination, and include parameters for listing chapters.
pub struct ChapterListSpec {
    //
    /// Foreign key scoping the chapter listing to a single comic.
    pub comic_id: String,
    /// Flags controlling which optional associations (such as comic or creator
    /// data) are joined into results.
    pub incl_opt: Vec<ChapterInclOpt>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}
