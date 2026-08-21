//! Insert models for immutable chapter workflow records.

use time::OffsetDateTime;

use crate::util::next_snowflake_id;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

/// One immutable chapter workflow record to persist within a transaction.
pub struct ChapterWorkflowRecordEntry {
    //
    /// Unique record identifier.
    pub id: String,
    /// Chapter that owns this record.
    pub chapter_id: String,
    /// User that caused the event, or `None` for a system operation.
    pub actor_user_id: Option<String>,
    /// Typed event details. Its kind is derived at storage time.
    pub payload: ChapterWorkflowRecordPayload,
    /// Fixed record creation timestamp.
    pub created_at: OffsetDateTime,
}

impl ChapterWorkflowRecordEntry {
    /// Constructs one immutable record with a snowflake ID and current UTC time.
    pub fn new<C>(
        chapter_id: C,
        actor_user_id: Option<String>,
        payload: ChapterWorkflowRecordPayload,
    ) -> Self
    where
        C: Into<String>,
    {
        //
        Self {
            id: next_snowflake_id(),
            chapter_id: chapter_id.into(),
            actor_user_id,
            payload,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}
