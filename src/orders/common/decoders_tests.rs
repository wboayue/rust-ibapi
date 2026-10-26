use crate::common::test_utils::helpers::assert_decimal_parse_error;

use super::*;
use crate::common::test_utils::helpers::{assert_missing_field, assert_rejects_text_framing};
use crate::contracts::Symbol;
use crate::messages::IncomingMessages;
use crate::orders::{Action, OrderStatusKind, UnknownCondition};
use crate::testdata::builders::orders::{order_bound, OrderBoundResponse};
use crate::testdata::builders::ResponseProtoEncoder;

#[test]
fn decode_open_order_proto_works() {
    let proto_msg = crate::proto::OpenOrder {
        order_id: Some(42),
        contract: Some(crate::proto::Contract {
            con_id: Some(265598),
            symbol: Some("AAPL".into()),
            sec_type: Some("STK".into()),
            exchange: Some("SMART".into()),
            currency: Some("USD".into()),
            ..Default::default()
        }),
        order: Some(crate::proto::Order {
            order_id: Some(42),
            action: Some("BUY".into()),
            total_quantity: Some("100".into()),
            order_type: Some("LMT".into()),
            lmt_price: Some(150.0),
            ..Default::default()
        }),
        order_state: Some(crate::proto::OrderState {
            status: Some("Submitted".into()),
            ..Default::default()
        }),
    };

    let result = decode_open_order_proto(proto_msg).unwrap();
    assert_eq!(result.order_id, 42);
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.order.order_id, 42);
    assert_eq!(result.order.action, Action::Buy);
    assert_eq!(result.order.total_quantity, 100.0);
    assert_eq!(result.order.order_type, "LMT");
    assert_eq!(result.order.limit_price, Some(150.0));
    assert_eq!(result.order_state.status, OrderStatusKind::Submitted);
}

#[test]
fn decode_order_status_proto_works() {
    let proto_msg = crate::proto::OrderStatus {
        order_id: Some(99),
        status: Some("Filled".into()),
        filled: Some("50".into()),
        remaining: Some("0".into()),
        avg_fill_price: Some(152.5),
        perm_id: Some(123456),
        parent_id: Some(10),
        last_fill_price: Some(152.75),
        client_id: Some(7),
        why_held: Some("locate".into()),
        mkt_cap_price: Some(1.23),
    };

    let result = decode_order_status_proto(proto_msg).unwrap();
    assert_eq!(result.order_id, 99);
    assert_eq!(result.status, OrderStatusKind::Filled);
    assert_eq!(result.filled, 50.0);
    assert_eq!(result.remaining, 0.0);
    assert_eq!(result.average_fill_price, Some(152.5));
    assert_eq!(result.perm_id, 123456);
    assert_eq!(result.parent_id, 10);
    assert_eq!(result.last_fill_price, Some(152.75));
    assert_eq!(result.client_id, 7);
    assert_eq!(result.why_held, "locate");
    assert_eq!(result.market_cap_price, Some(1.23));
}

#[test]
fn decode_order_status_proto_missing_doubles() {
    // Regression: previously decoded to Some(0.0) via unwrap_or_default().
    let proto_msg = crate::proto::OrderStatus {
        order_id: Some(99),
        status: Some("Submitted".into()),
        filled: Some("0".into()),
        remaining: Some("100".into()),
        avg_fill_price: None,
        perm_id: Some(123456),
        parent_id: Some(0),
        last_fill_price: None,
        client_id: Some(7),
        why_held: None,
        mkt_cap_price: None,
    };

    let result = decode_order_status_proto(proto_msg).unwrap();
    assert_eq!(result.average_fill_price, None);
    assert_eq!(result.last_fill_price, None);
    assert_eq!(result.market_cap_price, None);
}

