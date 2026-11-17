use super::*;
use crate::orders::common::decoders::decode_order;
use crate::orders::{self, Action, OrderOpenClose, OrderType, ShortSaleSlot, TimeInForce, WhatIfType};
use crate::testdata::builders::orders::{auto_open_orders_request, completed_orders_request};
use crate::testdata::builders::RequestEncoder;

#[test]
fn encode_order_basic() {
    let order = Order {
        action: Action::Buy,
        total_quantity: 100.0,
        order_type: Some(OrderType::Limit),
        limit_price: Some(150.0),
        tif: TimeInForce::Day,
        transmit: true,
        ..Default::default()
    };

    let proto = encode_order(&order);

    assert_eq!(proto.action.as_deref(), Some("BUY"));
    assert_eq!(proto.total_quantity.as_deref(), Some("100"));
    assert_eq!(proto.order_type.as_deref(), Some("LMT"));
    assert_eq!(proto.lmt_price, Some(150.0));
    assert_eq!(proto.tif.as_deref(), Some("DAY"));
    assert_eq!(proto.transmit, Some(true));
}

#[test]
fn encode_order_hedge_max_size() {
    let order = Order {
        hedge_max_size: Some(500),
        ..Default::default()
    };

    let proto = encode_order(&order);

    assert_eq!(proto.hedge_max_size, Some(500));
}

#[test]
fn encode_order_deactivate() {
    let order = Order {
        deactivate: true,
        ..Default::default()
    };

    let proto = encode_order(&order);

    assert_eq!(proto.deactivate, Some(true));
}

#[test]
fn encode_order_open_close_fields() {
    let cases = [
        (None, None),
        (Some(OrderOpenClose::Open), Some("O")),
        (Some(OrderOpenClose::Close), Some("C")),
        (Some(OrderOpenClose::Unknown("X".into())), Some("X")),
        (Some(OrderOpenClose::Unknown(String::new())), None),
    ];
    for (value, expected) in cases {
        let order = Order {
            open_close: value.clone(),
            delta_neutral_open_close: value.clone(),
            ..Default::default()
        };

        let proto = encode_order(&order);

        assert_eq!(proto.open_close.as_deref(), expected, "open_close {value:?}");
        assert_eq!(proto.delta_neutral_open_close.as_deref(), expected, "delta_neutral_open_close {value:?}");
    }
}

#[test]
fn encode_order_short_sale_slots() {
    let cases = [
        (ShortSaleSlot::None, None),
        (ShortSaleSlot::Broker, Some(1)),
        (ShortSaleSlot::ThirdParty, Some(2)),
        (ShortSaleSlot::Unknown(7), Some(7)),
    ];
    for (slot, expected) in cases {
        let order = Order {
            short_sale_slot: slot,
            delta_neutral_short_sale_slot: slot,
            ..Default::default()
        };

        let proto = encode_order(&order);

        assert_eq!(proto.short_sale_slot, expected, "short_sale_slot {slot:?}");
        assert_eq!(proto.delta_neutral_short_sale_slot, expected, "delta_neutral_short_sale_slot {slot:?}");
    }
}

/// The tri-state flags: unset is absent, explicit off is 0, on is 1.
#[test]
fn encode_order_tri_state_flags() {
    for (value, expected) in [(None, None), (Some(false), Some(0)), (Some(true), Some(1))] {
        let order = Order {
            route_marketable_to_bbo: value,
            use_price_mgmt_algo: value,
            ..Default::default()
        };

        let proto = encode_order(&order);

        assert_eq!(proto.route_marketable_to_bbo, expected, "route_marketable_to_bbo {value:?}");
        assert_eq!(proto.use_price_mgmt_algo, expected, "use_price_mgmt_algo {value:?}");
    }
}

#[test]
fn encode_order_additional_params() {
    let order = Order {
        post_only: true,
        allow_pre_open: true,
        ignore_open_auction: true,
        seek_price_improvement: Some(true),
        what_if_type: Some(WhatIfType::MarginRequirements),
        ..Default::default()
    };

    let proto = encode_order(&order);

    assert_eq!(proto.post_only, Some(true));
    assert_eq!(proto.allow_pre_open, Some(true));
    assert_eq!(proto.ignore_open_auction, Some(true));
    assert_eq!(proto.seek_price_improvement, Some(1));
    assert_eq!(proto.what_if_type, Some(1));
}

#[test]
fn encode_order_seek_price_improvement_false_is_sent() {
    let order = Order {
        seek_price_improvement: Some(false),
        ..Default::default()
    };

    let proto = encode_order(&order);

    assert_eq!(proto.seek_price_improvement, Some(0));
}

