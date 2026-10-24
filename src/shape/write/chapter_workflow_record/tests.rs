#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::ChapterWorkflowRecordEntryShape;

use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

// construction(new)(positive): an existing chapter and actor remain borrowed through record construction.
#[test]
fn workflow_record_preserves_borrowed_identifiers() {
    let chapter_id = String::from("chapter-1");

    let actor_user_id = String::from("user-1");

    let entry = ChapterWorkflowRecordEntryShape::new(
        chapter_id.as_str(),
        Some(actor_user_id.as_str()),
        ChapterWorkflowRecordPayload::ChapterCreated,
    );

    assert_eq!(entry.chapter_id.as_ptr(), chapter_id.as_ptr());

    assert_eq!(
        entry.actor_user_id.as_ref().unwrap().as_ptr(),
        actor_user_id.as_ptr()
    );
}
