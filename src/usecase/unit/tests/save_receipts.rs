// save_edits(save_receipts)(positive): replay returns the original pairs without duplicate side effects.
// save_edits(save_receipts)(negative): changed payload and unauthorized replay are rejected.
// save_edits(save_receipts)(negative): rollback does not retain a successful receipt.
use super::*;

fn batch(save_id: &str, text: &str) -> SavePageUnitEditsInstr {
    let mut instr = save_instr(vec![create("local-save", None, Some(text))]);

    instr.save_id = save_id.into();

    instr
}

#[tokio::test]
async fn replay_returns_original_struct_array_and_preserves_counts() {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let save_id = uuid::Uuid::new_v4().to_string();

    let first = save_edits(
        (&mock, &mock),
        token("translator-1"),
        batch(&save_id, "saved"),
    )
    .await
    .unwrap();

    let records = mock.snapshot().chapter_workflow_records.len();

    let replay = save_edits(
        (&mock, &mock),
        token("translator-1"),
        batch(&save_id, "saved"),
    )
    .await
    .unwrap();

    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );

    assert_eq!(
        first.created_unit_ids.first().unwrap().local_id,
        "local-save"
    );

    assert_eq!(
        first.created_unit_ids.first().unwrap().unit_id,
        mock.snapshot().units.first().unwrap().id
    );

    assert_eq!(mock.snapshot().units.len(), 1);

    assert_eq!(mock.snapshot().pages.first().unwrap().total_unit_count, 1);

    assert_eq!(mock.snapshot().chapter_workflow_records.len(), records);

    let mismatch = save_edits(
        (&mock, &mock),
        token("translator-1"),
        batch(&save_id, "changed"),
    )
    .await
    .unwrap_err();

    assert!(matches!(
        mismatch,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));

    {
        let mut state = mock.state.lock().unwrap();

        state.assignments.clear();
    }

    assert!(
        save_edits(
            (&mock, &mock),
            token("translator-1"),
            batch(&save_id, "saved")
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn failed_batch_does_not_consume_save_identity() {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let save_id = uuid::Uuid::new_v4().to_string();

    let mut invalid = batch(&save_id, "text");

    invalid.edits.push(UnitEditInstr::Delete {
        id: "missing".into(),
    });

    assert!(
        save_edits((&mock, &mock), token("translator-1"), invalid)
            .await
            .is_err()
    );

    assert!(mock.state.lock().unwrap().unit_saves.is_empty());

    assert!(mock.snapshot().units.is_empty());

    assert!(
        save_edits(
            (&mock, &mock),
            token("translator-1"),
            batch(&save_id, "text")
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn delete_returns_an_empty_struct_array() {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let created = save_edits(
        (&mock, &mock),
        token("translator-1"),
        batch(&uuid::Uuid::new_v4().to_string(), "text"),
    )
    .await
    .unwrap();

    let deleted = save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![UnitEditInstr::Delete {
            id: created.created_unit_ids.first().unwrap().unit_id.clone(),
        }]),
    )
    .await
    .unwrap();

    assert_eq!(
        serde_json::to_value(deleted).unwrap(),
        serde_json::json!({"created_unit_ids": []})
    );
}

// A structural no-op still records its receipt and replays without touching Units.
#[tokio::test]
async fn empty_patch_records_receipt_without_refreshing_unit_timestamp() {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        batch(&uuid::Uuid::new_v4().to_string(), "saved"),
    )
    .await
    .unwrap();

    let unit_id = mock.snapshot().units.first().unwrap().id.clone();

    let original = time::OffsetDateTime::UNIX_EPOCH;

    mock.state
        .lock()
        .unwrap()
        .units
        .first_mut()
        .unwrap()
        .updated_at = original;

    let save_id = uuid::Uuid::new_v4().to_string();

    for _ in 0..2 {
        let mut instr = save_instr(vec![UnitEditInstr::Patch {
            id: unit_id.clone(),
            next_id: Patch::Skip,
            is_bubble: None,
            is_flagged: None,
            coord: None,
            translation: Patch::Skip,
            revision: Patch::Skip,
        }]);

        instr.save_id.clone_from(&save_id);

        let result = save_edits((&mock, &mock), token("translator-1"), instr)
            .await
            .unwrap();

        assert!(result.created_unit_ids.is_empty());

        assert_eq!(mock.snapshot().units.first().unwrap().updated_at, original);

        assert_eq!(mock.state.lock().unwrap().unit_saves.len(), 2);
    }
}
