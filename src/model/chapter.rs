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

use crate::value::chapter::StagePhase;

/// A chapter（章节）record as stored in the database.
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
#[cfg_attr(test, derive(Clone))]
pub struct ChapterInfo {
    pub id: String,
    pub comic_id: String,

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

    pub raw_provide_phase: StagePhase,
    pub translate_phase: StagePhase,
    pub proofread_phase: StagePhase,
    pub typeset_redraw_phase: StagePhase,
    pub review_phase: StagePhase,
    pub publish_phase: StagePhase,

    pub creator_id: String,

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
pub struct ChapterForm {
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
pub struct ChapterInfoUpdate {
    pub id: String,

    pub subtitle: Option<String>,
    pub is_pinned: Option<bool>,
}

/// Mutable workflow phase fields for a chapter.
///
/// Each `Option<StagePhase>` field represents a possible phase transition.
/// A `None` value leaves the current phase unchanged; a `Some` value
/// advances (or regresses) that stage to the given [`StagePhase`].
///
/// The production use case layer should validate transition legality
/// before building this update struct.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterStageUpdate {
    pub id: String,

    pub raw_provide_phase: Option<StagePhase>,
    pub translate_phase: Option<StagePhase>,
    pub proofread_phase: Option<StagePhase>,
    pub typeset_redraw_phase: Option<StagePhase>,
    pub review_phase: Option<StagePhase>,
    pub publish_phase: Option<StagePhase>,
}
