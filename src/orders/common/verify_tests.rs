use super::*;
use crate::client::ids::REQUEST_ID_FLOOR;
use crate::orders::WhatIfType;

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
    let cases = [
        (
            "hedge_max_size",
            Order {
                hedge_max_size: Some(1),
                ..Order::default()
            },
            HEDGE_MAX_SIZE,
        ),
        (
            "deactivate",
            Order {
                deactivate: true,
                ..Order::default()
            },
            DEACTIVATE,
        ),
        (
            "post_only",
            Order {
                post_only: true,
                ..Order::default()
            },
            POST_ONLY,
        ),
        (
            "allow_pre_open",
            Order {
                allow_pre_open: true,
                ..Order::default()
            },
            ALLOW_PRE_OPEN,
        ),
        (
            "ignore_open_auction",
            Order {
                ignore_open_auction: true,
                ..Order::default()
            },
            IGNORE_OPEN_AUCTION,
        ),
        (
            "route_marketable_to_bbo",
            Order {
                route_marketable_to_bbo: true,
                ..Order::default()
            },
            ROUTE_MARKETABLE_TO_BBO,
        ),
        (
            "seek_price_improvement",
            Order {
                seek_price_improvement: Some(false),
                ..Order::default()
            },
            SEEK_PRICE_IMPROVEMENT,
        ),
        (
            "what_if_type",
            Order {
                what_if_type: Some(WhatIfType::FullOrderCheck),
                ..Order::default()
            },
            WHAT_IF_TYPE,
        ),
        (
            "preset_stop_loss_order_id",
            Order {
                preset_stop_loss_order_id: Some(8),
                ..Order::default()
            },
            ATTACHED_ORDERS,
        ),
        (
            "preset_profit_taker_order_id",
            Order {
                preset_profit_taker_order_id: Some(9),
                ..Order::default()
            },
            ATTACHED_ORDERS,
        ),
        (
            "conditions_include_overnight",
            Order {
                conditions_include_overnight: true,
                ..Order::default()
            },
            CONDITIONS_INCLUDE_OVERNIGHT,
        ),
    ];
    for (name, order, feature) in cases {
        assert!(verify_order(feature.min_version, &order).is_ok(), "{name} at its minimum version");
        match verify_order(feature.min_version - 1, &order) {
            Err(Error::ServerVersion(required, actual, message)) => assert_eq!(
                (required, actual, message.as_str()),
                (feature.min_version, feature.min_version - 1, feature.name),
                "{name}"
            ),
            other => panic!("{name} below its minimum version: {other:?}"),
        }
    }
    assert!(verify_order(server_versions::PROTOBUF_REST_MESSAGES_3, &Order::default()).is_ok());
}
