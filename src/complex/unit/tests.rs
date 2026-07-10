// prepare_diff(UnitComplex::prepare_diff)(positive): create, save, and delete opers are preserved while create ids are mapped.
// prepare_diff(UnitComplex::prepare_diff)(positive): delete and later save on the same id remain ordered replay opers.
// prepare_diff(UnitComplex::prepare_diff)(negative): invalid ids and duplicate local ids are rejected.
// apply_opers_to_order(UnitComplex::apply_opers_to_order)(positive): create and save place units before the anchor or at the tail.
// apply_opers_to_order(UnitComplex::apply_opers_to_order)(positive): delete removes a unit and preserves the remaining order.
// apply_opers_to_order(UnitComplex::apply_opers_to_order)(positive): save upsert restores a missing unit at the tail.
// build_index_updates(UnitComplex::build_index_updates)(positive): persisted server order is compacted without client-provided order.
// build_index_updates(UnitComplex::build_index_updates)(positive): already compact server order emits no index updates.

use super::*;

use crate::model::unit::{UnitDiff, UnitIndex, UnitOper, UnitPayload};
use crate::result::{ExpectedVariant, RegularError};

fn payload(text: &str, proofread: bool) -> UnitPayload {
    UnitPayload {
        is_bubble: true,
        is_proofread: proofread,
        x_coord: 1.0,
        y_coord: 2.0,
        translated_text: Some(text.into()),
        last_translator_id: None,
        proofread_text: None,
        last_proofreader_id: None,
    }
}

fn diff(opers: Vec<UnitOper>) -> UnitDiff {
    UnitDiff {
        page_id: "page-1".into(),
        opers,
    }
}

fn assert_args_error(error: RegularError) {
    match error {
        //
        RegularError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::Args));
        }

        RegularError::Unrecoverable { .. } => {
            panic!("expected argument error");
        }
    }
}

#[test]
fn prepare_diff_maps_create_ids_and_keeps_oper_order() {
    //
    let unit_diff = diff(vec![
        UnitOper::Save {
            local_id: None,
            id: Some("unit-a".into()),
            payload: payload("alpha", false),
            before_id: None,
        },
        UnitOper::Save {
            local_id: Some("local-x".into()),
            id: None,
            payload: payload("inserted", true),
            before_id: Some("unit-a".into()),
        },
        UnitOper::Delete {
            id: "unit-b".into(),
        },
        UnitOper::Save {
            local_id: None,
            id: Some("unit-a".into()),
            payload: payload("alpha-later", false),
            before_id: None,
        },
    ]);

    let receipt = match UnitComplex::prepare_diff(unit_diff) {
        //
        Ok(receipt) => receipt,

        Err(_) => panic!("expected valid difference"),
    };

    assert_eq!(receipt.opers.len(), 4);

    assert_eq!(receipt.local_id_map.len(), 1);

    assert_eq!(receipt.local_id_map[0].local_id, "local-x");

    assert!(!receipt.local_id_map[0].unit_id.is_empty());

    match &receipt.opers[1] {
        //
        UnitOper::Save { id, local_id, .. } => {
            //
            assert!(local_id.is_none());

            assert_eq!(
                id.as_deref(),
                Some(receipt.local_id_map[0].unit_id.as_str())
            );
        }

        UnitOper::Delete { .. } => {
            panic!("expected save oper");
        }
    }
}

#[test]
fn prepare_diff_rejects_invalid_compact_diff() {
    //
    let empty_id_error =
        UnitComplex::prepare_diff(diff(vec![UnitOper::Save {
            local_id: None,
            id: Some(String::new()),
            payload: payload("alpha", false),
            before_id: None,
        }]))
        .err()
        .unwrap();

    assert_args_error(empty_id_error);

    let duplicate_local_id_error = UnitComplex::prepare_diff(diff(vec![
        UnitOper::Save {
            local_id: Some("local-x".into()),
            id: None,
            payload: payload("one", false),
            before_id: None,
        },
        UnitOper::Save {
            local_id: Some("local-x".into()),
            id: None,
            payload: payload("two", false),
            before_id: None,
        },
    ]))
    .err()
    .unwrap();

    assert_args_error(duplicate_local_id_error);
}

