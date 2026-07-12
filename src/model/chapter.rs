//! Domain models for chapters inside comics — workflow state, progress counters,
//! and display metadata tracked per chapter.
//!
//! See [`StagePhase`] and [`WorkflowStage`](crate::value::chapter::WorkflowStage)
//! for the six production stages and their phase transitions.
//! Convert to [`ChapterInfoVal`] for presentation.
//!
//! [`ChapterInfoVal`]: crate::data::chapter::ChapterInfoVal
//! [`StagePhase`]: crate::value::chapter::StagePhase

use time::OffsetDateTime;

use poprako_macro::Paginate;

use crate::model::{comic_model, user_model};
use crate::value::chapter::{ChapterInclOpt, StageMask};

/// A chapterrecord as stored in the database.
///
/// Each chapter belongs to exactly one comic and carries a full snapshot
/// of its workflow progress. The four `unit_count` fields are denormalised
/// counters refreshed by the pipeline as units are processed; they drive
/// percentage-complete rendering without per-unit joins.
///
/// The `is_pinned` flag marks the latest-active chapter for quick access
/// from the comic detail screen. Only one chapter per comic should be
/// pinned at any time.
///
/// Workflow stages are ordered: raw_provide → translate → proofread →
/// typeset_redraw → review → publish. Each phase transitions through
/// [`StagePhase`] values independently.
#[derive(Clone)]
pub struct Info {
    pub id: String,
    pub comic_id: String,

    pub comic: Option<comic_model::Info>,

    pub is_pinned: bool,
    pub index: i32,
    pub subtitle: String,

    /// Denormalised total count of units in this chapter.
    pub page_count: i32,
    /// Denormalised number of units submitted for translation.
    pub total_unit_count: i32,
    /// Denormalised number of units with a completed translation.
    pub translated_unit_count: i32,
    /// Denormalised number of units with a completed proofread.
    pub proofread_unit_count: i32,

    pub stages: StageMask,

    pub creator_id: String,

    pub creator: Option<user_model::Info>,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to insert a new chapter row.
///
/// Supplied at chapter-creation time by the use case layer. The `id` is
/// typically generated via [`ChapterComplex::gen_id`] before constructing
/// this form. The `is_pinned` initial value is determined by the caller
/// (usually `true` for the first chapter in a comic, or the new
/// highest-index chapter).
///
/// [`ChapterComplex::gen_id`]: crate::complex::chapter::ChapterComplex::gen_id
#[cfg_attr(test, derive(Clone))]
pub struct Form {
    pub id: String,
    pub comic_id: String,

    pub is_pinned: bool,
    pub index: i32,
    pub subtitle: String,

    pub creator_id: String,
}

/// Mutable profile (non-workflow) fields for a chapter.
///
/// Only `subtitle` and `is_pinned` are user-editable through the profile
/// update endpoint. Workflow phase transitions are handled via
/// [`ChapterStageUpdate`] instead.
#[cfg_attr(test, derive(Clone))]
pub struct InfoUpdate {
    pub id: String,

    pub subtitle: Option<String>,
    pub pin: Option<bool>,
}

/// Mutable workflow stage mask for a chapter.
///
/// The use case layer validates transition legality before building this update.
#[cfg_attr(test, derive(Clone))]
pub struct StageUpdate {
    pub id: String,

    pub stages: StageMask,
}

/// Filtering, pagination, and include parameters for listing chapters.
#[Paginate]
pub struct ListSpec {
    pub comic_id: String,
    pub incl_opt: Vec<ChapterInclOpt>,
}
