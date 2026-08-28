use super::*;

fn order(id: &str, next_id: Option<&str>) -> UnitOrder {
    UnitOrder {
        id: id.to_string(),
        next_id: next_id.map(str::to_string),
        is_hidden: false,
    }
}

fn order_test_units(unit_orders: &mut [UnitOrder]) -> BaseRest<()> {
    order_units(
        unit_orders,
        |unit_order| unit_order.id.as_str(),
        |unit_order| unit_order.next_id.as_deref(),
    )
}

#[test]
fn order_units_orders_a_shuffled_chain() {
    //
    let mut unit_orders = vec![
        order("c", None),
        order("a", Some("b")),
        order("b", Some("c")),
    ];

    order_test_units(&mut unit_orders).unwrap();

    assert_eq!(
        unit_orders
            .iter()
            .map(|unit_order| unit_order.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
}

#[test]
fn order_units_rejects_a_forked_chain() {
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

    let error = order_test_units(&mut unit_orders).unwrap_err();

    assert!(matches!(error, BaseError::Unrecoverable { .. }));
}

#[test]
fn order_units_rejects_duplicate_dangling_and_cyclic_chains() {
    //
    let invalid_chains = [
        vec![order("a", None), order("a", None)],
        vec![order("a", Some("missing"))],
        vec![order("a", Some("b")), order("b", Some("a"))],
    ];

    for mut unit_orders in invalid_chains {
        let error = order_test_units(&mut unit_orders).unwrap_err();

        assert!(matches!(error, BaseError::Unrecoverable { .. }));
    }
}
