use std::borrow::Cow;

use super::ChapterWorkflowRecordEntry;

use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

// construction(new)(positive): an existing chapter and actor remain borrowed through record construction.
#[test]
fn workflow_record_preserves_borrowed_identifiers() {
    let chapter_id = String::from("chapter-1");

    let actor_user_id = String::from("user-1");

    let entry = ChapterWorkflowRecordEntry::new(
        chapter_id.as_str(),
        Some(actor_user_id.as_str().into()),
        ChapterWorkflowRecordPayload::ChapterCreated,
    );

    assert!(matches!(entry.chapter_id, Cow::Borrowed(_)));

    assert!(matches!(entry.actor_user_id, Some(Cow::Borrowed(_))));

    assert_eq!(entry.chapter_id.as_ptr(), chapter_id.as_ptr());

    assert_eq!(
        entry.actor_user_id.as_ref().unwrap().as_ptr(),
        actor_user_id.as_ptr()
    );
}

// construction(new)(positive): moved identifiers remain owned without reallocating their storage.
#[test]
fn workflow_record_preserves_owned_identifiers() {
    let chapter_id = String::from("chapter-1");

    let chapter_ptr = chapter_id.as_ptr();

    let actor_user_id = String::from("user-1");

    let actor_ptr = actor_user_id.as_ptr();

    let entry = ChapterWorkflowRecordEntry::new(
        chapter_id,
        Some(actor_user_id.into()),
        ChapterWorkflowRecordPayload::ChapterCreated,
    );

    assert!(matches!(entry.chapter_id, Cow::Owned(_)));

    assert!(matches!(entry.actor_user_id, Some(Cow::Owned(_))));

    assert_eq!(entry.chapter_id.as_ptr(), chapter_ptr);

    assert_eq!(entry.actor_user_id.as_ref().unwrap().as_ptr(), actor_ptr);
}
