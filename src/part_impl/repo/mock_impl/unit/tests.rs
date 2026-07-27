use super::*;

use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::result::BaseError;

fn create_edit(id: &str, text: &str) -> UnitEdit {
    UnitEdit::Save {
        id: id.to_string(),
        next_id: PatchField::Clear,
        is_bubble: Some(true),
        coord: Some(UnitCoord {
            x_coord: 1.0,
            y_coord: 2.0,
        }),
        translation: PatchField::Assign(UnitTranslation {
            translated_text: text.to_string(),
            last_translator_id: "translator-1".to_string(),
        }),
        revision: PatchField::Clear,
    }
}

#[test]
fn apply_edits_soft_deletes_and_restores_a_unit() {
    //
    let mut state = MockState::default();

    let create = create_edit("unit-1", "translated");

    let create_order = [UnitOrder {
        id: "unit-1".to_string(),
        next_id: None,
        is_hidden: false,
    }];

    let counters =
        apply_edits(&mut state, "page-1", &create_order, &[create]).unwrap();

    assert_eq!(counters.total_unit_count, 1);

    let hidden_order = [UnitOrder {
        id: "unit-1".to_string(),
        next_id: None,
        is_hidden: true,
    }];

    let hidden = apply_edits(
        &mut state,
        "page-1",
        &hidden_order,
        &[UnitEdit::Delete {
            id: "unit-1".to_string(),
        }],
    )
    .unwrap();

    assert_eq!(hidden.total_unit_count, 0);

    assert!(state.units[0].hidden_at.is_some());

    let restore = UnitEdit::Save {
        id: "unit-1".to_string(),
        next_id: PatchField::Skip,
        is_bubble: None,
        coord: None,
        translation: PatchField::Skip,
        revision: PatchField::Assign(UnitRevision {
            is_proofread: true,
            proofread_text: Some("proofread".to_string()),
            last_proofreader_id: "proofreader-1".to_string(),
        }),
    };

    let visible_order = [UnitOrder {
        id: "unit-1".to_string(),
        next_id: None,
        is_hidden: false,
    }];

    let restored =
        apply_edits(&mut state, "page-1", &visible_order, &[restore]).unwrap();

    assert_eq!(restored.total_unit_count, 1);

    assert_eq!(restored.translated_unit_count, 1);

    assert_eq!(restored.proofread_unit_count, 1);

    assert!(state.units[0].hidden_at.is_none());
}

#[test]
fn order_unit_orders_rejects_a_forked_chain() {
    //
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