#[test]
fn prepare_diff_keeps_delete_and_later_save_for_ordered_replay() {
    //
    let receipt = match UnitComplex::prepare_diff(diff(vec![
        UnitOper::Delete {
            id: "unit-a".into(),
        },
        UnitOper::Save {
            local_id: None,
            id: Some("unit-a".into()),
            payload: payload("alpha", false),
            before_id: None,
        },
    ])) {
        //
        Ok(receipt) => receipt,

        Err(_) => panic!("expected delete and later save to be valid"),
    };

    assert_eq!(receipt.opers.len(), 2);
}

#[test]
fn apply_opers_to_order_places_create_and_save_before_anchor_or_tail() {
    //
    let opers = vec![
        UnitOper::Save {
            local_id: None,
            id: Some("unit-x".into()),
            payload: payload("x", false),
            before_id: Some("unit-b".into()),
        },
        UnitOper::Save {
            local_id: None,
            id: Some("unit-a".into()),
            payload: payload("a", false),
            before_id: None,
        },
        UnitOper::Save {
            local_id: None,
            id: Some("unit-c".into()),
            payload: payload("c", false),
            before_id: Some("unit-missing".into()),
        },
    ];

    let current_order = vec!["unit-a".into(), "unit-b".into()];

    let final_order = UnitComplex::apply_opers_to_order(&opers, current_order);

    assert_eq!(final_order, vec!["unit-x", "unit-b", "unit-a", "unit-c"]);
}

#[test]
fn apply_opers_to_order_removes_deleted_unit_and_keeps_remaining_order() {
    //
    let opers = vec![UnitOper::Delete {
        id: "unit-b".into(),
    }];

    let current_order = vec!["unit-a".into(), "unit-b".into(), "unit-c".into()];

    let final_order = UnitComplex::apply_opers_to_order(&opers, current_order);

    assert_eq!(final_order, vec!["unit-a", "unit-c"]);
}

#[test]
fn apply_opers_to_order_save_upsert_restores_missing_unit_at_tail() {
    //
    let opers = vec![UnitOper::Save {
        local_id: None,
        id: Some("unit-z".into()),
        payload: payload("z", false),
        before_id: None,
    }];

    let current_order = vec!["unit-a".into(), "unit-b".into()];

    let final_order = UnitComplex::apply_opers_to_order(&opers, current_order);

    assert_eq!(final_order, vec!["unit-a", "unit-b", "unit-z"]);
}

#[test]
fn build_index_updates_compacts_server_order() {
    //
    let current_indexes = vec![
        UnitIndex {
            id: "unit-a".into(),
            index: 0,
        },
        UnitIndex {
            id: "unit-b".into(),
            index: 3,
        },
        UnitIndex {
            id: "unit-c".into(),
            index: 1,
        },
        UnitIndex {
            id: "unit-x".into(),
            index: 7,
        },
        UnitIndex {
            id: "unit-z".into(),
            index: 7,
        },
    ];

    let unit_index_updates = UnitComplex::build_index_updates(current_indexes);

    let ordered_pairs = unit_index_updates
        .iter()
        .map(|unit_index_update| {
            (unit_index_update.id.as_str(), unit_index_update.index)
        })
        .collect::<Vec<_>>();

    assert_eq!(
        ordered_pairs,
        vec![("unit-b", 2), ("unit-x", 3), ("unit-z", 4),]
    );
}

#[test]
fn build_index_updates_skips_compact_server_order() {
    //
    let current_indexes = vec![
        UnitIndex {
            id: "unit-a".into(),
            index: 0,
        },
        UnitIndex {
            id: "unit-b".into(),
            index: 1,
        },
        UnitIndex {
            id: "unit-c".into(),
            index: 2,
        },
    ];

    let unit_index_updates = UnitComplex::build_index_updates(current_indexes);

    assert!(unit_index_updates.is_empty());
}
