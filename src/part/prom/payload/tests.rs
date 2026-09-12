use super::*;

// task_payload_serde(current_json)(positive): domain and operation tags round trip with required ownership.
#[test]
fn round_trips_current_contract() {
    //
    let cases = [
        (
            TaskPayload::Chapter {
                payload: ChapterPayload::TryAdvanceRawProvideStage {
                    chapter_id: "chapter-1".into(),
                    actor_user_id: "user-1".into(),
                },
            },
            serde_json::json!({
                "Chapter": { "payload": { "TryAdvanceRawProvideStage": {
                    "chapter_id": "chapter-1", "actor_user_id": "user-1"
                } } }
            }),
        ),
        (
            TaskPayload::Invitation {
                payload: InvitationPayload::Member {
                    invitation_id: "invitation-1".into(),
                },
            },
            serde_json::json!({
                "Invitation": { "payload": { "Member": {
                    "invitation_id": "invitation-1"
                } } }
            }),
        ),
    ];

    for (task, expected) in cases {
        let encoded = serde_json::to_value(&task).unwrap();

        assert_eq!(encoded, expected);

        assert_eq!(
            serde_json::from_value::<TaskPayload>(encoded).unwrap(),
            task
        );
    }
}

// task_payload_serde(invalid_json)(negative): historical tags and missing ownership are rejected.
#[test]
fn rejects_obsolete_or_incomplete_payloads() {
    //
    let invalid = [
        serde_json::json!({ "AdvanceRawProvide": { "chapter_id": "chapter-1" } }),
        serde_json::json!({
            "PurgeExpiredInvitation": { "Member": { "invitation_id": "invitation-1" } }
        }),
        serde_json::json!({
            "Chapter": { "payload": { "chapter_id": "chapter-1", "actor_user_id": "user-1" } }
        }),
        serde_json::json!({
            "Chapter": { "payload": { "TryAdvanceRawProvideStage": { "chapter_id": "chapter-1" } } }
        }),
    ];

    for payload in invalid {
        assert!(serde_json::from_value::<TaskPayload>(payload).is_err());
    }
}
