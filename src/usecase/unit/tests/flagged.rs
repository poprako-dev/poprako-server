// save_edits(flagged)(positive): either editor can flag, clear, and restore without changing workflow.
// save_edits(flagged)(negative): unauthorized or invalid batches leave flags and receipts unchanged.

use super::*;

use serde_json::json;
use sha2::{Digest as _, Sha256};

use crate::data::instr::unit_save::prepare_save;

// Decode a transport edit without coupling tests to every unrelated field.
fn edit(payload: serde_json::Value) -> UnitEditInstr {
    serde_json::from_value(payload).unwrap()
}

#[tokio::test]
async fn flagged_is_shared_and_does_not_start_workflow() {
    for role in [RoleField::TRANSLATOR, RoleField::PROOFREADER] {
        let mock = save_scope(RoleMask::from(role));

        let created = save_edits(
            (&mock, &mock),
            token("translator-1"),
            save_instr(vec![edit(json!({
                "edit": "create", "local_id": "flagged", "is_bubble": true,
                "coord": {"x_coord": 1.0, "y_coord": 2.0}, "is_flagged": true
            }))]),
        )
        .await
        .unwrap();

        let id = &created.created_unit_ids[0].unit_id;

        assert!(mock.snapshot().units[0].is_flagged);

        for payload in [
            json!({"edit": "patch", "id": id}),
            json!({"edit": "patch", "id": id, "is_flagged": null}),
            json!({"edit": "delete", "id": id}),
            json!({"edit": "patch", "id": id}),
        ] {
            save_edits(
                (&mock, &mock),
                token("translator-1"),
                save_instr(vec![edit(payload)]),
            )
            .await
            .unwrap();

            assert!(mock.snapshot().units[0].is_flagged);
        }

        save_edits(
            (&mock, &mock),
            token("translator-1"),
            save_instr(vec![
                edit(json!({"edit": "patch", "id": id, "is_flagged": true})),
                edit(json!({"edit": "patch", "id": id, "is_flagged": false})),
                edit(json!({"edit": "patch", "id": id})),
            ]),
        )
        .await
        .unwrap();

        let snapshot = mock.snapshot();

        assert!(!snapshot.units[0].is_flagged);
        assert!(snapshot.units[0].hidden_at.is_none());
        assert!(snapshot.units[0].last_translator_id.is_none());
        assert!(snapshot.units[0].last_proofreader_id.is_none());
        assert_eq!(snapshot.pages[0].total_unit_count, 1);
        assert_eq!(snapshot.pages[0].translated_unit_count, 0);
        assert_eq!(snapshot.pages[0].proofread_unit_count, 0);
        assert!(snapshot.chapter_workflow_records.is_empty());
    }
}

#[tokio::test]
async fn flagged_survives_text_changes_and_failed_batches() {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let created = save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![create("local", None, Some("translated"))]),
    )
    .await
    .unwrap();

    let id = &created.created_unit_ids[0].unit_id;

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![edit(
            json!({"edit": "patch", "id": id, "is_flagged": true}),
        )]),
    )
    .await
    .unwrap();

    assert_eq!(
        mock.snapshot().units[0].last_translator_id.as_deref(),
        Some("translator-1")
    );

    for translation in [
        json!({"type": "assign", "value": {"translated_text": "changed"}}),
        json!({"type": "clear"}),
    ] {
        save_edits(
            (&mock, &mock),
            token("translator-1"),
            save_instr(vec![edit(
                json!({"edit": "patch", "id": id, "translation": translation}),
            )]),
        )
        .await
        .unwrap();

        assert!(mock.snapshot().units[0].is_flagged);
    }

    let save_count = mock.state.lock().unwrap().unit_saves.len();

    let rejected = save_edits(
        (&mock, &mock),
        token("outsider"),
        save_instr(vec![edit(
            json!({"edit": "patch", "id": id, "is_flagged": false}),
        )]),
    )
    .await;

    assert!(matches!(
        rejected,
        Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            ..
        })
    ));

    let rejected = save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![
            edit(json!({"edit": "patch", "id": id, "is_flagged": false})),
            edit(json!({"edit": "patch", "id": "missing", "is_flagged": true})),
        ]),
    )
    .await;

    assert!(rejected.is_err());
    assert!(mock.snapshot().units[0].is_flagged);
    assert_eq!(mock.state.lock().unwrap().unit_saves.len(), save_count);
}

#[tokio::test]
async fn flagged_receipts_preserve_legacy_digest_and_reject_changed_flag() {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let save_id = uuid::Uuid::new_v4().to_string();

    let request = || SavePageUnitEditsInstr {
        save_id: save_id.clone(),
        page_id: "page-1".into(),
        edits: vec![create("local", None, None)],
    };

    let legacy_payload = br#"[{"edit":"create","local_id":"local","next_id":null,"is_bubble":true,"coord":{"x_coord":1.0,"y_coord":2.0},"translation":null,"revision":null}]"#;

    let (_, prepared) = prepare_save(request(), "translator-1").unwrap();

    assert_eq!(
        prepared.payload_digest,
        Sha256::digest(legacy_payload).to_vec()
    );

    let saved = save_edits((&mock, &mock), token("translator-1"), request())
        .await
        .unwrap();

    let replay = save_edits((&mock, &mock), token("translator-1"), request())
        .await
        .unwrap();

    assert_eq!(
        saved.created_unit_ids[0].unit_id,
        replay.created_unit_ids[0].unit_id
    );

    let mut changed = request();

    let UnitEditInstr::Create { is_flagged, .. } = &mut changed.edits[0] else {
        panic!("expected create");
    };

    *is_flagged = true;

    assert!(
        save_edits((&mock, &mock), token("translator-1"), changed)
            .await
            .is_err()
    );
    assert!(!mock.snapshot().units[0].is_flagged);

    let legacy_patch = br#"[{"edit":"patch","id":"unit-1","next_id":"Skip","is_bubble":null,"coord":null,"translation":"Skip","revision":"Skip"}]"#;

    let mut patch =
        save_instr(vec![edit(json!({"edit":"patch","id":"unit-1"}))]);

    let (_, prepared) = prepare_save(patch, "translator-1").unwrap();

    assert_eq!(
        prepared.payload_digest,
        Sha256::digest(legacy_patch).to_vec()
    );

    patch = save_instr(vec![edit(
        json!({"edit":"patch","id":"unit-1","is_flagged":false}),
    )]);

    let (_, changed) = prepare_save(patch, "translator-1").unwrap();

    assert_ne!(prepared.payload_digest, changed.payload_digest);
}
