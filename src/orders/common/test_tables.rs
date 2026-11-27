//! Table-driven test data for orders module tests

use crate::common::test_utils::helpers::{proto_error_response, proto_response};
use crate::contracts::{Contract, SecurityType};
use crate::messages::{IncomingMessages, ResponseMessage, ORDER_PRESET_TIF_CODE};
use crate::orders::{OptionExerciseType, Order, OrderStatusKind, OrderType, PlaceOrder, WhatIfType};
use crate::server_versions;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::orders::{execution_data, open_order, order_status};
use crate::testdata::builders::ResponseProtoEncoder;

/// An order field gated above the connection floor: the server version it needs and the
/// feature name its `Error::ServerVersion` carries.
pub(crate) struct ParamGate {
    pub name: &'static str,
    pub version: i32,
    pub feature: &'static str,
    pub set: fn(&mut Order),
}

/// Every field `verify_order` gates above the connection floor, mirroring C#
/// `ValidateOrderParameters`. Versions come from `server_versions`, not `verify.rs`, so the
/// table checks the production gates rather than restating them.
pub(crate) const ORDER_PARAM_GATES: &[ParamGate] = &[
    ParamGate {
        name: "hedge_max_size",
        version: server_versions::HEDGE_MAX_SIZE,
        feature: "hedge max size",
        set: |order| order.hedge_max_size = Some(1),
    },
    ParamGate {
        name: "deactivate",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        feature: "deactivate",
        set: |order| order.deactivate = true,
    },
    ParamGate {
        name: "post_only",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        feature: "post only",
        set: |order| order.post_only = true,
    },
    ParamGate {
        name: "allow_pre_open",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        feature: "allow pre-open",
        set: |order| order.allow_pre_open = true,
    },
    ParamGate {
        name: "ignore_open_auction",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        feature: "ignore open auction",
        set: |order| order.ignore_open_auction = true,
    },
    // Gated on presence: an explicit `false` is sent too.
    ParamGate {
        name: "route_marketable_to_bbo",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_2,
        feature: "route marketable to BBO",
        set: |order| order.route_marketable_to_bbo = Some(false),
    },
    ParamGate {
        name: "seek_price_improvement",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_2,
        feature: "seek price improvement",
        set: |order| order.seek_price_improvement = Some(false),
    },
    ParamGate {
        name: "what_if_type",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_2,
        feature: "what-if type",
        set: |order| order.what_if_type = Some(WhatIfType::FullOrderCheck),
    },
    ParamGate {
        name: "preset_stop_loss_order_id",
        version: server_versions::ATTACHED_ORDERS,
        feature: "attached orders",
        set: |order| order.preset_stop_loss_order_id = Some(8),
    },
    ParamGate {
        name: "preset_profit_taker_order_id",
        version: server_versions::ATTACHED_ORDERS,
        feature: "attached orders",
        set: |order| order.preset_profit_taker_order_id = Some(9),
    },
    ParamGate {
        name: "conditions_include_overnight",
        version: server_versions::CONDITIONS_INCLUDE_OVERNIGHT,
        feature: "conditions include overnight",
        set: |order| order.conditions_include_overnight = true,
    },
];

const EXERCISE_PERM_ID: i64 = 1995911677;
const EXERCISE_STRIKE: f64 = 755.0;

/// The open order TWS echoes for an exercise: a BUY LMT at 0 for one option, no account.
fn exercise_open_order(order_id: i32, status: OrderStatusKind) -> ResponseMessage {
    proto_response(
        IncomingMessages::OpenOrder,
        open_order()
            .order_id(order_id)
            .symbol("SPY")
            .security_type("OPT")
            .last_trade_date_or_contract_month("20261120")
            .strike(EXERCISE_STRIKE)
            .right("C")
            .multiplier("100")
            .exchange("SMART")
            .action("BUY")
            .total_quantity(1.0)
            .order_type("LMT")
            .limit_price(Some(0.0))
            .perm_id(EXERCISE_PERM_ID)
            .status(status)
            .encode_proto(),
    )
}

fn exercise_filled_status(order_id: i32, filled: f64) -> ResponseMessage {
    proto_response(
        IncomingMessages::OrderStatus,
        order_status()
            .order_id(order_id)
            .status(OrderStatusKind::Filled)
            .filled(filled)
            .remaining(0.0)
            .perm_id(EXERCISE_PERM_ID)
            .encode_proto(),
    )
}

/// Every frame one exercise of a SPY 20261120 755 call produced, in live order (paper TWS,
/// server 223; `docs/wire-facts/orders.md`, "`exercise_options` replies").
pub(crate) fn exercise_live_frames(order_id: i32) -> Vec<ResponseMessage> {
    vec![
        proto_error_response(order_id, ORDER_PRESET_TIF_CODE, "Order TIF was set to DAY based on order preset."),
        exercise_open_order(order_id, OrderStatusKind::PreSubmitted),
        proto_response(
            IncomingMessages::ExecutionData,
            execution_data()
                .order_id(order_id)
                .contract_id(756733)
                .symbol("SPY")
                .security_type("STK")
                .execution_id("0000fd27.6af734bb.01.01")
                .side("BOT")
                .shares(100.0)
                .price(EXERCISE_STRIKE)
                .perm_id(EXERCISE_PERM_ID)
                .option_exercise_type(1)
                .encode_proto(),
        ),
        exercise_open_order(order_id, OrderStatusKind::Filled),
        // `filled` is the underlying's 100 shares on a one-contract order.
        exercise_filled_status(order_id, 100.0),
        proto_response(
            IncomingMessages::ExecutionData,
            execution_data()
                .order_id(order_id)
                .contract_id(849150811)
                .symbol("SPY")
                .security_type("OPT")
                .execution_id("0000fd27.6af734bb.01.02")
                .side("SLD")
                .shares(1.0)
                .price(0.0)
                .perm_id(EXERCISE_PERM_ID)
                .option_exercise_type(1)
                .encode_proto(),
        ),
        exercise_open_order(order_id, OrderStatusKind::Filled),
        exercise_filled_status(order_id, 1.0),
    ]
}