#[test]
fn encode_order_conditions_include_overnight() {
    let order = Order {
        conditions_include_overnight: true,
        ..Default::default()
    };

    let proto = encode_order(&order);

    assert_eq!(proto.conditions_include_overnight, Some(true));
}

#[test]
fn encode_order_default_fields_omitted() {
    let order = Order::default();
    let proto = encode_order(&order);

    assert!(proto.client_id.is_none());
    assert!(proto.parent_id.is_none());
    assert!(proto.block_order.is_none());
    assert!(proto.hidden.is_none());
    assert!(proto.all_or_none.is_none());
    assert!(proto.hedge_max_size.is_none());
    assert!(proto.deactivate.is_none());
    assert!(proto.use_price_mgmt_algo.is_none());
    // `verify_order` gates these on the field: absent when unset, as C# checks presence.
    assert!(proto.route_marketable_to_bbo.is_none());
    assert!(proto.conditions_include_overnight.is_none());
    assert!(proto.post_only.is_none());
    assert!(proto.allow_pre_open.is_none());
    assert!(proto.ignore_open_auction.is_none());
    assert!(proto.seek_price_improvement.is_none());
    assert!(proto.what_if_type.is_none());
}

#[test]
fn encode_soft_dollar_tier_empty() {
    let tier = SoftDollarTier::default();
    assert!(encode_soft_dollar_tier(&tier).is_none());
}

#[test]
fn encode_soft_dollar_tier_filled() {
    let tier = SoftDollarTier {
        name: "Tier1".to_string(),
        value: "Val1".to_string(),
        display_name: "Display1".to_string(),
    };
    let proto = encode_soft_dollar_tier(&tier).unwrap();
    assert_eq!(proto.name.as_deref(), Some("Tier1"));
    assert_eq!(proto.value.as_deref(), Some("Val1"));
    assert_eq!(proto.display_name.as_deref(), Some("Display1"));
}

