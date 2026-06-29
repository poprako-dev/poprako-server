// prepare_difference(UnitComplex::prepare_difference)(positive): create, save, and delete opers are preserved while create ids are mapped.
// prepare_difference(UnitComplex::prepare_difference)(negative): invalid ids, duplicate local ids, duplicate candidate ids, deleted candidate ids, and missing candidate ids are rejected.
// build_index_updates(UnitComplex::build_index_updates)(positive): candidate order resolves local ids, skips stale anchors, preserves unknown units, and emits changed indexes only.
// build_index_updates(UnitComplex::build_index_updates)(positive): order-only differences produce compact index updates without unit mutations.

use super::*;

use crate::model::unit::{UnitDiff, UnitIdMap, UnitIndex, UnitOper, UnitPayload};
use crate::result::{ExpectedVariant, RootError};

fn payload(text: &str, proofread: bool) -> UnitPayload {
    UnitPayload {
        is_bubble: true,
        is_proofread: proofread,
        x_coord: 1.0,
        y_coord: 2.0,
        translated_text: Some(text.into()),
        translator_comment: None,
        last_translator_id: None,
        proofread_text: None,
        proofreader_comment: None,
        last_proofreader_id: None,
    }
}

fn diff(opers: Vec<UnitOper>, candidate_order: Vec<&str>) -> UnitDiff {
    UnitDiff {
        page_id: "page-1".into(),
        opers,
        candidate_order: candidate_order.into_iter().map(Into::into).collect(),
    }
}

fn assert_args_error(error: RootError) {
    match error {
        RootError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::Args));
        }
        RootError::Unrecoverable { .. } => {
            panic!("expected argument error");
        }
    }
}

#[test]
fn prepare_diff_maps_create_ids_and_keeps_oper_order() {
    let unit_diff = diff(
        vec![
            UnitOper::Save {
                id: "unit-a".into(),
                payload: payload("alpha", false),
            },
            UnitOper::Create {
                local_id: "local-x".into(),
                id: None,
                payload: payload("inserted", true),
            },
            UnitOper::Delete {
                id: "unit-b".into(),
            },
            UnitOper::Save {
                id: "unit-a".into(),
                payload: payload("alpha-later", false),
            },
        ],
        vec!["unit-a", "local-x"],
    );

    let receipt = match UnitComplex::prepare_diff(unit_diff) {
        Ok(receipt) => receipt,
        Err(_) => panic!("expected valid difference"),
    };

    assert_eq!(receipt.opers.len(), 4);
    assert_eq!(receipt.local_id_maps.len(), 1);
    assert_eq!(receipt.local_id_maps[0].local_id, "local-x");
    assert!(!receipt.local_id_maps[0].unit_id.is_empty());
    assert_eq!(receipt.candidate_order[0], "unit-a");
    assert_eq!(
        receipt.candidate_order[1],
        receipt.local_id_maps[0].unit_id
    );

    match &receipt.opers[1] {
        UnitOper::Create { local_id, id, .. } => {
            assert_eq!(local_id, "local-x");
            assert_eq!(
                id.as_deref(),
                Some(receipt.local_id_maps[0].unit_id.as_str())
            );
        }
        UnitOper::Save { .. } | UnitOper::Delete { .. } => {
            panic!("expected create oper");
        }
    }
}

#[test]
fn prepare_diff_rejects_invalid_compact_diff() {
    let empty_id_error = UnitComplex::prepare_diff(diff(
        vec![UnitOper::Save {
            id: String::new(),
            payload: payload("alpha", false),
        }],
        vec!["unit-a"],
    ))
    .err()
    .unwrap();

    assert_args_error(empty_id_error);

    let duplicate_local_id_error = UnitComplex::prepare_diff(diff(
        vec![
            UnitOper::Create {
                local_id: "local-x".into(),
                id: None,
                payload: payload("one", false),
            },
            UnitOper::Create {
                local_id: "local-x".into(),
                id: None,
                payload: payload("two", false),
            },
        ],
        vec!["local-x"],
    ))
    .err()
    .unwrap();

    assert_args_error(duplicate_local_id_error);

    let duplicate_candidate_error =
        UnitComplex::prepare_diff(diff(Vec::new(), vec!["unit-a", "unit-a"]))
            .err()
            .unwrap();

    assert_args_error(duplicate_candidate_error);

    let deleted_candidate_error = UnitComplex::prepare_diff(diff(
        vec![UnitOper::Delete {
            id: "unit-a".into(),
        }],
        vec!["unit-a"],
    ))
    .err()
    .unwrap();

    assert_args_error(deleted_candidate_error);

    let missing_candidate_error = UnitComplex::prepare_diff(diff(
        vec![UnitOper::Create {
            local_id: "local-x".into(),
            id: None,
            payload: payload("one", false),
        }],
        Vec::new(),
    ))
    .err()
    .unwrap();

    assert_args_error(missing_candidate_error);

    let save_delete_error = UnitComplex::prepare_diff(diff(
        vec![
            UnitOper::Save {
                id: "unit-a".into(),
                payload: payload("alpha", false),
            },
            UnitOper::Delete {
                id: "unit-a".into(),
            },
        ],
        vec!["unit-a"],
    ))
    .err()
    .unwrap();

    assert_args_error(save_delete_error);
}

#[test]
fn build_index_updates_resolves_local_ids_and_preserves_unknown_units() {
    let candidate_order = vec![
        "unit-c".into(),
        "local-x".into(),
        "stale-anchor".into(),
        "unit-a".into(),
    ];
    let local_id_maps = vec![UnitIdMap {
        local_id: "local-x".into(),
        unit_id: "unit-x".into(),
    }];
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
        UnitIndex {
            id: "unit-x".into(),
            index: 3,
        },
        UnitIndex {
            id: "unit-z".into(),
            index: 4,
        },
    ];

    let unit_index_updates =
        UnitComplex::build_index_updates(&candidate_order, &local_id_maps, current_indexes);

    let ordered_pairs = unit_index_updates
        .iter()
        .map(|unit_index_update| (unit_index_update.id.as_str(), unit_index_update.index))
        .collect::<Vec<_>>();

    assert_eq!(
        ordered_pairs,
        vec![("unit-c", 0), ("unit-x", 2), ("unit-a", 3),]
    );
}

#[test]
fn build_index_updates_supports_order_only_diffs() {
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

    let unit_index_updates = UnitComplex::build_index_updates(
        &["unit-c".into(), "unit-b".into(), "unit-a".into()],
        &[],
        current_indexes,
    );

    let ordered_pairs = unit_index_updates
        .iter()
        .map(|unit_index_update| (unit_index_update.id.as_str(), unit_index_update.index))
        .collect::<Vec<_>>();

    assert_eq!(ordered_pairs, vec![("unit-c", 0), ("unit-a", 2)]);
}
