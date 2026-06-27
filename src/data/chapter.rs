//! Data transfer objects for chapter use cases.

use poprako_util::time::ToUnixMilli;

use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::role::RoleMask;
use crate::value::chapter::{StagePhase, WorkflowEvent, WorkflowStage};

/// Presentation-ready chapter information.
pub struct ChapterInfoVal {
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
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<ChapterInfo> for ChapterInfoVal {
    fn from(model: ChapterInfo) -> Self {
        Self {
            id: model.id,
            comic_id: model.comic_id,
            is_pinned: model.is_pinned,
            index: model.index,
            subtitle: model.subtitle,
            page_count: model.page_count,
            total_unit_count: model.total_unit_count,
            translated_unit_count: model.translated_unit_count,
            proofread_unit_count: model.proofread_unit_count,
            raw_provide_phase: model.raw_provide_phase,
            translate_phase: model.translate_phase,
            proofread_phase: model.proofread_phase,
            typeset_redraw_phase: model.typeset_redraw_phase,
            review_phase: model.review_phase,
            publish_phase: model.publish_phase,
            creator_id: model.creator_id,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Presentation-ready assignment information.
pub struct AssignmentInfoVal {
    pub id: String,
    pub chapter_id: String,
    pub user_id: String,
    pub role_mask: RoleMask,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<AssignmentInfo> for AssignmentInfoVal {
    fn from(model: AssignmentInfo) -> Self {
        Self {
            id: model.id,
            chapter_id: model.chapter_id,
            user_id: model.user_id,
            role_mask: model.role_mask,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for creating a chapter.
pub struct CreateChapterData {
    pub comic_id: String,
    pub subtitle: Option<String>,
}

/// Return value from successful chapter creation.
pub struct CreateChapterVal {
    pub id: String,
}

/// Input parameters for listing chapters.
pub struct ListChapterInfosData {
    pub comic_id: String,
    pub offset: u64,
    pub limit: u64,
}

/// Input parameters for updating chapter metadata.
pub struct UpdateChapterInfoData {
    pub id: String,
    pub subtitle: Option<String>,
    pub is_pinned: Option<bool>,
    pub workflow: Option<ChapterWorkflowData>,
}

/// Input parameters for updating chapter workflow.
pub struct ChapterWorkflowData {
    pub stage: WorkflowStage,
    pub event: WorkflowEvent,
}

/// Input parameters for joining a chapter assignment.
pub struct JoinChapterData {
    pub chapter_id: String,
    pub role_mask: RoleMask,
}
