use super::*;

use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::result::BaseError;

// Internal implementation of `create_edit`.
fn create_edit(id: &str, text: &str) -> UnitEdit {
    UnitEdit::Create {
        id: id.to_string(),
        next_id: None,
        is_bubble: true,
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
fn order_unit_orders_rejects_a_forked_chain() {
    //
    // Internal implementation detail.
    let mut unit_orders = vec![
        UnitOrder {
            id: "a".to_string(),
            next_id: Some("c".to_string()),
            is_hidden: false,
        },
        UnitOrder {
            id: "b".to_string(),
            next_id: Some("c".to_string()),
            is_hidden: false,
        },
        UnitOrder {
            id: "c".to_string(),
            next_id: None,
            is_hidden: false,
        },
    ];

    let error = order_units(
        &mut unit_orders,
        |unit_order| unit_order.id.as_str(),
        |unit_order| unit_order.next_id.as_deref(),
    )
    .unwrap_err();

    assert!(matches!(error, BaseError::Unrecoverable { .. }));
}
