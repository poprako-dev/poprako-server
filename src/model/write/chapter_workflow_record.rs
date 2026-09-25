//! Insert models for immutable chapter workflow records.

#[cfg(test)]
mod tests;

use std::borrow::Cow;

use time::OffsetDateTime;

use crate::util::next_snowflake_id;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

/// One immutable chapter workflow record to persist within a transaction.
pub struct ChapterWorkflowRecordEntry<'a> {
    /// Unique record identifier.
    pub id: String,

    /// Chapter that owns this record.
    pub chapter_id: Cow<'a, str>,
    /// User that caused the event, or `None` for a system operation.
    pub actor_user_id: Option<Cow<'a, str>>,

    /// Typed event details. Its kind is derived at storage time.
    pub payload: ChapterWorkflowRecordPayload,

    /// Fixed record creation timestamp.
    pub created_at: OffsetDateTime,
}

impl<'a> ChapterWorkflowRecordEntry<'a> {
    /// Constructs one immutable record with a snowflake ID and current UTC time.
    ///
    /// Existing identifiers stay borrowed; owned identifiers are moved unchanged.
    pub fn new<C>(
        chapter_id: C,
        actor_user_id: Option<Cow<'a, str>>,
        payload: ChapterWorkflowRecordPayload,
    ) -> Self
    where
        C: Into<Cow<'a, str>>,
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
