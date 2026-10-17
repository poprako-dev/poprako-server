// save_edits(flagged)(positive): either editor can flag, clear, and restore without changing workflow.
// save_edits(flagged)(negative): unauthorized or invalid batches leave flags and tokens unchanged.

use super::*;

use serde_json::json;

// Decode a transport edit without coupling tests to every unrelated field.
fn edit(payload: serde_json::Value) -> UnitEditInstr {
    serde_json::from_value(payload).unwrap()
}

#[tokio::test]
async fn flagged_is_shared_and_does_not_start_workflow() {
    for role in [RoleField::TRANSLATOR, RoleField::PROOFREADER] {
        let mock = save_scope(RoleMask::from(role));

        save_edits(
            (&mock, &mock),
            token("translator-1"),
            save_instr(vec![edit(json!({
                "edit": "create", "local_id": "flagged", "is_bubble": true,
                "coord": {"x_coord": 1.0, "y_coord": 2.0}, "is_flagged": true
            }))]),
        )
        .await
        .unwrap();

        let id = mock.snapshot().units.first().unwrap().id.clone();

        assert!(mock.snapshot().units.first().unwrap().is_flagged);

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

            assert!(mock.snapshot().units.first().unwrap().is_flagged);
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

        assert!(!snapshot.units.first().unwrap().is_flagged);
        assert!(snapshot.units.first().unwrap().hidden_at.is_none());
        assert!(snapshot.units.first().unwrap().last_translator_id.is_none());
        assert!(
            snapshot
                .units
                .first()
                .unwrap()
                .last_proofreader_id
                .is_none()
        );
        assert_eq!(snapshot.pages.first().unwrap().total_unit_count, 1);
        assert_eq!(snapshot.pages.first().unwrap().translated_unit_count, 0);
        assert_eq!(snapshot.pages.first().unwrap().proofread_unit_count, 0);
        assert!(snapshot.chapter_workflow_records.is_empty());
    }
}

#[tokio::test]
async fn flagged_survives_text_changes_and_failed_batches() {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![create("local", None, Some("translated"))]),
    )
    .await
    .unwrap();

    let id = mock.snapshot().units.first().unwrap().id.clone();

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
        mock.snapshot()
            .units
            .first()
            .unwrap()
            .last_translator_id
            .as_deref(),
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

        assert!(mock.snapshot().units.first().unwrap().is_flagged);
    }

    let save_count = mock.unit_saves.token_count();

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
    assert!(mock.snapshot().units.first().unwrap().is_flagged);
    assert_eq!(mock.unit_saves.token_count(), save_count);
}

// Reusing a completed token skips even a changed flag payload.
#[tokio::test]
async fn retained_token_does_not_reapply_changed_flag() {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let save_id = uuid::Uuid::new_v4().to_string();

    let mut instr = save_instr(vec![create("local", None, None)]);

    instr.save_id.clone_from(&save_id);

    save_edits((&mock, &mock), token("translator-1"), instr)
        .await
        .unwrap();

    let mut changed = save_instr(vec![edit(json!({
        "edit": "create", "local_id": "local", "is_bubble": true,
        "coord": {"x_coord": 1.0, "y_coord": 2.0}, "is_flagged": true
    }))]);

    changed.save_id = save_id;

    save_edits((&mock, &mock), token("translator-1"), changed)
        .await
        .unwrap();

    assert_eq!(mock.snapshot().units.len(), 1);

    assert!(!mock.snapshot().units.first().unwrap().is_flagged);
}
