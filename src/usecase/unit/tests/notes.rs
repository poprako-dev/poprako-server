// save_edits(notes)(positive): editors share annotations and typography notes count as translation.
// save_edits(notes)(negative): rejected batches preserve notes, counters, and save receipts.

use super::*;

use serde_json::{Value, json};

use crate::value::chapter::stage::{Stage, StagePhase};

// Decode a public edit payload for the annotation lifecycle.
fn edit(value: Value) -> UnitEditInstr {
    serde_json::from_value(value).unwrap()
}

// Save one annotation operation through the public use case.
async fn save((mock, cache): (&Mock, &MemCache), value: Value) {
    save_edits(
        (mock, mock, cache),
        token("translator-1"),
        save_instr(vec![edit(value)]),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn notes_are_shared_and_typography_note_starts_translation() {
    for role in [RoleField::TRANSLATOR, RoleField::PROOFREADER] {
        let (mock, cache) = save_scope(RoleMask::from(role));

        save(
            (&mock, &cache),
            json!({
                "edit": "create",
                "local_id": "note",
                "is_bubble": true,
                "coord": {"x_coord": 0.1, "y_coord": 0.2},
                "typography_note": " 小字\n\n保留括号 ",
                "translation_note": "这句存疑"
            }),
        )
        .await;

        let snapshot = mock.snapshot();

        let unit = snapshot.units.first().unwrap();

        let id = unit.id.clone();

        assert_eq!(unit.typography_note.as_deref(), Some(" 小字\n\n保留括号 "));

        assert_eq!(unit.translation_note.as_deref(), Some("这句存疑"));

        assert!(unit.translated_text.is_none());

        assert!(unit.proofread_text.is_none());

        assert!(unit.last_translator_id.is_none());

        assert!(unit.last_proofreader_id.is_none());

        assert!(!unit.is_proofread);

        assert_eq!(snapshot.pages.first().unwrap().translated_unit_count, 1);

        assert_eq!(snapshot.chapters.first().unwrap().translated_unit_count, 1);

        assert_eq!(
            snapshot
                .chapters
                .first()
                .unwrap()
                .stages
                .get_phase(Stage::Translate),
            StagePhase::Active
        );

        assert_eq!(
            snapshot
                .chapters
                .first()
                .unwrap()
                .stages
                .get_phase(Stage::Proofread),
            StagePhase::Pending
        );

        for payload in [
            json!({"edit": "patch", "id": id, "typography_note": null}),
            json!({
                "edit": "patch",
                "id": id,
                "typography_note": {"type": "skip"},
            }),
            json!({"edit": "delete", "id": id}),
            json!({"edit": "patch", "id": id}),
        ] {
            save((&mock, &cache), payload).await;

            assert_eq!(
                mock.snapshot().units.first().unwrap().typography_note,
                unit.typography_note
            );
        }

        let listed = list_infos(
            (&mock, &mock),
            token("translator-1"),
            ListPageUnitInfosInstr {
                page_id: "page-1".into(),
            },
        )
        .await
        .unwrap();

        assert_eq!(
            listed.unit_infos.first().unwrap().translation_note,
            unit.translation_note
        );

        save(
            (&mock, &cache),
            json!({"edit": "patch", "id": id,
                "typography_note": {"type": "clear"},
                "translation_note": {"type": "assign", "value": "校对已确认"}
            }),
        )
        .await;

        let cleared = mock.snapshot();

        assert!(cleared.units.first().unwrap().typography_note.is_none());

        assert_eq!(
            cleared.units.first().unwrap().translation_note.as_deref(),
            Some("校对已确认")
        );

        assert_eq!(cleared.pages.first().unwrap().translated_unit_count, 0);

        assert_eq!(cleared.chapters.first().unwrap().translated_unit_count, 0);

        assert_eq!(
            cleared
                .chapters
                .first()
                .unwrap()
                .stages
                .get_phase(Stage::Translate),
            StagePhase::Active
        );
    }
}

#[tokio::test]
async fn note_patches_merge_and_blank_assignments_clear() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        save_instr(vec![
            edit(json!({
                "edit": "create",
                "local_id": "note",
                "is_bubble": true,
                "coord": {"x_coord": 0.1, "y_coord": 0.2},
                "typography_note": " \t\u{3000}",
            })),
            edit(json!({
                "edit": "patch",
                "id": "note",
                "translation_note": {"type": "assign", "value": "待确认"},
            })),
        ]),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    let id = snapshot.units.first().unwrap().id.clone();

    assert!(snapshot.units.first().unwrap().typography_note.is_none());

    assert_eq!(snapshot.pages.first().unwrap().translated_unit_count, 0);

    assert!(snapshot.chapter_workflow_records.is_empty());

    save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        save_instr(vec![
            edit(json!({
                "edit": "patch",
                "id": id,
                "typography_note": {"type": "assign", "value": "first"},
            })),
            edit(json!({
                "edit": "patch",
                "id": id,
                "typography_note": {"type": "assign", "value": "last"},
            })),
            edit(json!({
                "edit": "patch",
                "id": id,
                "translation_note": {"type": "clear"},
            })),
        ]),
    )
    .await
    .unwrap();

    assert_eq!(
        mock.snapshot()
            .units
            .first()
            .unwrap()
            .typography_note
            .as_deref(),
        Some("last")
    );

    assert!(
        mock.snapshot()
            .units
            .first()
            .unwrap()
            .translation_note
            .is_none()
    );

    save(
        (&mock, &cache),
        json!({"edit": "patch", "id": id,
            "typography_note": {"type": "assign", "value": "\n\t\u{3000}"}
        }),
    )
    .await;

    assert!(
        mock.snapshot()
            .units
            .first()
            .unwrap()
            .typography_note
            .is_none()
    );

    assert_eq!(
        mock.snapshot().pages.first().unwrap().translated_unit_count,
        0
    );
}

