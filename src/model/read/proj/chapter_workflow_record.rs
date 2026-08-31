//! Read projection for immutable chapter workflow records.

use time::OffsetDateTime;

use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordKind, ChapterWorkflowRecordPayload,
};

/// One immutable activity record attached to a chapter workflow.
#[derive(Clone)]
pub struct ChapterWorkflowRecordInfo {
    //
    /// Unique record identifier.
    pub id: String,

    /// Chapter that owns this record.
    pub chapter_id: String,
    /// User that caused the event, or `None` for a system operation.
    pub actor_user_id: Option<String>,

    /// Stable event kind.
    pub kind: ChapterWorkflowRecordKind,
    /// Typed, language-neutral event details.
    pub payload: ChapterWorkflowRecordPayload,

    /// Record creation timestamp.
    pub created_at: OffsetDateTime,
}