#[test]
fn decode_order_status_proto_rejects_empty_status() {
    // Missing or empty status must error rather than silently defaulting to
    // Submitted; matches the text decoder which fails on empty status fields.
    for status in [None, Some(String::new())] {
        let proto_msg = crate::proto::OrderStatus {
            order_id: Some(99),
            status,
            filled: Some("0".into()),
            remaining: Some("100".into()),
            avg_fill_price: None,
            perm_id: Some(1),
            parent_id: Some(0),
            last_fill_price: None,
            client_id: Some(0),
            why_held: None,
            mkt_cap_price: None,
        };

        assert!(matches!(decode_order_status_proto(proto_msg), Err(crate::Error::Parse(..))));
    }
}

#[test]
fn decode_commission_report_proto_works() {
    let proto_msg = crate::proto::CommissionAndFeesReport {
        exec_id: Some("exec123".into()),
        commission_and_fees: Some(1.25),
        currency: Some("USD".into()),
        realized_pnl: Some(500.0),
        bond_yield: Some(f64::MAX),
        yield_redemption_date: Some("20260101".into()),
    };

    let result = decode_commission_report_proto(proto_msg).unwrap();
    assert_eq!(result.execution_id, "exec123");
    assert_eq!(result.commission, 1.25);
    assert_eq!(result.currency, "USD");
    assert_eq!(result.realized_pnl, Some(500.0));
    assert_eq!(result.yields, None); // f64::MAX filtered out
    assert_eq!(result.yield_redemption_date, "20260101");
}

#[test]
fn decode_execution_data_proto_works() {
    let proto_msg = crate::proto::ExecutionDetails {
        req_id: Some(42),
        contract: Some(crate::proto::Contract {
            con_id: Some(265598),
            symbol: Some("AAPL".into()),
            sec_type: Some("STK".into()),
            ..Default::default()
        }),
        execution: Some(crate::proto::Execution {
            order_id: Some(100),
            exec_id: Some("exec001".into()),
            time: Some("20260101 12:00:00".into()),
            acct_number: Some("DU1234".into()),
            side: Some("BOT".into()),
            shares: Some("50".into()),
            price: Some(152.5),
            perm_id: Some(99999),
            ..Default::default()
        }),
    };

    let result = decode_execution_data_proto(proto_msg).unwrap();
    assert_eq!(result.request_id, 42);
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.execution.execution_id, "exec001");
    assert_eq!(result.execution.shares, 50.0);
    assert_eq!(result.execution.price, 152.5);
    assert_eq!(result.execution.perm_id, 99999);
    assert_eq!(result.execution.side, crate::orders::ExecutionSide::Bought);
}

#[test]
fn decode_completed_order_proto_works() {
    let proto_msg = crate::proto::CompletedOrder {
        contract: Some(crate::proto::Contract {
            con_id: Some(265598),
            symbol: Some("AAPL".into()),
            sec_type: Some("STK".into()),
            ..Default::default()
        }),
        order: Some(crate::proto::Order {
            order_id: Some(200),
            action: Some("SELL".into()),
            total_quantity: Some("200".into()),
            order_type: Some("MKT".into()),
            ..Default::default()
        }),
        order_state: Some(crate::proto::OrderState {
            status: Some("Filled".into()),
            completed_time: Some("20260101 12:00:00".into()),
            completed_status: Some("Filled".into()),
            ..Default::default()
        }),
    };

    let result = decode_completed_order_proto(proto_msg).unwrap();
    // Completed orders always report `order_id = -1` (legacy text-decoder sentinel).
    assert_eq!(result.order_id, -1);
    assert_eq!(result.order.order_id, 200);
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.contract.symbol, Symbol::from("AAPL"));
    assert_eq!(result.order.action, Action::Sell);
    assert_eq!(result.order_state.completed_time, "20260101 12:00:00");
}

// =============================================================================
// Builder → production-decoder integration tests
// =============================================================================

#[test]
fn decode_open_order_proto_via_builder() {
    use crate::testdata::builders::orders::open_order;
    use crate::testdata::builders::ResponseProtoEncoder;

    let p = open_order()
        .order_id(42)
        .contract_id(265598)
        .symbol("AAPL")
        .security_type("STK")
        .order_type("LMT")
        .limit_price(Some(150.0))
        .status(OrderStatusKind::Submitted)
        .to_proto();

    let result = super::decode_open_order_proto(p).unwrap();
    assert_eq!(result.order_id, 42);
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.contract.symbol, Symbol::from("AAPL"));
    assert_eq!(result.order.action, Action::Buy);
    assert_eq!(result.order.order_type, "LMT");
    assert_eq!(result.order.limit_price, Some(150.0));
    assert_eq!(result.order_state.status, OrderStatusKind::Submitted);
}

