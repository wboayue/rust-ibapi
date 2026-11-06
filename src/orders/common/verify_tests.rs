use super::*;
use crate::client::ids::REQUEST_ID_FLOOR;
use crate::orders::common::test_tables::ORDER_PARAM_GATES;

fn assert_rejected(result: Result<OrderId, Error>, what: &str) {
    assert!(matches!(result, Err(Error::OrderIdInRequestRange { .. })), "{what}: {result:?}");
}

#[test]
fn verify_order_ids_accepts_order_range() {
    let order = Order {
        parent_id: 7,
        preset_stop_loss_order_id: Some(8),
        preset_profit_taker_order_id: Some(9),
        ..Order::default()
    };
    assert_eq!(
        verify_order_ids(OrderId::from(REQUEST_ID_FLOOR - 1), &order).unwrap(),
        OrderId::from(REQUEST_ID_FLOOR - 1)
    );
    // `parent_id` 0 means no parent.
    assert!(verify_order_ids(OrderId::from(1), &Order::default()).is_ok());
}

/// Every order id an `Order` carries is checked: TWS routes frames for the
/// attached orders by their ids (#789).
#[test]
fn verify_order_ids_rejects_request_range() {
    let floor = REQUEST_ID_FLOOR;
    assert_rejected(verify_order_ids(OrderId::from(floor), &Order::default()), "order id");

    let parent = Order {
        parent_id: floor,
        ..Order::default()
    };
    assert_rejected(verify_order_ids(OrderId::from(1), &parent), "parent_id");

    let stop_loss = Order {
        preset_stop_loss_order_id: Some(floor),
        ..Order::default()
    };
    assert_rejected(verify_order_ids(OrderId::from(1), &stop_loss), "preset_stop_loss_order_id");

    let profit_taker = Order {
        preset_profit_taker_order_id: Some(floor),
        ..Order::default()
    };
    assert_rejected(verify_order_ids(OrderId::from(1), &profit_taker), "preset_profit_taker_order_id");
}

/// Each field above the connection floor passes at its feature's minimum
/// server version and fails one below it with that feature's version and
/// message in the error.
#[test]
fn verify_order_gates_fields_above_the_floor() {
    for gate in ORDER_PARAM_GATES {
        let name = gate.name;
        let mut order = Order::default();
        (gate.set)(&mut order);
        assert!(verify_order(gate.version, &order).is_ok(), "{name} at its minimum version");
        match verify_order(gate.version - 1, &order) {
            Err(Error::ServerVersion(required, actual, message)) => assert_eq!(
                (required, actual, message.as_str()),
                (gate.version, gate.version - 1, gate.feature),
                "{name}"
            ),
            other => panic!("{name} below its minimum version: {other:?}"),
        }
    }
    assert!(verify_order(server_versions::PROTOBUF_REST_MESSAGES_3, &Order::default()).is_ok());
}
