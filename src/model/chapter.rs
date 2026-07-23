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

use crate::model::comic::ComicInfo;
use crate::model::user::UserInfo;
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
pub struct ChapterInfo {
    /// Unique identifier for the chapter.
    pub id: String,
    /// Foreign key to the parent comic this chapter belongs to.
    pub comic_id: String,

    /// Optional joined comic data included when the query specifies comic expansion.
    pub comic: Option<ComicInfo>,

    /// Marks this chapter as the currently active chapter within its comic.
    pub is_pinned: bool,
    /// Ordinal position of this chapter within the comic, used for sorting.
    pub index: i32,
    /// Human-readable chapter subtitle or number, such as "Chapter 5".
    pub subtitle: String,

    /// Denormalised total count of units in this chapter.
    pub page_count: i32,
    /// Denormalised number of units submitted for translation.
    pub total_unit_count: i32,
    /// Denormalised number of units with a completed translation.
    pub translated_unit_count: i32,
    /// Denormalised number of units with a completed proofread.
    pub proofread_unit_count: i32,

    /// Bitmask tracking the completion phase of each workflow stage.
    pub stages: StageMask,

    /// Foreign key to the user who created this chapter record.
    pub creator_id: String,

    /// Optional joined user data for the chapter creator.
    pub creator: Option<UserInfo>,

    /// Timestamp when the chapter was created.
    pub created_at: OffsetDateTime,
    /// Timestamp of the last modification to the chapter.
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
pub struct ChapterEntry {
    /// Unique identifier to insert for the new chapter.
    pub id: String,
    /// Foreign key identifying the parent comic.
    pub comic_id: String,

    /// Whether the new chapter should be set as the active chapter immediately.
    pub is_pinned: bool,
    /// Ordinal position assigned to the new chapter within the comic.
    pub index: i32,
    /// Chapter subtitle or number for display.
    pub subtitle: String,

    /// Foreign key identifying the user creating the chapter.
    pub creator_id: String,
}

/// Mutable profile (non-workflow) fields for a chapter.
///
/// Only `subtitle` and `is_pinned` are user-editable through the profile
/// update endpoint. Workflow phase transitions are handled via
/// [`ChapterStageUpdate`] instead.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterInfoUpdate {
    /// Unique identifier of the chapter whose profile fields are being changed.
    pub id: String,

    /// New subtitle value, or `None` to leave unchanged.
    pub subtitle: Option<String>,
    /// New pinned state, or `None` to leave unchanged.
    pub pin: Option<bool>,
}

/// Mutable workflow stage mask for a chapter.
///
/// The use case layer validates transition legality before building this update.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterStageUpdate {
    /// Unique identifier of the chapter whose stages are being transitioned.
    pub id: String,

    /// Updated workflow stage mask after applying the transition.
    pub stages: StageMask,
}

/// Filtering, pagination, and include parameters for listing chapters.
pub struct ChapterInfoListSpec {
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