#[test]
fn decode_completed_order_proto_via_builder() {
    use crate::testdata::builders::orders::completed_order;
    use crate::testdata::builders::ResponseProtoEncoder;

    let p = completed_order()
        .symbol("AAPL")
        .security_type("STK")
        .total_quantity(100.0)
        .completed_time("20260101 12:00:00")
        .completed_status("Filled by Trader")
        .to_proto();

    let result = super::decode_completed_order_proto(p).unwrap();
    assert_eq!(result.contract.symbol, Symbol::from("AAPL"));
    assert_eq!(result.order.total_quantity, 100.0);
    assert_eq!(result.order_state.status, OrderStatusKind::Filled);
    assert_eq!(result.order_state.completed_time, "20260101 12:00:00");
    assert_eq!(result.order_state.completed_status, "Filled by Trader");
}

#[test]
fn decode_order_status_proto_via_builder() {
    use crate::testdata::builders::orders::order_status;
    use crate::testdata::builders::ResponseProtoEncoder;

    let p = order_status()
        .order_id(99)
        .status(OrderStatusKind::Filled)
        .filled(50.0)
        .remaining(0.0)
        .average_fill_price(Some(152.5))
        .perm_id(123456)
        .last_fill_price(Some(152.75))
        .client_id(7)
        .market_cap_price(Some(1.23))
        .to_proto();

    let result = super::decode_order_status_proto(p).unwrap();
    assert_eq!(result.order_id, 99);
    assert_eq!(result.status, OrderStatusKind::Filled);
    assert_eq!(result.filled, 50.0);
    assert_eq!(result.remaining, 0.0);
    assert_eq!(result.average_fill_price, Some(152.5));
    assert_eq!(result.perm_id, 123456);
    assert_eq!(result.last_fill_price, Some(152.75));
    assert_eq!(result.client_id, 7);
    assert_eq!(result.market_cap_price, Some(1.23));
}

#[test]
fn decode_commission_report_proto_via_builder() {
    use crate::testdata::builders::orders::commission_report;
    use crate::testdata::builders::ResponseProtoEncoder;

    let p = commission_report()
        .execution_id("exec123")
        .commission(1.25)
        .currency("USD")
        .realized_pnl(Some(500.0))
        .yields(Some(f64::MAX))
        .to_proto();

    let result = super::decode_commission_report_proto(p).unwrap();
    assert_eq!(result.execution_id, "exec123");
    assert_eq!(result.commission, 1.25);
    assert_eq!(result.currency, "USD");
    assert_eq!(result.realized_pnl, Some(500.0));
    assert_eq!(result.yields, None); // f64::MAX is the IBKR sentinel for "unset"
}

#[test]
fn decode_execution_data_proto_via_builder() {
    use crate::testdata::builders::orders::execution_data;
    use crate::testdata::builders::ResponseProtoEncoder;

    let p = execution_data()
        .request_id(42)
        .order_id(100)
        .contract_id(265598)
        .symbol("AAPL")
        .security_type("STK")
        .execution_id("exec001")
        .side("BOT")
        .shares(50.0)
        .price(152.5)
        .perm_id(99999)
        .to_proto();

    let result = super::decode_execution_data_proto(p).unwrap();
    assert_eq!(result.request_id, 42);
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.execution.execution_id, "exec001");
    assert_eq!(result.execution.shares, 50.0);
    assert_eq!(result.execution.price, 152.5);
    assert_eq!(result.execution.perm_id, 99999);
    assert_eq!(result.execution.side, crate::orders::ExecutionSide::Bought);
}

// =============================================================================
// Text-framing rejection (docs/rules/wire/proto-only-decoding.md)
// =============================================================================
//
// Servers ≥ the connection floor always emit these messages in
// proto framing. Text-framed arrival raises UnexpectedWireFormat — the message
// was addressed to this decoder, so it is not skippable.

