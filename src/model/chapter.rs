//! Domain models for chapters inside comics.

use time::OffsetDateTime;

use crate::value::chapter::StagePhase;

/// A chapter record as stored in the database.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterInfo {
    pub id: String,
    pub comic_id: String,
    pub is_pinned: bool,
    pub index: i32,
    pub subtitle: String,
    pub page_count: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
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

/// The data needed to create a new chapter.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterForm {
    pub id: String,
    pub comic_id: String,
    pub is_pinned: bool,
    pub index: i32,
    pub subtitle: String,
    pub creator_id: String,
}

/// Mutable profile fields for a chapter.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterInfoUpdate {
    pub id: String,
    pub subtitle: Option<String>,
    pub is_pinned: Option<bool>,
}

/// Mutable workflow phase fields for a chapter.
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
