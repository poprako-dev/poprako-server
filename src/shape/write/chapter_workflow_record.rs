//! Insert models for immutable chapter workflow records.

#[cfg(test)]
mod tests;

use time::OffsetDateTime;

use crate::util::next_business_id;
use crate::value::business_id::BusinessIdKind;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

/// One immutable chapter workflow record to persist within a transaction.
pub struct ChapterWorkflowRecordEntryShape<'a> {
    /// Unique record identifier.
    pub id: String,

    /// Chapter that owns this record.
    pub chapter_id: &'a str,
    /// User that caused the event, or `None` for a system operation.
    pub actor_user_id: Option<&'a str>,

    /// Typed event details. Its kind is derived at storage time.
    pub payload: ChapterWorkflowRecordPayload,

    /// Fixed record creation timestamp.
    pub created_at: OffsetDateTime,
}

impl<'a> ChapterWorkflowRecordEntryShape<'a> {
    /// Constructs one immutable record with a snowflake ID and current UTC time.
    ///
    /// Chapter and actor identifiers borrow their owners through persistence.
    pub fn new(
        chapter_id: &'a str,
        actor_user_id: Option<&'a str>,
        payload: ChapterWorkflowRecordPayload,
    ) -> Self {
        Self {
            id: next_business_id(BusinessIdKind::ChapterWorkflowRecord),
            chapter_id,
            actor_user_id,
            payload,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}
