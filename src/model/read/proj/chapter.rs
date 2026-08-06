//! Domain models for chapters inside comics — workflow state, progress counters,
//! and display metadata tracked per chapter.
//!
//! See [`StagePhase`] and [`WorkflowStage`](crate::value::chapter::WorkflowStage)
//! for the six production stages and their phase transitions.
//! Convert to [`ChapterInfoView`] for presentation.
//!
//! [`ChapterInfoView`]: crate::data::view::chapter::ChapterInfoView
//! [`StagePhase`]: crate::value::chapter::StagePhase

use time::OffsetDateTime;

use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::user::UserInfo;
use crate::value::chapter::StageMask;

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
