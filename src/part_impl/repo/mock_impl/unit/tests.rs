use super::*;

use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::result::BaseError;

// Internal implementation of `create_edit`.
fn create_edit(id: &str, text: &str) -> UnitEdit {
    UnitEdit::Create {
        id: id.to_string(),
        next_id: None,
        is_bubble: true,
        is_flagged: false,
        coord: UnitCoord {
            x_coord: 1.0,
            y_coord: 2.0,
        },
        translation: Some(UnitTranslation {
            translated_text: text.to_string(),
            last_translator_id: "translator-1".to_string(),
        }),
        revision: None,
    }
}

#[test]
fn apply_edits_soft_deletes_and_restores_a_unit() {
    //
    // Internal implementation detail.
    let mut state = MockState::default();

    let create = create_edit("unit-1", "translated");

    let create_order = [UnitOrder {
        id: "unit-1".to_string(),
        next_id: None,
        is_hidden: false,
    }];

    let count_metrics =
        apply_edits(&mut state, "page-1", &[], &[create]).unwrap();

    assert_eq!(count_metrics.total, 1);

    let hidden = apply_edits(
        &mut state,
        "page-1",
        &create_order,
        &[UnitEdit::Delete {
            id: "unit-1".to_string(),
        }],
    )
    .unwrap();

    assert_eq!(hidden.total, 0);

    assert!(state.units[0].hidden_at.is_some());

    let unit_infos = list_infos(&state, "page-1").unwrap();

    assert_eq!(unit_infos.len(), 1);

    assert!(unit_infos[0].hidden_at.is_some());

    let restore = UnitEdit::Save {
        id: "unit-1".to_string(),
        next_id: Patch::Skip,
        is_bubble: None,
        is_flagged: None,
        coord: None,
        translation: Patch::Skip,
        revision: Patch::Assign {
            value: UnitRevision {
                is_proofread: true,
                proofread_text: Some("proofread".to_string()),
                last_proofreader_id: "proofreader-1".to_string(),
            },
        },
    };

    let hidden_order = [UnitOrder {
        id: "unit-1".to_string(),
        next_id: None,
        is_hidden: true,
    }];

    let restored =
        apply_edits(&mut state, "page-1", &hidden_order, &[restore]).unwrap();

    assert_eq!(restored.total, 1);

    assert_eq!(restored.translated, 1);

    assert_eq!(restored.proofread, 1);

    assert!(state.units[0].hidden_at.is_none());
}

#[test]
fn reads_preserve_page_repetitions_tombstones_and_independent_snapshots() {
    //
    let mut state = MockState::default();

    let first_edit = create_edit("first", "first content");

    let second_edit = create_edit("second", "second content");

    let other_edit = create_edit("other", "other content");

    let first = unit_from_edit("page-1", &first_edit, Some("second")).unwrap();

    let mut second = unit_from_edit("page-1", &second_edit, None).unwrap();

    second.hidden_at = Some(now());

    let other = unit_from_edit("page-2", &other_edit, None).unwrap();

    state.units = vec![second, other, first];

    let orders = list_orders(&state, "page-1").unwrap();

    assert_eq!(
        orders
            .iter()
            .map(|order| (order.id.as_str(), order.is_hidden))
            .collect::<Vec<_>>(),
        [("first", false), ("second", true)],
    );

    let snapshots = list_infos_by_page_ids(
        &state,
        &["page-2", "page-1", "missing", "page-1"],
    )
    .unwrap();

    assert_eq!(
        snapshots
            .iter()
            .map(|unit_info| unit_info.id.as_str())
            .collect::<Vec<_>>(),
        ["other", "first", "second", "first", "second"],
    );

    state.units[2].translated_text = Some("updated".into());

    assert_eq!(
        snapshots[1].translated_text.as_deref(),
        Some("first content")
    );

    assert!(snapshots[2].hidden_at.is_some());
}

#[test]
fn reads_and_edit_counts_reject_corrupt_hidden_links() {
    //
    let mut state = MockState::default();

    let edit = create_edit("hidden", "content");

    let mut unit_info =
        unit_from_edit("page-1", &edit, Some("missing")).unwrap();

    unit_info.hidden_at = Some(now());

    state.units.push(unit_info);

    assert!(matches!(
        list_orders(&state, "page-1"),
        Err(BaseError::Unrecoverable { .. }),
    ));

    assert!(matches!(
        list_infos_by_page_ids(&state, &["page-1"]),
        Err(BaseError::Unrecoverable { .. }),
    ));

    assert!(matches!(
        apply_edits(&mut state, "page-1", &[], &[]),
        Err(BaseError::Unrecoverable { .. }),
    ));
}