#[tokio::test]
async fn notes_preserve_authorship_and_reject_invalid_batches() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let instr = save_instr(vec![create("unit", None, Some("正文"))]);

    save_edits((&mock, &mock, &cache), token("translator-1"), instr)
        .await
        .unwrap();

    let id = mock.snapshot().units.first().unwrap().id.clone();

    save(
        (&mock, &cache),
        json!({
            "edit": "patch",
            "id": id,
            "typography_note": {"type": "assign", "value": "小字"},
        }),
    )
    .await;

    let before = mock.snapshot();

    let save_count = cache.unit_save_token_count();

    for (actor, edits) in [
        (
            "outsider",
            vec![edit(json!({
                "edit": "patch",
                "id": id,
                "typography_note": {"type": "clear"},
            }))],
        ),
        (
            "translator-1",
            vec![
                edit(json!({
                    "edit": "patch",
                    "id": id,
                    "typography_note": {"type": "clear"},
                })),
                edit(json!({
                    "edit": "patch",
                    "id": "missing",
                    "translation_note": {"type": "clear"},
                })),
            ],
        ),
    ] {
        assert!(
            save_edits((&mock, &mock, &cache), token(actor), save_instr(edits))
                .await
                .is_err()
        );

        assert_eq!(
            mock.snapshot().units.first().unwrap().typography_note,
            before.units.first().unwrap().typography_note
        );

        assert_eq!(cache.unit_save_token_count(), save_count);
    }

    assert_eq!(
        mock.snapshot()
            .units
            .first()
            .unwrap()
            .last_translator_id
            .as_deref(),
        Some("translator-1")
    );

    save(
        (&mock, &cache),
        json!({"edit": "patch", "id": id, "translation": {"type": "clear"}}),
    )
    .await;

    assert_eq!(
        mock.snapshot().pages.first().unwrap().translated_unit_count,
        1
    );

    let mut published = mock.snapshot().chapters.first().unwrap().clone();

    published.stages = published
        .stages
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    mock.state
        .lock()
        .unwrap()
        .chapters
        .first_mut()
        .unwrap()
        .stages = published.stages;

    let instr = save_instr(vec![edit(json!({
        "edit": "patch", "id": id, "typography_note": {"type": "clear"}
    }))]);

    assert!(
        save_edits((&mock, &mock, &cache), token("translator-1"), instr)
            .await
            .is_err()
    );

    assert_eq!(
        mock.snapshot()
            .units
            .first()
            .unwrap()
            .typography_note
            .as_deref(),
        Some("小字")
    );
}

// save_edits(notes)(positive): replayed receipts never reapply changed annotations.
#[tokio::test]
async fn completed_save_token_preserves_original_note() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let mut initial = save_instr(vec![edit(json!({
        "edit": "create", "local_id": "note", "is_bubble": true,
        "coord": {"x_coord": 0.1, "y_coord": 0.2}, "typography_note": "original"
    }))]);

    let save_id = initial.save_id.clone();

    let first =
        save_edits((&mock, &mock, &cache), token("translator-1"), initial)
            .await
            .unwrap();

    initial = save_instr(vec![edit(json!({
        "edit": "create", "local_id": "note", "is_bubble": true,
        "coord": {"x_coord": 0.1, "y_coord": 0.2}, "typography_note": "changed"
    }))]);

    initial.save_id = save_id;

    let replay =
        save_edits((&mock, &mock, &cache), token("translator-1"), initial)
            .await
            .unwrap();

    assert_eq!(first.created_unit_ids, replay.created_unit_ids);

    assert_eq!(mock.snapshot().units.len(), 1);

    assert_eq!(
        mock.snapshot()
            .units
            .first()
            .unwrap()
            .typography_note
            .as_deref(),
        Some("original")
    );

    assert_eq!(mock.snapshot().chapter_workflow_records.len(), 1);
}