#[test]
fn decode_open_order_rejects_text_framing() {
    assert_rejects_text_framing(IncomingMessages::OpenOrder, "5\013\076792991\0AAPL\0STK\0", decode_open_order);
}

#[test]
fn decode_completed_order_rejects_text_framing() {
    assert_rejects_text_framing(IncomingMessages::CompletedOrder, "101\0265598\0AAPL\0STK\0", decode_completed_order);
}

#[test]
fn decode_execution_data_rejects_text_framing() {
    assert_rejects_text_framing(
        IncomingMessages::ExecutionData,
        "11\09000\042\0265598\0AAPL\0STK\0",
        decode_execution_data,
    );
}

#[test]
fn decode_commission_report_rejects_text_framing() {
    assert_rejects_text_framing(
        IncomingMessages::CommissionsReport,
        "59\01\0exec001\02.5\0USD\0",
        decode_commission_report,
    );
}

#[test]
fn decode_order_status_rejects_text_framing() {
    assert_rejects_text_framing(
        IncomingMessages::OrderStatus,
        "3\013\0PreSubmitted\00\0100\00.0\01376327563\00\00.0\0100\0\00.0\0",
        decode_order_status,
    );
}

// === decimal wire fields are routed through parse_optional_decimal (issue #716) ===

#[test]
fn decode_order_status_proto_rejects_malformed_filled() {
    let p = crate::proto::OrderStatus {
        order_id: Some(1),
        status: Some("Submitted".into()),
        filled: Some("abc".into()),
        remaining: Some("0".into()),
        ..Default::default()
    };

    assert_decimal_parse_error(super::decode_order_status_proto(p), "abc");
}

#[test]
fn decode_order_status_proto_rejects_malformed_remaining() {
    let p = crate::proto::OrderStatus {
        order_id: Some(1),
        status: Some("Submitted".into()),
        filled: Some("0".into()),
        remaining: Some("abc".into()),
        ..Default::default()
    };

    assert_decimal_parse_error(super::decode_order_status_proto(p), "abc");
}

// Absent or "unset" filled/remaining: Error::Parse, not a fabricated 0 (#1089).
// A remaining defaulted to 0 made a partial fill read as Filled via
// OrderStatus::outcome(). Live, TWS sends an explicit "0" in every state.
#[test]
fn decode_order_status_proto_requires_filled_and_remaining() {
    let partial = || crate::proto::OrderStatus {
        order_id: Some(1),
        status: Some("Submitted".into()),
        filled: Some("40".into()),
        remaining: Some("60".into()),
        ..Default::default()
    };
    let sentinel = || Some("9223372036854775807".to_string());
    let cases = [
        ("filled absent", "filled", crate::proto::OrderStatus { filled: None, ..partial() }),
        (
            "filled sentinel",
            "filled",
            crate::proto::OrderStatus {
                filled: sentinel(),
                ..partial()
            },
        ),
        (
            "remaining absent",
            "remaining",
            crate::proto::OrderStatus {
                remaining: None,
                ..partial()
            },
        ),
        (
            "remaining sentinel",
            "remaining",
            crate::proto::OrderStatus {
                remaining: sentinel(),
                ..partial()
            },
        ),
    ];
    for (label, field, p) in cases {
        match super::decode_order_status_proto(p) {
            Err(Error::Parse(_, name, reason)) => {
                assert_eq!(name, field, "{label}");
                assert_eq!(reason, "missing in OrderStatus", "{label}");
            }
            other => panic!("{label}: expected Error::Parse, got {other:?}"),
        }
    }

    let filled = super::decode_order_status_proto(crate::proto::OrderStatus {
        filled: Some("100".into()),
        remaining: Some("0".into()),
        ..partial()
    })
    .unwrap();
    assert_eq!((filled.filled, filled.remaining), (100.0, 0.0), "explicit \"0\" remaining decodes");
}