/// Asserts the items an `exercise_options` subscription yields for [`exercise_live_frames`],
/// including that the documented stop rule (the execution whose security type is the
/// exercised `contract`'s) picks the execution on the option, not the one on the underlying.
pub(crate) fn assert_exercise_live_items(items: &[SubscriptionItem<PlaceOrder>], contract: &Contract) {
    let kinds: Vec<&str> = items
        .iter()
        .map(|item| match item {
            SubscriptionItem::Notice(_) => "notice",
            SubscriptionItem::Data(PlaceOrder::OpenOrder(_)) => "open order",
            SubscriptionItem::Data(PlaceOrder::OrderStatus(_)) => "status",
            SubscriptionItem::Data(PlaceOrder::ExecutionData(_)) => "execution",
            SubscriptionItem::Data(other) => panic!("unexpected item {other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "notice",
            "open order",
            "execution",
            "open order",
            "status",
            "execution",
            "open order",
            "status"
        ]
    );

    let SubscriptionItem::Notice(notice) = &items[0] else { unreachable!() };
    assert_eq!(notice.code, ORDER_PRESET_TIF_CODE);
    assert!(notice.is_warning(), "10349 should be a warning, got {notice:?}");

    let SubscriptionItem::Data(PlaceOrder::OpenOrder(order)) = &items[1] else {
        unreachable!()
    };
    assert_eq!(order.order.perm_id, EXERCISE_PERM_ID);

    let SubscriptionItem::Data(PlaceOrder::ExecutionData(underlying)) = &items[2] else {
        unreachable!()
    };
    assert_eq!(underlying.contract.security_type, Some(SecurityType::Stock));
    assert_eq!(underlying.execution.shares, 100.0);
    assert_eq!(underlying.execution.price, EXERCISE_STRIKE);
    assert_eq!(underlying.execution.option_exercise_type, OptionExerciseType::Exercise);

    let SubscriptionItem::Data(PlaceOrder::OrderStatus(first_filled)) = &items[4] else {
        unreachable!()
    };
    assert_eq!(first_filled.filled, 100.0, "the first Filled reports the underlying's shares");

    let SubscriptionItem::Data(PlaceOrder::ExecutionData(option)) = &items[5] else {
        unreachable!()
    };
    assert_eq!(option.contract.security_type, Some(SecurityType::Option));
    assert_eq!(option.execution.shares, 1.0);
    assert_eq!(option.execution.option_exercise_type, OptionExerciseType::Exercise);

    let stops: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            matches!(item, SubscriptionItem::Data(PlaceOrder::ExecutionData(data)) if data.contract.security_type == contract.security_type)
        })
        .map(|(i, _)| i)
        .collect();
    assert_eq!(stops, [5], "the stop rule should match only the execution on the option");

    let SubscriptionItem::Data(PlaceOrder::OrderStatus(last)) = &items[7] else {
        unreachable!()
    };
    assert_eq!(last.filled, 1.0);
}

/// Every modeled `OrderType` with its wire code. `all_order_types_covers_every_variant`
/// (`src/orders/tests.rs`) fails to compile when `OrderType` gains a variant, and fails when
/// this table lacks one; the builder tests drive the same list through `build()`.
pub(crate) const ALL_ORDER_TYPES: &[(OrderType, &str)] = &[
    (OrderType::Market, "MKT"),
    (OrderType::Limit, "LMT"),
    (OrderType::Stop, "STP"),
    (OrderType::StopLimit, "STP LMT"),
    (OrderType::TrailingStop, "TRAIL"),
    (OrderType::TrailingStopLimit, "TRAIL LIMIT"),
    (OrderType::MarketOnClose, "MOC"),
    (OrderType::LimitOnClose, "LOC"),
    (OrderType::MarketIfTouched, "MIT"),
    (OrderType::LimitIfTouched, "LIT"),
    (OrderType::MarketWithProtection, "MKT PRT"),
    (OrderType::StopWithProtection, "STP PRT"),
    (OrderType::MarketToLimit, "MTL"),
    (OrderType::Midprice, "MIDPRICE"),
    (OrderType::PeggedToMarket, "PEG MKT"),
    (OrderType::PeggedToStock, "PEG STK"),
    (OrderType::PeggedToMidpoint, "PEG MID"),
    (OrderType::PeggedToBenchmark, "PEG BENCH"),
    (OrderType::PegBest, "PEG BEST"),
    (OrderType::Relative, "REL"),
    (OrderType::PassiveRelative, "PASSV REL"),
    (OrderType::Volatility, "VOL"),
    (OrderType::BoxTop, "BOX TOP"),
    (OrderType::RelativeLimitCombo, "REL + LMT"),
    (OrderType::RelativeMarketCombo, "REL + MKT"),
];
