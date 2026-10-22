#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions require valid plans"
)]

use super::*;

use crate::model::shared::unit::{UnitCoord, UnitTranslation};

// Construct a minimal persisted link.
fn order(id: &str, next_id: Option<&str>, is_hidden: bool) -> UnitOrder {
    UnitOrder {
        id: id.into(),
        next_id: next_id.map(str::to_owned),
        is_hidden,
    }
}

// Construct an order-only save.
fn save(id: &str, next_id: Patch<String>) -> UnitEdit {
    UnitEdit::Save {
        id: id.into(),
        next_id,
        is_bubble: None,
        is_flagged: None,
        coord: None,
        typography_note: Patch::Skip,
        translation_note: Patch::Skip,
        translation: Patch::Skip,
        revision: Patch::Skip,
    }
}

// Structural no-ops require target validation but produce no mutations.
#[test]
fn plan_skips_empty_save_unchanged_move_and_repeated_delete() {
    let orders = [
        order("a", Some("b"), false),
        order("b", Some("c"), true),
        order("c", None, false),
    ];

    let edits = [
        save("a", Patch::Skip),
        UnitEdit::Delete { id: "b".into() },
        save("c", Patch::Clear),
    ];

    let plan = plan_writes(&orders, &edits).unwrap();

    assert!(plan.creates.is_empty());

    assert!(plan.changes.is_empty());

    assert_eq!(plan.skipped_ids, ["a", "b", "c"]);
}

// Empty patches restore tombstones without resending order or content.
#[test]
fn plan_restores_hidden_unit_and_hides_visible_unit_without_order_changes() {
    let orders = [order("a", Some("b"), true), order("b", None, false)];

    let edits = [save("a", Patch::Skip), UnitEdit::Delete { id: "b".into() }];

    let plan = plan_writes(&orders, &edits).unwrap();

    assert_eq!(plan.changes.get("a").unwrap().is_hidden, Some(false));

    assert_eq!(plan.changes.get("b").unwrap().is_hidden, Some(true));

    assert!(
        plan.changes
            .values()
            .all(|change| change.next_id.is_skip() && change.edit.is_none())
    );
}

// Moving a node updates only final changed successors, including its neighbors.
#[test]
fn plan_moves_only_changed_links() {
    let orders = [
        order("a", Some("b"), false),
        order("b", Some("c"), false),
        order("c", Some("d"), false),
        order("d", None, false),
    ];

    let edits = [save("b", Patch::Clear)];

    let plan = plan_writes(&orders, &edits).unwrap();

    assert_eq!(
        plan.changes.keys().copied().collect::<Vec<_>>(),
        ["a", "b", "d"]
    );

    assert_eq!(
        plan.changes.get("a").unwrap().next_id,
        Patch::Assign { value: "c" }
    );

    assert_eq!(plan.changes.get("b").unwrap().next_id, Patch::Clear);

    assert_eq!(
        plan.changes.get("d").unwrap().next_id,
        Patch::Assign { value: "b" }
    );

    assert!(
        plan.changes
            .values()
            .all(|change| change.is_hidden.is_none() && change.edit.is_none())
    );
}

// New-row patches fold content after all moves, preserving final neighbor links.
#[test]
fn plan_folds_create_patch_content_after_sequence_planning() {
    let orders = [order("a", Some("b"), false), order("b", None, false)];

    let edits = [
        UnitEdit::Create {
            id: "new".into(),
            next_id: Some("b".into()),
            is_bubble: true,
            is_flagged: false,
            coord: UnitCoord {
                x_coord: 1.0,
                y_coord: 2.0,
            },
            typography_note: None,
            translation_note: None,
            translation: Some(UnitTranslation {
                translated_text: "initial".into(),
                last_translator_id: "first".into(),
            }),
            revision: None,
        },
        save("a", Patch::Clear),
        UnitEdit::Save {
            id: "new".into(),
            next_id: Patch::Assign { value: "a".into() },
            is_bubble: Some(false),
            is_flagged: Some(true),
            coord: None,
            typography_note: Patch::Skip,
            translation_note: Patch::Skip,
            translation: Patch::Clear,
            revision: Patch::Skip,
        },
    ];

    let plan = plan_writes(&orders, &edits).unwrap();

    assert_eq!(plan.creates.len(), 1);

    assert!(matches!(plan.creates.first().unwrap(), UnitEdit::Create {
        next_id: Some(next_id), is_bubble: false, is_flagged: true, translation: None, ..
    } if next_id == "a"));

    assert!(!plan.changes.contains_key("new"));

    assert_eq!(
        plan.changes.get("b").unwrap().next_id,
        Patch::Assign { value: "new" }
    );

    assert_eq!(plan.changes.get("a").unwrap().next_id, Patch::Clear);
}

// Explicit field assignments remain writes even when old content is unknown.
#[test]
fn plan_preserves_content_assignment_and_clear_without_forcing_structure() {
    let orders = [order("a", None, false)];

    let edits = [UnitEdit::Save {
        id: "a".into(),
        next_id: Patch::Skip,
        is_bubble: None,
        is_flagged: Some(false),
        coord: None,
        typography_note: Patch::Skip,
        translation_note: Patch::Skip,
        translation: Patch::Clear,
        revision: Patch::Skip,
    }];

    let plan = plan_writes(&orders, &edits).unwrap();

    let change = plan.changes.get("a").unwrap();

    assert!(change.next_id.is_skip());

    assert!(change.is_hidden.is_none());

    assert_eq!(change.edit, edits.first());
}