#[test]
fn order_binding_decodes_all_identity_fields() {
    let message = crate::common::test_utils::helpers::proto_response(IncomingMessages::OrderBound, order_bound().client_id(7).encode_proto());
    assert_eq!(
        decode_order_bound(&message).unwrap(),
        crate::orders::OrderBound {
            perm_id: 9_876_543_210,
            client_id: 7,
            order_id: 42
        }
    );
}

#[test]
fn order_binding_requires_complete_identity() {
    for (fixture, missing) in [
        (
            OrderBoundResponse {
                perm_id: None,
                ..order_bound()
            },
            "perm_id",
        ),
        (
            OrderBoundResponse {
                client_id: None,
                ..order_bound()
            },
            "client_id",
        ),
        (
            OrderBoundResponse {
                order_id: None,
                ..order_bound()
            },
            "order_id",
        ),
    ] {
        let message = crate::common::test_utils::helpers::proto_response(IncomingMessages::OrderBound, fixture.encode_proto());
        assert_missing_field(decode_order_bound(&message), missing, "OrderBound");
    }
}

#[test]
fn order_binding_rejects_text_framing() {
    assert_rejects_text_framing(IncomingMessages::OrderBound, "100\0", decode_order_bound);
}

// =============================================================================
// Required submessages (#825 review follow-up)
// =============================================================================

#[test]
fn decode_open_order_proto_rejects_missing_submessages() {
    // #825 made a present-but-empty `action` Error::Parse, but the layer here
    // still defaulted a wholly absent `order` to Order::default() — action ==
    // Buy, the mishandling it removed. The reference client drops such a frame
    // (EDecoder.cs OpenOrderEventProtoBuf returns before eWrapper.openOrder);
    // this crate has no skip channel, so it errors.
    let full = crate::proto::OpenOrder {
        order_id: Some(42),
        contract: Some(crate::proto::Contract {
            sec_type: Some("STK".into()),
            ..Default::default()
        }),
        order: Some(crate::proto::Order {
            action: Some("BUY".into()),
            ..Default::default()
        }),
        order_state: Some(crate::proto::OrderState {
            status: Some("Submitted".into()),
            ..Default::default()
        }),
    };
    assert!(decode_open_order_proto(full.clone()).is_ok(), "control frame must decode");

    for (name, frame) in [
        (
            "contract",
            crate::proto::OpenOrder {
                contract: None,
                ..full.clone()
            },
        ),
        ("order", crate::proto::OpenOrder { order: None, ..full.clone() }),
        (
            "order_state",
            crate::proto::OpenOrder {
                order_state: None,
                ..full.clone()
            },
        ),
    ] {
        assert_missing_field(decode_open_order_proto(frame), name, "OpenOrder");
    }
}

#[test]
fn decode_completed_order_proto_rejects_missing_submessages() {
    let full = crate::proto::CompletedOrder {
        contract: Some(crate::proto::Contract {
            sec_type: Some("STK".into()),
            ..Default::default()
        }),
        order: Some(crate::proto::Order {
            action: Some("BUY".into()),
            ..Default::default()
        }),
        order_state: Some(crate::proto::OrderState {
            status: Some("Submitted".into()),
            ..Default::default()
        }),
    };
    assert!(decode_completed_order_proto(full.clone()).is_ok(), "control frame must decode");

    for (name, frame) in [
        (
            "contract",
            crate::proto::CompletedOrder {
                contract: None,
                ..full.clone()
            },
        ),
        ("order", crate::proto::CompletedOrder { order: None, ..full.clone() }),
        (
            "order_state",
            crate::proto::CompletedOrder {
                order_state: None,
                ..full.clone()
            },
        ),
    ] {
        assert_missing_field(decode_completed_order_proto(frame), name, "CompletedOrder");
    }
}

