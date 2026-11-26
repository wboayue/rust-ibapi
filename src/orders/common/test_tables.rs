//! Table-driven test data for orders module tests

use crate::common::test_utils::helpers::{proto_error_response, proto_response};
use crate::contracts::{Contract, SecurityType};
use crate::messages::{IncomingMessages, ResponseMessage, ORDER_PRESET_TIF_CODE};
use crate::orders::{OptionExerciseType, Order, OrderLookup, OrderStatusKind, OrderType, PlaceOrder, WhatIfType};
use crate::server_versions;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::orders::{
    completed_order, completed_orders_end as completed_orders_end_response, execution_data, open_order, open_order_end, order_status,
};
use crate::testdata::builders::ResponseProtoEncoder;
use crate::Error;
use std::time::Duration;

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

/// The client id `Client::stubbed` connects with.
const LOOKUP_CLIENT_ID: i32 = 100;

/// What a lookup case expects back.
#[derive(Debug, PartialEq)]
pub(crate) enum LookupExpected {
    /// Working, with the order's open-order record and status.
    Working {
        order_id: i32,
        status: OrderStatusKind,
    },
    /// Finished, with the final status and the fill from the completed record.
    Finished {
        status: OrderStatusKind,
        filled: f64,
        remaining: f64,
        client_id: i32,
        perm_id: i64,
    },
    NotFound,
    /// A reply ended without its end marker.
    UnexpectedEnd,
}

impl LookupExpected {
    /// Assert `result` is this outcome.
    pub(crate) fn check(&self, name: &str, result: Result<OrderLookup, Error>) {
        let got = match result {
            Ok(OrderLookup::Working {
                order: Some(order),
                status: Some(status),
            }) => {
                assert_eq!(order.order.order_id, status.order_id, "{name}: record and status of one order");
                LookupExpected::Working {
                    order_id: order.order.order_id,
                    status: status.status,
                }
            }
            Ok(OrderLookup::Finished { order, status }) => {
                assert_eq!(status, order.completed_order_status(), "{name}: status built from the record");
                LookupExpected::Finished {
                    status: status.status,
                    filled: status.filled,
                    remaining: status.remaining,
                    client_id: status.client_id,
                    perm_id: status.perm_id,
                }
            }
            Ok(OrderLookup::NotFound) => LookupExpected::NotFound,
            Err(Error::UnexpectedEndOfStream) => LookupExpected::UnexpectedEnd,
            other => panic!("{name}: {other:?}"),
        };
        assert_eq!(&got, self, "{name}");
    }
}

/// One `find_order` / `find_order_by_perm_id` call against stubbed replies.
pub(crate) struct LookupCase {
    pub name: &'static str,
    /// The reply to each request, in order: open orders, then completed orders.
    pub replies: Vec<Vec<ResponseMessage>>,
    /// `Some` looks the order up by perm id, `None` by order id 13.
    pub perm_id: Option<i64>,
    pub within: Duration,
    pub expected: LookupExpected,
    /// Requests sent: one open-orders request, then completed-orders requests.
    pub requests: usize,
}

fn open_orders_end() -> ResponseMessage {
    proto_response(IncomingMessages::OpenOrderEnd, open_order_end().encode_proto())
}

fn completed_orders_end() -> ResponseMessage {
    proto_response(IncomingMessages::CompletedOrdersEnd, completed_orders_end_response().encode_proto())
}

/// A completed record of 1000 shares, as TWS sends it: `totalQuantity` is the unfilled part.
fn completed(client_id: i32, order_id: i32, perm_id: i64, status: OrderStatusKind, filled: f64) -> ResponseMessage {
    proto_response(
        IncomingMessages::CompletedOrder,
        completed_order()
            .client_id(client_id)
            .order_id(order_id)
            .perm_id(perm_id)
            .status(status)
            .total_quantity(1000.0 - filled)
            .filled_quantity(filled)
            .encode_proto(),
    )
}

fn working(client_id: i32, order_id: i32, perm_id: i64) -> Vec<ResponseMessage> {
    vec![
        proto_response(
            IncomingMessages::OpenOrder,
            open_order()
                .order_id(order_id)
                .client_id(client_id)
                .perm_id(perm_id)
                .status(OrderStatusKind::Submitted)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::OrderStatus,
            order_status()
                .order_id(order_id)
                .client_id(client_id)
                .perm_id(perm_id)
                .status(OrderStatusKind::Submitted)
                .filled(0.0)
                .remaining(1000.0)
                .encode_proto(),
        ),
        open_orders_end(),
    ]
}

