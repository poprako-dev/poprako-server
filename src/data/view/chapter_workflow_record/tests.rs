use super::*;

use time::OffsetDateTime;

use crate::value::role::RoleField;

// workflow_record_view_renders_each_payload_without_actor_data(ChapterWorkflowRecordInfoView)(positive): every structured payload has localized text without user-profile resolution.
#[test]
fn workflow_record_view_renders_each_payload_without_actor_data() {
    let payloads = vec![
        ChapterWorkflowRecordPayload::ChapterCreated,
        ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
            previous_subtitle: "before".into(),
            next_subtitle: "after".into(),
        },
        ChapterWorkflowRecordPayload::ChapterPinned,
        ChapterWorkflowRecordPayload::ChapterUnpinned,
        ChapterWorkflowRecordPayload::AssignmentCreated {
            subject_user_id: "subject-user".into(),
            roles: RoleMask::from(RoleField::TRANSLATOR),
        },
        ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
            subject_user_id: "subject-user".into(),
            previous_roles: RoleMask::from(RoleField::TRANSLATOR),
            next_roles: RoleMask::from(RoleField::PROOFREADER),
        },
        ChapterWorkflowRecordPayload::AssignmentDeleted {
            subject_user_id: "subject-user".into(),
            previous_roles: RoleMask::from(RoleField::TRANSLATOR),
        },
        ChapterWorkflowRecordPayload::TranslationImported {
            format: TranslationFormat::PopRaKo,
            imported_page_count: 2,
            imported_unit_count: 4,
        },
        ChapterWorkflowRecordPayload::TranslationExported {
            format: TranslationFormat::LabelPlus,
        },
        ChapterWorkflowRecordPayload::StageTransitioned {
            stage: Stage::Translate,
            previous_phase: StagePhase::Pending,
            next_phase: StagePhase::Active,
            origin: ChapterWorkflowRecordOrigin::Manual,
        },
    ];

    for payload in payloads {
        let workflow_record_info = ChapterWorkflowRecordInfo {
            id: "record-1".into(),
            chapter_id: "chapter-1".into(),
            actor_user_id: Some("actor-user".into()),
            kind: payload.kind(),
            payload,
            created_at: OffsetDateTime::UNIX_EPOCH,
        };

        let workflow_record_view =
            ChapterWorkflowRecordInfoView::from(workflow_record_info);

        assert!(!workflow_record_view.text.is_empty());

        assert!(!workflow_record_view.text.contains("actor-user"));

        assert!(!workflow_record_view.text.contains("subject-user"));

        assert!(workflow_record_view.payload.is_object());

        assert!(workflow_record_view.payload.get("type").is_none());
    }
}