#[test]
fn decode_execution_data_proto_rejects_missing_submessages() {
    // Same shape as the two above: EDecoder.cs's ExecutionDataEventProtoBuf
    // returns before eWrapper.execDetails(..) when either is null.
    let full = crate::proto::ExecutionDetails {
        req_id: Some(9),
        contract: Some(crate::proto::Contract {
            sec_type: Some("STK".into()),
            ..Default::default()
        }),
        execution: Some(crate::proto::Execution {
            side: Some("BOT".into()),
            ..Default::default()
        }),
    };
    decode_execution_data_proto(full.clone()).expect("control frame must decode");

    for (name, frame) in [
        (
            "contract",
            crate::proto::ExecutionDetails {
                contract: None,
                ..full.clone()
            },
        ),
        (
            "execution",
            crate::proto::ExecutionDetails {
                execution: None,
                ..full.clone()
            },
        ),
    ] {
        assert_missing_field(decode_execution_data_proto(frame), name, "ExecutionDetails");
    }
}

// === decode_order ===

/// The smallest `proto::Order` `decode_order` accepts: `action` is the one
/// required wire field.
fn proto_order() -> proto::Order {
    proto::Order {
        action: Some("BUY".into()),
        ..Default::default()
    }
}

#[test]
fn decode_order_maps_hedge_max_size() {
    let proto_order = proto::Order {
        hedge_max_size: Some(500),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(order.hedge_max_size, Some(500));
}

#[test]
fn decode_order_hedge_max_size_absent_is_none() {
    let order = decode_order(&proto_order()).unwrap();
    assert!(order.hedge_max_size.is_none());
}

#[test]
fn decode_order_keeps_the_up_to_mid_sentinel() {
    // PEG BEST "compete up to the midpoint" is sent as +inf; decoding must not drop it like the
    // f64::MAX unset sentinel.
    let proto_order = proto::Order {
        compete_against_best_offset: Some(f64::INFINITY),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(order.compete_against_best_offset, Some(f64::INFINITY));
}

// === decode_order deactivate ===

#[test]
fn decode_order_maps_deactivate() {
    let proto_order = proto::Order {
        deactivate: Some(true),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert!(order.deactivate);
}

#[test]
fn decode_order_deactivate_absent_is_false() {
    let order = decode_order(&proto_order()).unwrap();
    assert!(!order.deactivate);
}

// === decode_order additional order params ===

#[test]
fn decode_order_maps_additional_params() {
    let proto_order = proto::Order {
        post_only: Some(true),
        allow_pre_open: Some(true),
        ignore_open_auction: Some(true),
        seek_price_improvement: Some(1),
        what_if_type: Some(1),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert!(order.post_only);
    assert!(order.allow_pre_open);
    assert!(order.ignore_open_auction);
    assert_eq!(order.seek_price_improvement, Some(true));
    assert_eq!(order.what_if_type, Some(WhatIfType::MarginRequirements));
}

#[test]
fn decode_order_additional_params_absent_are_unset() {
    let order = decode_order(&proto_order()).unwrap();
    assert!(!order.post_only);
    assert!(!order.allow_pre_open);
    assert!(!order.ignore_open_auction);
    assert_eq!(order.seek_price_improvement, None);
    assert_eq!(order.what_if_type, None);
}

#[test]
fn decode_order_seek_price_improvement_zero_is_false() {
    let proto_order = proto::Order {
        seek_price_improvement: Some(0),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(order.seek_price_improvement, Some(false));
}

// === decode_order conditions_include_overnight ===

#[test]
fn decode_order_maps_conditions_include_overnight() {
    let proto_order = proto::Order {
        conditions_include_overnight: Some(true),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert!(order.conditions_include_overnight);
}

#[test]
fn decode_order_conditions_include_overnight_absent_is_false() {
    let order = decode_order(&proto_order()).unwrap();
    assert!(!order.conditions_include_overnight);
}

// === decode_order tif ===

#[test]
fn decode_order_preserves_unknown_tif() {
    // An unmodeled TIF keeps its raw value instead of decoding as DAY — the
    // bug GoodTillCrossing hit before it was a variant (#822).
    let proto_order = proto::Order {
        tif: Some("GTZ".to_string()),
        ..proto_order()
    };
    assert_eq!(decode_order(&proto_order).unwrap().tif, TimeInForce::Unknown("GTZ".to_string()));
}

#[test]
fn decode_order_maps_known_tif() {
    let proto_order = proto::Order {
        tif: Some("GTX".to_string()),
        ..proto_order()
    };
    assert_eq!(decode_order(&proto_order).unwrap().tif, TimeInForce::GoodTillCrossing);
}

#[test]
fn decode_order_tif_absent_or_empty_is_day() {
    // Upstream omits an empty Tif rather than sending one, so neither form is
    // an unknown value to preserve (unlike action, which is required).
    assert_eq!(decode_order(&proto_order()).unwrap().tif, TimeInForce::Day);

    let empty_tif = proto::Order {
        tif: Some(String::new()),
        ..proto_order()
    };
    assert_eq!(decode_order(&empty_tif).unwrap().tif, TimeInForce::Day);
}

// === decimal wire fields are routed through parse_optional_decimal (issue #716) ===

#[test]
fn decode_order_rejects_malformed_total_quantity() {
    let proto_order = proto::Order {
        total_quantity: Some("abc".into()),
        ..proto_order()
    };
    assert_decimal_parse_error(decode_order(&proto_order), "abc");
}

#[test]
fn decode_order_preserves_fractional_total_quantity() {
    let proto_order = proto::Order {
        total_quantity: Some("0.5".into()),
        ..proto_order()
    };
    assert_eq!(decode_order(&proto_order).unwrap().total_quantity, 0.5);
}

// === decode_order enum fields go through parse_required / parse_optional ===

#[test]
fn decode_order_rejects_missing_or_empty_action() {
    // Unlike tif, action has no unset state upstream: an order without a
    // side is a malformed frame, not a default.
    for proto_order in [
        proto::Order {
            action: None,
            ..proto_order()
        },
        proto::Order {
            action: Some(String::new()),
            ..proto_order()
        },
    ] {
        assert!(
            matches!(decode_order(&proto_order), Err(Error::Parse(_, _, _))),
            "expected Error::Parse for {proto_order:?}"
        );
    }
}

#[test]
fn decode_order_rejects_unknown_action() {
    // Action is closed: an unrecognized side fails the decode rather than
    // reading as Buy.
    let proto_order = proto::Order {
        action: Some("NOTASIDE".into()),
        ..proto_order()
    };
    assert!(matches!(decode_order(&proto_order), Err(Error::Parse(_, _, _))));
}

#[test]
fn decode_order_preserves_unknown_string_codes() {
    // rule80_a and open_close are open: an unrecognized value survives the
    // decode as Unknown(raw) instead of failing the order stream (tif is
    // covered by decode_order_preserves_unknown_tif above).
    let proto_order = proto::Order {
        rule80_a: Some("Z".into()),
        open_close: Some("X".into()),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(order.rule_80_a, Some(Rule80A::Unknown("Z".into())));
    assert_eq!(order.open_close, Some(OrderOpenClose::Unknown("X".into())));
}

#[test]
fn decode_order_empty_optional_strings_are_none() {
    let proto_order = proto::Order {
        rule80_a: Some(String::new()),
        open_close: Some(String::new()),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(order.rule_80_a, None);
    assert_eq!(order.open_close, None);
}

#[test]
fn decode_order_preserves_unknown_integer_codes() {
    let proto_order = proto::Order {
        oca_type: Some(9),
        trigger_method: Some(99),
        volatility_type: Some(9),
        reference_price_type: Some(9),
        origin: Some(9),
        short_sale_slot: Some(9),
        what_if_type: Some(9),
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(order.oca_type, OcaType::Unknown(9));
    assert_eq!(order.trigger_method, TriggerMethod::Unknown(99));
    assert_eq!(order.volatility_type, Some(VolatilityType::Unknown(9)));
    assert_eq!(order.reference_price_type, Some(ReferencePriceType::Unknown(9)));
    assert_eq!(order.origin, OrderOrigin::Unknown(9));
    assert_eq!(order.short_sale_slot, ShortSaleSlot::Unknown(9));
    assert_eq!(order.what_if_type, Some(WhatIfType::Unknown(9)));
}

#[test]
fn decode_order_condition_preserves_unknown_trigger_method() {
    let proto_order = proto::Order {
        conditions: vec![proto::OrderCondition {
            r#type: Some(1),
            trigger_method: Some(99),
            ..Default::default()
        }],
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    match &order.conditions[..] {
        [OrderCondition::Price(price)] => assert_eq!(price.trigger_method, TriggerMethod::Unknown(99)),
        other => panic!("expected one price condition, got {other:?}"),
    }
}

fn populated_condition(condition_type: i32) -> proto::OrderCondition {
    proto::OrderCondition {
        r#type: Some(condition_type),
        is_conjunction_connection: Some(false),
        is_more: Some(true),
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
    }
}

#[test]
fn decode_order_preserves_unknown_condition_type() {
    // #827: an unmodeled type (2 is unassigned) must not read as a zeroed price condition.
    let proto_order = proto::Order {
        conditions: vec![
            populated_condition(2),
            proto::OrderCondition {
                r#type: Some(99),
                ..Default::default()
            },
        ],
        ..proto_order()
    };
    let order = decode_order(&proto_order).unwrap();
    assert_eq!(
        order.conditions,
        vec![
            OrderCondition::Unknown(UnknownCondition {
                condition_type: 2,
                is_conjunction: false,
                is_more: Some(true),
                contract_id: Some(265598),
                exchange: Some("SMART".into()),
                symbol: Some("AAPL".into()),
                security_type: Some("STK".into()),
                percent: Some(12),
                change_percent: Some(2.5),
                price: Some(150.0),
                trigger_method: Some(2),
                time: Some("20251230 14:30:00 US/Eastern".into()),
                volume: Some(1000),
            }),
            OrderCondition::Unknown(UnknownCondition {
                condition_type: 99,
                is_conjunction: true,
                ..Default::default()
            }),
        ]
    );
}

#[test]
fn decode_order_condition_missing_type_is_parse_error() {
    let proto_order = proto::Order {
        conditions: vec![proto::OrderCondition::default()],
        ..proto_order()
    };
    match decode_order(&proto_order).unwrap_err() {
        Error::Parse(_, _, msg) => assert!(msg.contains("OrderCondition type"), "got: {msg}"),
        other => panic!("expected Error::Parse, got {other:?}"),
    }
}

#[test]
fn decode_order_condition_maps_every_type_code() {
    // Probes the same range as check_wire_code_round_trip: a type added to the
    // decoder without a row here fails.
    const MODELED: [i32; 6] = [1, 3, 4, 5, 6, 7];
    for code in -8..=64 {
        let condition = decode_order_condition(&proto::OrderCondition {
            r#type: Some(code),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(condition.condition_type(), code, "code {code}");
        assert_eq!(
            matches!(condition, OrderCondition::Unknown(_)),
            !MODELED.contains(&code),
            "code {code} decoded as {condition:?}"
        );
    }
}

#[test]
fn decode_execution_rejects_malformed_shares() {
    let proto_exec = proto::Execution {
        side: Some("BOT".into()),
        shares: Some("abc".into()),
        ..Default::default()
    };
    assert_decimal_parse_error(decode_execution(&proto_exec), "abc");
}

#[test]
fn decode_execution_rejects_malformed_cumulative_quantity() {
    let proto_exec = proto::Execution {
        side: Some("BOT".into()),
        shares: Some("10".into()),
        cum_qty: Some("abc".into()),
        ..Default::default()
    };
    assert_decimal_parse_error(decode_execution(&proto_exec), "abc");
}

#[test]
fn decode_order_state_sentinel_suggested_size_is_none() {
    // Regression guard: `optional_string_f64`'s unset semantics had to survive
    // the swap to `parse_optional_decimal`.
    let state = proto::OrderState {
        status: Some("Submitted".into()),
        suggested_size: Some("2147483647".into()),
        ..Default::default()
    };
    assert_eq!(decode_order_state(&state).unwrap().suggested_size, None);
}

#[test]
fn decode_order_state_rejects_malformed_allocation_position() {
    let state = proto::OrderState {
        status: Some("Submitted".into()),
        order_allocations: vec![proto::OrderAllocation {
            account: Some("DU1234".into()),
            position: Some("abc".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert_decimal_parse_error(decode_order_state(&state), "abc");
}
