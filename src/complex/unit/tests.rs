// apply_operations(UnitComplex::apply_operations)(positive): insert, update, move, delete, restore, and counters follow operation order.
// apply_operations(UnitComplex::apply_operations)(positive): missing anchors and self anchors insert at the tail.
// apply_operations(UnitComplex::apply_operations)(negative): empty ids, duplicate local ids, and empty before ids are rejected.

use super::*;

use time::OffsetDateTime;

use crate::model::unit::{UnitInfo, UnitLocalSnapshot, UnitOper, UnitServerSnapshot};
use crate::result::{ExpectedVariant, RootError};

fn server_unit(id: &str, text: &str, proofread_text: Option<&str>, proofread: bool) -> UnitInfo {
    let time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.into(),
        page_id: "page-1".into(),
        index: 0,
        is_bubble: true,
        is_proofread: proofread,
        x_coord: 1.0,
        y_coord: 2.0,
        translated_text: Some(text.into()),
        translator_comment: None,
        last_translator_id: None,
        proofread_text: proofread_text.map(Into::into),
        proofreader_comment: None,
        last_proofreader_id: None,
        created_at: time,
        updated_at: time,
    }
}

fn server_snapshot(id: &str, text: &str) -> UnitServerSnapshot {
    UnitServerSnapshot {
        id: id.into(),
        is_bubble: true,
        is_proofread: false,
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

fn local_snapshot(id: &str, text: &str, proofread: bool) -> UnitLocalSnapshot {
    UnitLocalSnapshot {
        local_id: id.into(),
        is_bubble: true,
        is_proofread: proofread,
        x_coord: 3.0,
        y_coord: 4.0,
        translated_text: Some(text.into()),
        translator_comment: None,
        last_translator_id: None,
        proofread_text: None,
        proofreader_comment: None,
        last_proofreader_id: None,
    }
}

#[test]
fn apply_operations_replays_writes_and_counters() {
    let now = OffsetDateTime::now_utc();
    let current_unit_infos = vec![
        server_unit("unit-a", "alpha", None, false),
        server_unit("unit-b", "", Some("proofread"), true),
        server_unit("unit-c", "gamma", None, false),
    ];
    let unit_operations = vec![
        UnitOper::Delete {
            unit_id: "unit-b".into(),
        },
        UnitOper::Update {
            unit: server_snapshot("unit-b", "restored"),
        },
        UnitOper::MoveBefore {
            unit: server_snapshot("unit-b", "moved"),
            before_id: Some("unit-a".into()),
        },
        UnitOper::InsertBefore {
            unit: local_snapshot("local-x", "inserted", true),
            before_id: Some("unit-c".into()),
        },
        UnitOper::Update {
            unit: server_snapshot("unit-a", ""),
        },
        UnitOper::Delete {
            unit_id: "unit-c".into(),
        },
    ];

    let applied = match UnitComplex::apply_opers("page-1", current_unit_infos, unit_operations, now)
    {
        Ok(applied) => applied,
        Err(_) => panic!("expected operation application"),
    };

    assert_eq!(applied.unit_infos.len(), 3);
    assert_eq!(applied.unit_infos[0].id, "unit-b");
    assert_eq!(applied.unit_infos[1].id, "unit-a");
    assert_eq!(
        applied.unit_infos[2].translated_text.as_deref(),
        Some("inserted")
    );
    assert_eq!(applied.unit_infos[0].index, 0);
    assert_eq!(applied.unit_infos[1].index, 1);
    assert_eq!(applied.unit_infos[2].index, 2);
    assert_eq!(applied.id_mapper.len(), 1);
    assert_eq!(applied.id_mapper[0].local_id, "local-x");
    assert_eq!(applied.counters.total_unit_count, 3);
    assert_eq!(applied.counters.translated_unit_count, 2);
    assert_eq!(applied.counters.proofread_unit_count, 1);
}

#[test]
fn apply_operations_places_missing_and_self_anchor_at_tail() {
    let now = OffsetDateTime::now_utc();
    let current_unit_infos = vec![
        server_unit("unit-a", "alpha", None, false),
        server_unit("unit-b", "beta", None, false),
    ];
    let unit_operations = vec![
        UnitOper::MoveBefore {
            unit: server_snapshot("unit-a", "alpha"),
            before_id: Some("unit-a".into()),
        },
        UnitOper::InsertBefore {
            unit: local_snapshot("local-x", "inserted", false),
            before_id: Some("missing".into()),
        },
    ];

    let applied = match UnitComplex::apply_opers("page-1", current_unit_infos, unit_operations, now)
    {
        Ok(applied) => applied,
        Err(_) => panic!("expected operation application"),
    };

    assert_eq!(applied.unit_infos[0].id, "unit-b");
    assert_eq!(applied.unit_infos[1].id, "unit-a");
    assert_eq!(
        applied.unit_infos[2].translated_text.as_deref(),
        Some("inserted")
    );
}

#[test]
fn apply_operations_rejects_invalid_identifiers() {
    let now = OffsetDateTime::now_utc();
    let err = UnitComplex::apply_opers(
        "page-1",
        Vec::new(),
        vec![UnitOper::InsertBefore {
            unit: local_snapshot("local-x", "one", false),
            before_id: Some(String::new()),
        }],
        now,
    )
    .err()
    .unwrap();

    match err {
        RootError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::Args));
        }
        RootError::Unrecoverable { .. } => {
            panic!("expected argument error");
        }
    }

    let dup_err = UnitComplex::apply_opers(
        "page-1",
        Vec::new(),
        vec![
            UnitOper::InsertBefore {
                unit: local_snapshot("local-x", "one", false),
                before_id: None,
            },
            UnitOper::InsertBefore {
                unit: local_snapshot("local-x", "two", false),
                before_id: None,
            },
        ],
        now,
    )
    .err()
    .unwrap();

    match dup_err {
        RootError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::Args));
        }
        RootError::Unrecoverable { .. } => {
            panic!("expected argument error");
        }
    }
}