#[test]
fn encode_condition_price() {
    use crate::orders::conditions::{PriceCondition, TriggerMethod};
    let cond = OrderCondition::Price(PriceCondition {
        contract_id: 265598,
        exchange: "SMART".to_string(),
        price: 150.0,
        trigger_method: TriggerMethod::Last,
        is_more: true,
        is_conjunction: true,
    });

    let proto = encode_condition(&cond);
    assert_eq!(proto.r#type, Some(1));
    assert_eq!(proto.is_conjunction_connection, Some(true));
    assert_eq!(proto.is_more, Some(true));
    assert_eq!(proto.con_id, Some(265598));
    assert_eq!(proto.exchange.as_deref(), Some("SMART"));
    assert_eq!(proto.price, Some(150.0));
    assert_eq!(proto.trigger_method, Some(2)); // Last = 2
}

#[test]
fn encode_condition_price_default_trigger_method_is_emitted() {
    // TWS rejects ("Invalid value in field # 6127") if trigger_method is omitted.
    use crate::orders::conditions::{PriceCondition, TriggerMethod};
    let cond = OrderCondition::Price(PriceCondition {
        contract_id: 265598,
        exchange: "SMART".to_string(),
        price: 150.0,
        trigger_method: TriggerMethod::Default,
        is_more: true,
        is_conjunction: true,
    });

    let proto = encode_condition(&cond);
    assert_eq!(proto.trigger_method, Some(0), "Default trigger_method must be emitted, not omitted");
}

#[test]
fn encode_unknown_condition_round_trips() {
    // #827: a condition decoded as Unknown goes back out as it arrived (conjunction flag set;
    // an absent one returns as explicit AND, see the test below).
    use crate::orders::conditions::UnknownCondition;
    let sparse = proto::OrderCondition {
        r#type: Some(2),
        is_conjunction_connection: Some(true),
        con_id: Some(265598),
        time: Some("20251230 14:30:00 US/Eastern".into()),
        ..Default::default()
    };
    let populated = proto::OrderCondition {
        r#type: Some(99),
        is_conjunction_connection: Some(false),
        is_more: Some(false),
        con_id: Some(265598),
        exchange: Some("SMART".into()),
        symbol: Some("AAPL".into()),
        sec_type: Some("STK".into()),
        percent: Some(12),
        change_percent: Some(2.5),
        price: Some(150.0),
        trigger_method: Some(2),
        time: Some("20251230 14:30:00 US/Eastern".into()),
        volume: Some(1000),
    };
    for wire in [sparse, populated] {
        let order = decode_order(&proto::Order {
            action: Some("BUY".into()),
            conditions: vec![wire.clone()],
            ..Default::default()
        })
        .unwrap();
        assert!(matches!(order.conditions[..], [OrderCondition::Unknown(UnknownCondition { .. })]));
        assert_eq!(encode_condition(&order.conditions[0]), wire);
    }
}

#[test]
fn encode_unknown_condition_absent_conjunction_returns_as_and() {
    // The one field that does not round-trip verbatim: absent reads as AND and is sent explicitly.
    let wire = proto::OrderCondition {
        r#type: Some(2),
        ..Default::default()
    };
    let order = decode_order(&proto::Order {
        action: Some("BUY".into()),
        conditions: vec![wire.clone()],
        ..Default::default()
    })
    .unwrap();
    let encoded = encode_condition(&order.conditions[0]);
    assert_eq!(encoded.is_conjunction_connection, Some(true));
    assert_eq!(
        proto::OrderCondition {
            is_conjunction_connection: None,
            ..encoded
        },
        wire
    );
}

#[test]
fn encode_condition_time() {
    use crate::orders::conditions::TimeCondition;
    let cond = OrderCondition::Time(TimeCondition {
        time: "20251230 14:30:00 US/Eastern".to_string(),
        is_more: false,
        is_conjunction: false,
    });

    let proto = encode_condition(&cond);
    assert_eq!(proto.r#type, Some(3));
    assert_eq!(proto.is_conjunction_connection, Some(false));
    assert_eq!(proto.is_more, Some(false));
    assert_eq!(proto.time.as_deref(), Some("20251230 14:30:00 US/Eastern"));
}

#[test]
fn encode_execution_filter_works() {
    let filter = orders::ExecutionFilter {
        client_id: Some(1),
        account_code: "DU123".to_string(),
        time: Some(time::macros::datetime!(2024-01-01 9:30:15.75 -5)),
        symbol: "AAPL".to_string(),
        security_type: "STK".to_string(),
        exchange: "SMART".to_string(),
        side: Some(orders::ExecutionFilterSide::Buy),
        last_n_days: 5,
        specific_dates: vec![time::macros::date!(2024 - 01 - 01), time::macros::date!(2026 - 11 - 30)],
    };

    let proto = encode_execution_filter(&filter);
    assert_eq!(proto.client_id, Some(1));
    assert_eq!(proto.acct_code.as_deref(), Some("DU123"));
    // Converted to UTC, sub-seconds dropped, `UTC` suffix: the zone form TWS accepts.
    assert_eq!(proto.time.as_deref(), Some("20240101 14:30:15 UTC"));
    assert_eq!(proto.symbol.as_deref(), Some("AAPL"));
    assert_eq!(proto.side.as_deref(), Some("BUY"));
    assert_eq!(proto.last_n_days, Some(5));
    assert_eq!(proto.specific_dates, vec![20240101, 20261130]);
}

#[test]
fn encode_execution_filter_default_sends_no_filters() {
    let proto = encode_execution_filter(&orders::ExecutionFilter::default());
    assert_eq!(proto, proto::ExecutionFilter::default());
}

fn preset_legs(stop_loss: Option<i32>, profit_taker: Option<i32>) -> Order {
    Order {
        preset_stop_loss_order_id: stop_loss,
        preset_profit_taker_order_id: profit_taker,
        ..Default::default()
    }
}

#[test]
fn encode_attached_orders_absent_without_preset_legs() {
    assert_eq!(encode_attached_orders(&Order::default()), None);
}

#[test]
fn encode_attached_orders_sends_preset_type_for_each_leg() {
    let preset = Some("PRESET".to_string());
    let cases = [
        (
            preset_legs(Some(11), Some(12)),
            proto::AttachedOrders {
                sl_order_id: Some(11),
                sl_order_type: preset.clone(),
                pt_order_id: Some(12),
                pt_order_type: preset.clone(),
            },
        ),
        (
            preset_legs(Some(11), None),
            proto::AttachedOrders {
                sl_order_id: Some(11),
                sl_order_type: preset.clone(),
                ..Default::default()
            },
        ),
        (
            preset_legs(None, Some(12)),
            proto::AttachedOrders {
                pt_order_id: Some(12),
                pt_order_type: preset.clone(),
                ..Default::default()
            },
        ),
    ];
    for (order, expected) in cases {
        assert_eq!(encode_attached_orders(&order), Some(expected));
    }
}

#[test]
fn encode_completed_orders_api_only() {
    for (api_only, sent) in [(ApiOnly::Yes, true), (ApiOnly::No, false)] {
        let bytes = encode_completed_orders(api_only).unwrap();
        assert_eq!(bytes, completed_orders_request().api_only(sent).encode_request(), "{api_only:?}");
    }
}

#[test]
fn encode_auto_open_orders_auto_bind() {
    for (auto_bind, sent) in [(AutoBind::Yes, true), (AutoBind::No, false)] {
        let bytes = encode_auto_open_orders(auto_bind).unwrap();
        assert_eq!(bytes, auto_open_orders_request().auto_bind(sent).encode_request(), "{auto_bind:?}");
    }
}
