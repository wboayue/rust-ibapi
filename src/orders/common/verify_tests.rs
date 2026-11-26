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

#[test]
fn order_id_after_passes_every_carried_id() {
    let carrying = |parent_id, stop_loss, profit_taker| Order {
        parent_id,
        preset_stop_loss_order_id: stop_loss,
        preset_profit_taker_order_id: profit_taker,
        ..Order::default()
    };
    let cases = [
        ("no carried ids", 7, Order::default(), 8),
        ("presets above the order id", 10, carrying(0, Some(12), Some(11)), 13),
        ("order id above the presets", 20, carrying(0, Some(12), Some(11)), 21),
        ("parent above the order id", 10, carrying(50, None, None), 51),
    ];
    for (label, order_id, order, expected) in cases {
        assert_eq!(order_id_after(OrderId::from(order_id), &order), OrderId::from(expected), "{label}");
    }
}

#[test]
fn verify_and_reserve_order_ids_moves_the_generator_past_the_order() {
    let ids = ClientIdManager::new(1).unwrap();
    let bracket = Order {
        preset_stop_loss_order_id: Some(30),
        preset_profit_taker_order_id: Some(25),
        ..Order::default()
    };

    let checked = verify_and_reserve_order_ids(&ids, server_versions::ATTACHED_ORDERS, OrderId::from(20), &bracket).unwrap();

    assert_eq!(checked, OrderId::from(20));
    assert_eq!(ids.next_order_id(), OrderId::from(31));
}

#[test]
fn verify_and_reserve_order_ids_leaves_the_generator_when_a_check_fails() {
    let ids = ClientIdManager::new(1).unwrap();
    let out_of_range = Order {
        preset_profit_taker_order_id: Some(REQUEST_ID_FLOOR),
        ..Order::default()
    };

    let result = verify_and_reserve_order_ids(&ids, server_versions::ATTACHED_ORDERS, OrderId::from(20), &out_of_range);

    assert_rejected(result, "preset_profit_taker_order_id");
    assert_eq!(ids.next_order_id(), OrderId::from(1), "a rejected order reserves nothing");
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
            Err(Error::ServerVersion {
                required,
                actual,
                feature: message,
            }) => assert_eq!(
                (required, actual, message.as_str()),
                (gate.version, gate.version - 1, gate.feature),
                "{name}"
            ),
            other => panic!("{name} below its minimum version: {other:?}"),
        }
    }
    assert!(verify_order(server_versions::PROTOBUF_REST_MESSAGES_3, &Order::default()).is_ok());
}

/// Values from the 2026-11-14 paper probe (server 226, AAPL): TWS accepted the first
/// set on orders and cancels; the second drew 201 on orders, wedged cancels in
/// PendingCancel, or (non-ASCII) drew 10363.
#[test]
fn verify_ext_operator_matches_probed_set() {
    let alnum = |n: usize| "T1".repeat(n).chars().take(n).collect::<String>();
    let accepted = ["", "T", "TRADER7", "Trader7", "trader7", "12345", &alnum(EXT_OPERATOR_MAX_LEN)];
    for value in accepted {
        assert!(verify_ext_operator("f", value).is_ok(), "{value:?}");
    }
    let rejected = [
        "TRADER_7",
        "TRADER.7",
        "TRADER 7",
        "TRADER/7",
        "TRADER@7",
        "probe-op",
        " TRADER7",
        " ",
        "TRADÉR7",
        &alnum(EXT_OPERATOR_MAX_LEN + 1),
        &alnum(64),
        // TWS accepts 65+, but verify_ext_operator rejects it (see its docs).
        &alnum(65),
    ];
    for value in rejected {
        match verify_ext_operator("Order::ext_operator", value) {
            Err(Error::Validation(ValidationError::InvalidField { field, value: rejected, .. })) => {
                assert_eq!(field, "Order::ext_operator");
                assert_eq!(rejected, value);
            }
            other => panic!("{value:?}: expected InvalidField, got {other:?}"),
        }
    }
}

#[test]
fn verify_order_and_cancel_ext_operator() {
    let order = Order {
        ext_operator: "probe-op".to_owned(),
        ..Order::default()
    };
    assert!(matches!(
        verify_order_ext_operator(&order),
        Err(Error::Validation(ValidationError::InvalidField { .. }))
    ));
    assert!(verify_order_ext_operator(&Order::default()).is_ok());
    let cancel = OrderCancel {
        ext_operator: "probe-cxl".to_owned(),
        ..OrderCancel::default()
    };
    assert!(matches!(
        verify_order_cancel(&cancel),
        Err(Error::Validation(ValidationError::InvalidField { .. }))
    ));
    assert!(verify_order_cancel(&OrderCancel::default()).is_ok());
}
