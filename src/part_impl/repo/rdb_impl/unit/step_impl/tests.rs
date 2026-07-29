use super::*;

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

    let error = order_units(
        &mut unit_orders,
        |unit_order| unit_order.id.as_str(),
        |unit_order| unit_order.next_id.as_deref(),
    )
    .unwrap_err();

    assert!(matches!(error, BaseError::Unrecoverable { .. }));
}
