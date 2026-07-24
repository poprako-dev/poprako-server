use super::*;

// task_payload_serde(persisted_json)(positive): renamed task types preserve existing queue records.
#[test]
fn preserves_persisted_tags() {
    //
    let chapter_task = TaskPayload::Chapter(ChapterPayload::TryAdvanceRawProvideStage {
        chapter_id: "chapter-1".to_string(),
    });

    let chapter_json = serde_json::to_value(&chapter_task).unwrap();

    assert_eq!(
        chapter_json,
        serde_json::json!({ "AdvanceRawProvide": { "chapter_id": "chapter-1" } }),
    );

    let invitation_task = TaskPayload::Invitation(InvitationPayload::Member {
        invitation_id: "invitation-1".to_string(),
    });

    let invitation_json = serde_json::to_value(&invitation_task).unwrap();

    assert_eq!(
        invitation_json,
        serde_json::json!({
            "PurgeExpiredInvitation": { "Member": { "invitation_id": "invitation-1" } },
        }),
    );

    let decoded_task: TaskPayload =
        serde_json::from_value(invitation_json).unwrap();

    assert_eq!(decoded_task, invitation_task);
}