/// Every lookup outcome through the client, shared by the sync and async tests.
pub(crate) fn lookup_cases() -> Vec<LookupCase> {
    use OrderStatusKind::*;
    let finished = |status, filled, remaining| LookupExpected::Finished {
        status,
        filled,
        remaining,
        client_id: LOOKUP_CLIENT_ID,
        perm_id: 501,
    };
    vec![
        LookupCase {
            name: "working: open orders list it, no completed-orders request",
            replies: vec![working(LOOKUP_CLIENT_ID, 13, 501)],
            perm_id: None,
            within: Duration::from_secs(5),
            expected: LookupExpected::Working {
                order_id: 13,
                status: Submitted,
            },
            requests: 1,
        },
        LookupCase {
            name: "finished: filled",
            replies: vec![
                vec![open_orders_end()],
                vec![completed(LOOKUP_CLIENT_ID, 13, 501, Filled, 1000.0), completed_orders_end()],
            ],
            perm_id: None,
            within: Duration::ZERO,
            expected: finished(Filled, 1000.0, 0.0),
            requests: 2,
        },
        LookupCase {
            name: "finished: cancelled after 600 of 1000 filled (wire total 400)",
            replies: vec![
                vec![open_orders_end()],
                vec![completed(LOOKUP_CLIENT_ID, 13, 501, Cancelled, 600.0), completed_orders_end()],
            ],
            perm_id: None,
            within: Duration::ZERO,
            expected: finished(Cancelled, 600.0, 400.0),
            requests: 2,
        },
        LookupCase {
            name: "not found with a zero bound: one pass",
            replies: vec![vec![open_orders_end()], vec![completed_orders_end()]],
            perm_id: None,
            within: Duration::ZERO,
            expected: LookupExpected::NotFound,
            requests: 2,
        },
        LookupCase {
            name: "found on a later re-check within the bound",
            replies: vec![
                vec![open_orders_end()],
                vec![completed_orders_end()],
                vec![completed(LOOKUP_CLIENT_ID, 13, 501, Cancelled, 0.0), completed_orders_end()],
            ],
            perm_id: None,
            within: Duration::from_secs(5),
            expected: finished(Cancelled, 0.0, 1000.0),
            requests: 3,
        },
        LookupCase {
            name: "partial open-orders reply is an error",
            replies: vec![working(LOOKUP_CLIENT_ID, 13, 501)[..2].to_vec()],
            perm_id: None,
            within: Duration::from_secs(5),
            expected: LookupExpected::UnexpectedEnd,
            requests: 1,
        },
        LookupCase {
            name: "partial completed-orders reply is an error",
            replies: vec![vec![open_orders_end()], vec![completed(LOOKUP_CLIENT_ID, 13, 501, Filled, 1000.0)]],
            perm_id: None,
            within: Duration::from_secs(5),
            expected: LookupExpected::UnexpectedEnd,
            requests: 2,
        },
        LookupCase {
            name: "another client's completed order with the same id does not match",
            replies: vec![
                vec![open_orders_end()],
                vec![completed(LOOKUP_CLIENT_ID + 1, 13, 777, Filled, 1000.0), completed_orders_end()],
            ],
            perm_id: None,
            within: Duration::ZERO,
            expected: LookupExpected::NotFound,
            requests: 2,
        },
        LookupCase {
            name: "perm id matches another client's completed order",
            replies: vec![
                vec![open_orders_end()],
                vec![
                    completed(LOOKUP_CLIENT_ID, 13, 501, Filled, 1000.0),
                    completed(LOOKUP_CLIENT_ID + 1, 13, 777, Cancelled, 0.0),
                    completed_orders_end(),
                ],
            ],
            perm_id: Some(777),
            within: Duration::ZERO,
            expected: LookupExpected::Finished {
                status: Cancelled,
                filled: 0.0,
                remaining: 1000.0,
                client_id: LOOKUP_CLIENT_ID + 1,
                perm_id: 777,
            },
            requests: 2,
        },
        LookupCase {
            name: "perm id matches a completed order listed without its ids",
            replies: vec![
                vec![open_orders_end()],
                vec![completed(0, 0, 501, Filled, 1000.0), completed_orders_end()],
            ],
            perm_id: Some(501),
            within: Duration::ZERO,
            expected: LookupExpected::Finished {
                status: Filled,
                filled: 1000.0,
                remaining: 0.0,
                client_id: 0,
                perm_id: 501,
            },
            requests: 2,
        },
        LookupCase {
            name: "perm id matches a working order",
            replies: vec![working(LOOKUP_CLIENT_ID, 13, 501)],
            perm_id: Some(501),
            within: Duration::ZERO,
            expected: LookupExpected::Working {
                order_id: 13,
                status: Submitted,
            },
            requests: 1,
        },
    ]
}

pub(crate) fn lookup_cases_with_order_id_zero() -> [LookupCase; 3] {
    use OrderStatusKind::{Cancelled, Filled, Submitted};
    let completed_rows = vec![
        completed(0, 0, 501, Cancelled, 600.0),
        completed(0, 0, 777, Filled, 1000.0),
        completed_orders_end(),
    ];
    [
        LookupCase {
            name: "zero order id does not match completed orders",
            replies: vec![vec![open_orders_end()], completed_rows.clone()],
            perm_id: None,
            within: Duration::ZERO,
            expected: LookupExpected::NotFound,
            requests: 2,
        },
        LookupCase {
            name: "perm id picks the right completed order without an API id",
            replies: vec![vec![open_orders_end()], completed_rows],
            perm_id: Some(501),
            within: Duration::ZERO,
            expected: LookupExpected::Finished {
                status: Cancelled,
                filled: 600.0,
                remaining: 400.0,
                client_id: 0,
                perm_id: 501,
            },
            requests: 2,
        },
        LookupCase {
            name: "zero order id matches a working order",
            replies: vec![working(0, 0, 501)],
            perm_id: None,
            within: Duration::ZERO,
            expected: LookupExpected::Working {
                order_id: 0,
                status: Submitted,
            },
            requests: 1,
        },
    ]
}
