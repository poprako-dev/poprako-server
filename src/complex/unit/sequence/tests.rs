use crate::complex::unit::sequence as unit_sequence_complex;
use crate::model::read::proj::unit::UnitOrder;
use crate::result::{BaseError, BaseRest};

// Build one persisted Unit link without content fields.
fn order(id: &str, next_id: Option<&str>) -> UnitOrder {
    UnitOrder {
        id: id.to_string(),
        next_id: next_id.map(str::to_string),
        is_hidden: false,
    }
}

// Reconstruct minimal persisted Unit projections through the shared interface.
fn order_test_units(orders: &mut [UnitOrder]) -> BaseRest<()> {
    unit_sequence_complex::order_units(
        orders,
        |order| order.id.as_str(),
        |order| order.next_id.as_deref(),
    )
}

#[test]
fn order_units_orders_a_shuffled_chain() {
    //
    let mut orders = [
        order("c", None),
        order("a", Some("b")),
        order("b", Some("c")),
    ];

    order_test_units(&mut orders).unwrap();

    assert!(
        orders
            == [
                order("a", Some("b")),
                order("b", Some("c")),
                order("c", None)
            ]
    );
}

#[test]
fn order_units_accepts_empty_singleton_and_ordered_chains() {
    //
    let valid_chains = [
        vec![],
        vec![order("a", None)],
        vec![
            order("a", Some("b")),
            order("b", Some("c")),
            order("c", None),
        ],
    ];

    for expected in valid_chains {
        //
        let mut orders = expected.clone();

        order_test_units(&mut orders).unwrap();

        assert!(orders == expected);
    }
}

#[test]
fn order_units_rejects_corrupt_persisted_graphs() {
    //
    let invalid_chains = [
        ("duplicate IDs", vec![order("a", None), order("a", None)]),
        ("missing successor", vec![order("a", Some("missing"))]),
        (
            "competing predecessors",
            vec![
                order("a", Some("c")),
                order("b", Some("c")),
                order("c", None),
            ],
        ),
        ("self-link", vec![order("a", Some("a"))]),
        ("cycle", vec![order("a", Some("b")), order("b", Some("a"))]),
        ("multiple heads", vec![order("a", None), order("b", None)]),
        (
            "disconnected chains",
            vec![
                order("a", Some("b")),
                order("b", None),
                order("c", Some("d")),
                order("d", None),
            ],
        ),
        (
            "unreachable cycle",
            vec![
                order("a", Some("b")),
                order("b", None),
                order("c", Some("d")),
                order("d", Some("c")),
            ],
        ),
    ];

    for (case, mut orders) in invalid_chains {
        //
        let error = order_test_units(&mut orders).unwrap_err();

        assert!(
            matches!(error, BaseError::Unrecoverable { message } if message == "persisted Unit chain is corrupt"),
            "{case}"
        );
    }
}

#[test]
fn order_units_preserves_tombstones_and_payload_at_every_position() {
    //
    for hidden_index in 0..3 {
        //
        let mut expected = [
            (order("a", Some("b")), "head translation", 17),
            (order("b", Some("c")), "middle translation", 23),
            (order("c", None), "tail translation", 41),
        ];

        expected[hidden_index].0.is_hidden = true;

        let mut records = [
            expected[2].clone(),
            expected[0].clone(),
            expected[1].clone(),
        ];

        unit_sequence_complex::order_units(
            &mut records,
            |record| record.0.id.as_str(),
            |record| record.0.next_id.as_deref(),
        )
        .unwrap();

        assert!(records == expected);
    }
}
