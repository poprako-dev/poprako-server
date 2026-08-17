use super::*;

use crate::value::role::RoleField;

// payloads_round_trip_through_tagged_and_storage_forms(ChapterWorkflowRecordPayload)(positive): every typed payload decodes from its separate kind and JSON-object storage fields.
#[test]
fn payloads_round_trip_through_tagged_and_storage_forms() {
    let translator_role = RoleMask::from(RoleField::TRANSLATOR);

    let payloads = vec![
        ChapterWorkflowRecordPayload::ChapterCreated,
        ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
            previous_subtitle: "before".into(),
            next_subtitle: "after".into(),
        },
        ChapterWorkflowRecordPayload::ChapterPinned,
        ChapterWorkflowRecordPayload::ChapterUnpinned,
        ChapterWorkflowRecordPayload::AssignmentCreated {
            subject_user_id: "user-1".into(),
            roles: translator_role,
        },
        ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
            subject_user_id: "user-1".into(),
            previous_roles: translator_role,
            next_roles: RoleMask::from(RoleField::PROOFREADER),
        },
        ChapterWorkflowRecordPayload::AssignmentDeleted {
            subject_user_id: "user-1".into(),
            previous_roles: translator_role,
        },
        ChapterWorkflowRecordPayload::TranslationImported {
            format: TranslationFormat::PopRaKo,
            imported_page_count: 3,
            imported_unit_count: 8,
        },
        ChapterWorkflowRecordPayload::TranslationExported {
            format: TranslationFormat::LabelPlus,
        },
        ChapterWorkflowRecordPayload::StageTransitioned {
            stage: Stage::Translate,
            previous_phase: StagePhase::Pending,
            next_phase: StagePhase::Active,
            origin: ChapterWorkflowRecordOrigin::UnitEdit,
        },
    ];

    for payload in payloads {
        let tagged_json = serde_json::to_value(&payload).unwrap();

        let tagged_round_trip =
            serde_json::from_value::<ChapterWorkflowRecordPayload>(tagged_json)
                .unwrap();

        assert_eq!(tagged_round_trip, payload);

        let storage_json = payload.to_storage_json();

        assert!(storage_json.is_object());

        assert!(storage_json.get("type").is_none());

        let storage_round_trip =
            ChapterWorkflowRecordPayload::from_storage_json(
                payload.kind(),
                storage_json,
            )
            .unwrap();

        assert_eq!(storage_round_trip, payload);
    }
}

// storage_payload_rejects_kind_mismatch_and_extra_fields(ChapterWorkflowRecordPayload)(negative): persisted payloads must exactly match their separate kind.
#[test]
fn storage_payload_rejects_kind_mismatch_and_extra_fields() {
    let mismatch = ChapterWorkflowRecordPayload::from_storage_json(
        ChapterWorkflowRecordKind::ChapterCreated,
        serde_json::json!({ "unexpected": true }),
    );

    assert!(mismatch.is_err());

    let missing_field = ChapterWorkflowRecordPayload::from_storage_json(
        ChapterWorkflowRecordKind::StageTransitioned,
        serde_json::json!({ "stage": "translate" }),
    );

    assert!(missing_field.is_err());
}
