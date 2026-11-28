use crate::orders::common::test_tables::{assert_linked_order_requests, linked_orders, LINKED_ORDER_IDS};
use std::sync::Arc;

use crate::common::test_utils::helpers::{
    assert_request, assert_tws_error_message, create_blocking_test_client, create_blocking_test_client_with_ordered_proto_responses,
    decode_request_proto, order_status_response, proto_error_response, proto_response, request_message_count,
};
use crate::contracts::{ComboLeg, Contract, Currency, Exchange, LegAction, OptionRight, SecurityIdType, SecurityType, Symbol};
use crate::messages::IncomingMessages;
use crate::orders::{Action, ExecutionFilterSide, ExecutionSide, OcaType, Order, OrderCondition, OrderStatusKind, OrderType, TimeInForce};
use crate::proto;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::orders::{
    all_open_orders_request, auto_open_orders_request, cancel_order_request, commission_and_fees_report, completed_order, completed_orders_end,
    completed_orders_request, execution_data, execution_data_end, executions_request, global_cancel_request, next_valid_id,
    next_valid_order_id_request, open_order, open_order_end, open_orders_request, order_bound, order_status, place_order_request,
};
use crate::testdata::builders::ResponseProtoEncoder;

use super::*;
use crate::builder::ValidationError;
use crate::client::ids::{OrderId, REQUEST_ID_FLOOR};
use crate::orders::builder::{BracketPrices, TrailBy};
use crate::orders::common::order_builder;

#[test]
fn place_order() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::OpenOrder,
            open_order().status(OrderStatusKind::PreSubmitted).encode_proto(),
        ),
        proto_response(
            IncomingMessages::OrderStatus,
            order_status().status(OrderStatusKind::PreSubmitted).remaining(100.0).encode_proto(),
        ),
        proto_response(IncomingMessages::ExecutionData, execution_data().encode_proto()),
        proto_response(IncomingMessages::OpenOrder, open_order().status(OrderStatusKind::Filled).encode_proto()),
        proto_response(
            IncomingMessages::OrderStatus,
            order_status()
                .status(OrderStatusKind::Filled)
                .filled(100.0)
                .remaining(0.0)
                .average_fill_price(Some(196.52))
                .last_fill_price(Some(196.52))
                .encode_proto(),
        ),
        proto_response(IncomingMessages::OpenOrder, open_order().status(OrderStatusKind::Filled).encode_proto()),
        proto_response(IncomingMessages::CommissionsAndFeesReport, commission_and_fees_report().encode_proto()),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract {
        symbol: Symbol::from("TSLA"),
        security_type: Some(SecurityType::Stock),
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        ..Contract::default()
    };

    let order_id = 13;
    let order = order_builder::market_order(Action::Buy, 100.0);

    let result = client.place_order(order_id, &contract, &order);

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &place_order_request().order_id(order_id).contract(&contract).order(&order),
    );

    assert!(result.is_ok(), "failed to place order: {}", result.err().unwrap());

    let notifications = result.unwrap();

    if let Some(Ok(PlaceOrder::OpenOrder(open_order))) = notifications.next_data() {
        assert_eq!(open_order.order_id, 13, "open_order.order_id");
        assert_eq!(open_order.order_state.status, OrderStatusKind::PreSubmitted, "order_state.status");
    } else {
        assert!(false, "message[0] expected an open order notification");
    }

    if let Some(Ok(PlaceOrder::OrderStatus(order_status))) = notifications.next_data() {
        assert_eq!(order_status.order_id, 13, "order_status.order_id");
        assert_eq!(order_status.status, OrderStatusKind::PreSubmitted, "order_status.status");
        assert_eq!(order_status.filled, 0.0, "order_status.filled");
        assert_eq!(order_status.remaining, 100.0, "order_status.remaining");
        assert_eq!(order_status.average_fill_price, Some(0.0), "order_status.average_fill_price");
        assert_eq!(order_status.perm_id, 1376327563, "order_status.perm_id");
        assert_eq!(order_status.parent_id, 0, "order_status.parent_id");
        assert_eq!(order_status.last_fill_price, Some(0.0), "order_status.last_fill_price");
        assert_eq!(order_status.client_id, 100, "order_status.client_id");
        assert_eq!(order_status.why_held, "", "order_status.why_held");
        assert_eq!(order_status.market_cap_price, Some(0.0), "order_status.market_cap_price");
    } else {
        assert!(false, "message[1] expected order status notification");
    }

    if let Some(Ok(PlaceOrder::ExecutionData(exec_data))) = notifications.next_data() {
        assert_eq!(exec_data.execution.order_id, 13, "execution.order_id");
        assert_eq!(exec_data.execution.shares, 100.0, "execution.shares");
        assert_eq!(exec_data.execution.price, 196.52, "execution.price");
        assert_eq!(exec_data.contract.symbol, Symbol::from("TSLA"), "contract.symbol");
    } else {
        assert!(false, "message[2] expected execution notification");
    }

    assert!(
        matches!(notifications.next_data(), Some(Ok(PlaceOrder::OpenOrder(_)))),
        "message[3] expected an open order notification"
    );

    if let Some(Ok(PlaceOrder::OrderStatus(order_status))) = notifications.next_data() {
        assert_eq!(order_status.status, OrderStatusKind::Filled, "order_status.status");
        assert_eq!(order_status.filled, 100.0, "order_status.filled");
        assert_eq!(order_status.remaining, 0.0, "order_status.remaining");
    } else {
        assert!(false, "message[4] expected order status notification");
    }

    assert!(
        matches!(notifications.next_data(), Some(Ok(PlaceOrder::OpenOrder(_)))),
        "message[5] expected an open order notification"
    );

    if let Some(Ok(PlaceOrder::CommissionAndFeesReport(report))) = notifications.next_data() {
        assert_eq!(report.execution_id, "00025b46.63f8f39c.01.01", "report.execution_id");
        assert_eq!(report.commission_and_fees, 1.0, "report.commission_and_fees");
        assert_eq!(report.currency, "USD", "report.currency");
    } else {
        assert!(false, "message[6] expected a commission report notification");
    }
}

// Drives the real place_order path and decodes the captured wire bytes to confirm
// hedge_max_size rides the outbound PlaceOrderRequest proto (docs/rules/testing/exercise-production-code.md).
#[test]
fn place_order_encodes_hedge_max_size() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::HEDGE_MAX_SIZE);

    let contract = Contract::stock("TSLA").build();
    let mut order = order_builder::market_order(Action::Buy, 100.0);
    order.hedge_max_size = Some(500);

    let _subscription = client.place_order(20, &contract, &order).expect("place_order should succeed");

    assert_eq!(request_message_count(&message_bus), 1);
    let request: crate::proto::PlaceOrderRequest = decode_request_proto(&message_bus, 0);
    let proto_order = request.order.expect("request carries an order");
    assert_eq!(proto_order.hedge_max_size, Some(500));
}

// Each flag is rejected below its gate before anything is sent, and sent at it.
#[test]
fn place_order_gates_order_params() {
    use crate::orders::common::test_tables::ORDER_PARAM_GATES;

    for gate in ORDER_PARAM_GATES {
        let name = gate.name;
        let contract = Contract::stock("TSLA").build();
        let mut order = order_builder::market_order(Action::Buy, 100.0);
        (gate.set)(&mut order);

        let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
        let client = Client::stubbed(message_bus.clone(), gate.version - 1);
        match client.place_order(20, &contract, &order) {
            Err(Error::ServerVersion { required, .. }) => assert_eq!(required, gate.version, "{name}"),
            Err(other) => panic!("{name}: expected ServerVersion error, got {other:?}"),
            Ok(_) => panic!("{name}: expected place_order to be rejected below the gate"),
        }
        assert_eq!(request_message_count(&message_bus), 0, "{name}: sent below the gate");

        let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
        let client = Client::stubbed(message_bus.clone(), gate.version);
        let _subscription = client
            .place_order(20, &contract, &order)
            .unwrap_or_else(|e| panic!("{name}: rejected at the gate: {e:?}"));
        assert_eq!(request_message_count(&message_bus), 1, "{name}: not sent at the gate");
    }
}

#[test]
fn cancel_order() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::OrderStatus,
        order_status()
            .order_id(41)
            .status(OrderStatusKind::Cancelled)
            .remaining(100.0)
            .perm_id(71270927)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let order_id = 41;
    let results = client.cancel_order(order_id);

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &cancel_order_request().order_id(order_id));

    assert!(results.is_ok(), "failed to cancel order: {}", results.err().unwrap());

    let results = results.unwrap();

    if let Some(Ok(CancelOrder::OrderStatus(order_status))) = results.next_data() {
        assert_eq!(order_status.order_id, 41, "order_status.order_id");
        assert_eq!(order_status.status, OrderStatusKind::Cancelled, "order_status.status");
        assert_eq!(order_status.filled, 0.0, "order_status.filled");
        assert_eq!(order_status.remaining, 100.0, "order_status.remaining");
        assert_eq!(order_status.average_fill_price, Some(0.0), "order_status.average_fill_price");
        assert_eq!(order_status.perm_id, 71270927, "order_status.perm_id");
        assert_eq!(order_status.parent_id, 0, "order_status.parent_id");
        assert_eq!(order_status.last_fill_price, Some(0.0), "order_status.last_fill_price");
        assert_eq!(order_status.client_id, 100, "order_status.client_id");
        assert_eq!(order_status.why_held, "", "order_status.why_held");
        assert_eq!(order_status.market_cap_price, Some(0.0), "order_status.market_cap_price");
    }
}

#[test]
fn cancel_order_delivers_202_as_non_terminal_notice() {
    // The cancellation confirmation (202) must arrive as a Notice and leave
    // the stream open: the OrderStatus frame queued behind it still arrives.
    let order_id = 41;
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_error_response(order_id, crate::messages::ORDER_CANCELLED_CODE, "Order Canceled - reason:"),
        proto_response(
            IncomingMessages::OrderStatus,
            order_status().order_id(order_id).status(OrderStatusKind::Cancelled).encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let subscription = client.cancel_order(order_id).expect("cancel_order failed");

    match subscription.next() {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert!(notice.is_cancellation(), "expected cancellation notice, got {notice:?}");
        }
        other => panic!("expected non-terminal cancellation Notice, got {other:?}"),
    }

    match subscription.next() {
        Some(Ok(SubscriptionItem::Data(CancelOrder::OrderStatus(status)))) => {
            assert_eq!(status.status, OrderStatusKind::Cancelled);
        }
        other => panic!("expected OrderStatus after the 202 notice, got {other:?}"),
    }
}

#[test]
fn global_cancel() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let results = client.global_cancel();

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &global_cancel_request());
    assert!(results.is_ok(), "failed to cancel order: {}", results.err().unwrap());
}

fn tagged_cancel() -> crate::orders::OrderCancel {
    crate::orders::OrderCancel {
        manual_order_cancel_time: "20261110-14:30:00".to_owned(),
        ext_operator: "TRADER7".to_owned(),
        manual_order_indicator: Some(1),
    }
}

#[test]
fn cancel_order_with_sends_operator_fields() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let _subscription = client.cancel_order_with(41, &tagged_cancel()).expect("cancel_order_with failed");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &cancel_order_request().order_id(41).order_cancel(tagged_cancel()));
}

#[test]
fn cancel_order_with_rejects_request_range_id() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let result = client.cancel_order_with(crate::client::ids::REQUEST_ID_FLOOR, &tagged_cancel());

    assert!(matches!(result, Err(Error::OrderIdInRequestRange { .. })), "got {:?}", result.err());
    assert_eq!(request_message_count(&message_bus), 0);
}

// A bad operator ID fails before anything is sent: on a cancel, TWS would never
// reply and the order would stick in PendingCancel (#1206).
#[test]
fn invalid_ext_operator_is_rejected_before_sending() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    let cancel = crate::orders::OrderCancel {
        ext_operator: "TRADER_7".to_owned(),
        ..Default::default()
    };
    let contract = Contract::stock("AAPL").build();
    let mut order = order_builder::limit_order(Action::Buy, 1.0, 1.0);
    order.ext_operator = "TRADER_7".to_owned();

    let rejected = |result: Result<(), Error>, what: &str| {
        assert!(
            matches!(result, Err(Error::Validation(ValidationError::InvalidField { .. }))),
            "{what}: {result:?}"
        )
    };
    rejected(client.cancel_order_with(41, &cancel).map(drop), "cancel_order_with");
    rejected(client.global_cancel_with(&cancel), "global_cancel_with");
    rejected(client.place_order(41, &contract, &order).map(drop), "place_order");
    rejected(client.submit_order(41, &contract, &order), "submit_order");
    assert_eq!(request_message_count(&message_bus), 0);
}

#[test]
fn global_cancel_with_sends_operator_fields() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    client.global_cancel_with(&tagged_cancel()).expect("global_cancel_with failed");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &global_cancel_request().order_cancel(tagged_cancel()));
}

#[test]
fn next_valid_order_id() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::NextValidId,
        next_valid_id().order_id(43).encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let results = client.next_valid_order_id();

    assert_request(&message_bus, 0, &next_valid_order_id_request());

    assert!(results.is_ok(), "failed to request next order id: {}", results.err().unwrap());
    assert_eq!(43, results.unwrap(), "next order id");
}

// The server only knows about IDs it has seen: an ID allocated locally but not
// yet transmitted is invisible to it, so its answer can sit at or below the
// local counter. That answer must not rewind the generator.
#[test]
fn next_valid_order_id_below_allocated_mark_does_not_rewind() {
    let (client, _bus) = create_blocking_test_client_with_ordered_proto_responses(vec![proto_response(
        IncomingMessages::NextValidId,
        next_valid_id().order_id(5).encode_proto(),
    )]);

    let allocated = client.next_order_id();
    let server_value = client.next_valid_order_id().expect("next_valid_order_id");

    assert_eq!(server_value, 5, "the server's value is still returned verbatim");
    assert_eq!(client.next_order_id(), allocated + 1, "generator must not rewind below an allocated ID");
}

#[test]
fn completed_orders() {
    let _ = env_logger::try_init();

    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::CompletedOrder,
            completed_order().trail_stop_price(Some(150.25)).encode_proto(),
        ),
        proto_response(IncomingMessages::CompletedOrdersEnd, completed_orders_end().encode_proto()),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let results = client.completed_orders(ApiOnly::Yes);

    assert_request(&message_bus, 0, &completed_orders_request().api_only(true));

    assert!(results.is_ok(), "failed to request completed orders: {}", results.err().unwrap());

    let results = results.unwrap();
    if let Some(Ok(Orders::OrderData(order_data))) = results.next_data() {
        assert_eq!(order_data.contract.symbol, Symbol::from("AAPL"), "contract.symbol");
        assert_eq!(order_data.contract.security_type, Some(SecurityType::Stock), "contract.security_type");
        assert_eq!(order_data.order.action, Action::Buy, "order.action");
        assert_eq!(order_data.order.total_quantity, 0.0, "order.total_quantity");
        assert_eq!(order_data.order.trail_stop_price, Some(150.25), "order.trail_stop_price");
        assert_eq!(
            order_data.order.shareholder, "Not an insider or substantial shareholder",
            "order.shareholder"
        );
        assert_eq!(order_data.order_state.status, OrderStatusKind::Filled, "order_state.status");
        assert_eq!(
            order_data.order_state.completed_time, "20231122 10:30:00 America/Los_Angeles",
            "order_state.completed_time"
        );
        assert_eq!(order_data.order_state.completed_status, "Filled", "order_state.completed_status");
    } else {
        assert!(false, "expected order data");
    }
}

#[test]
fn open_orders() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::OpenOrderEnd,
        open_order_end().encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let results = client.open_orders();

    assert_request(&message_bus, 0, &open_orders_request());
    let subscription = results.expect("open_orders");
    assert!(subscription.next().is_none(), "OpenOrderEnd alone must end the stream with no orders");
}

#[test]
fn all_open_orders() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::OpenOrderEnd,
        open_order_end().encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let results = client.all_open_orders();

    assert_request(&message_bus, 0, &all_open_orders_request());
    let subscription = results.expect("all_open_orders");
    assert!(subscription.next().is_none(), "OpenOrderEnd alone must end the stream with no orders");
}

#[test]
fn auto_open_orders() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::OpenOrderEnd,
        open_order_end().encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let results = client.auto_open_orders(AutoBind::Yes);

    assert_request(&message_bus, 0, &auto_open_orders_request().auto_bind(true));
    let subscription = results.expect("auto_open_orders");
    // OpenOrderEnd is terminal for auto_open_orders too: the stream ends rather than waiting for binds.
    assert!(subscription.next().is_none(), "OpenOrderEnd alone must end the stream with no orders");
}

#[test]
fn executions() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::ExecutionDataEnd,
        execution_data_end().encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let filter = ExecutionFilter {
        client_id: Some(100),
        account_code: "xyz".to_owned(),
        time: Some(time::macros::datetime!(2026-11-15 10:52 -5)),
        symbol: "TSLA".to_owned(),
        security_type: "STK".to_owned(),
        exchange: "ISLAND".to_owned(),
        side: Some(ExecutionFilterSide::Buy),
        ..Default::default()
    };
    let expected_filter = ExecutionFilter {
        client_id: Some(100),
        account_code: "xyz".to_owned(),
        time: Some(time::macros::datetime!(2026-11-15 10:52 -5)),
        symbol: "TSLA".to_owned(),
        security_type: "STK".to_owned(),
        exchange: "ISLAND".to_owned(),
        side: Some(ExecutionFilterSide::Buy),
        ..Default::default()
    };
    let results = client.executions(filter);

    assert_request(
        &message_bus,
        0,
        &executions_request()
            .request_id(crate::common::test_utils::helpers::TEST_REQ_ID_FIRST)
            .filter(expected_filter),
    );

    let subscription = results.expect("executions");
    assert!(
        subscription.next().is_none(),
        "ExecutionDataEnd alone must end the stream with no executions"
    );
}

#[test]
fn encode_limit_order() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let order_id = 12;
    let contract = Contract {
        security_type: Some(SecurityType::Future),
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        local_symbol: "FGBL MAR 23".to_owned(),
        last_trade_date_or_contract_month: Some("202303".parse().unwrap()),
        ..Contract::default()
    };
    let order = order_builder::limit_order(Action::Buy, 10.0, 500.00);

    let results = client.place_order(order_id, &contract, &order);

    assert_request(
        &message_bus,
        0,
        &place_order_request().order_id(order_id).contract(&contract).order(&order),
    );

    assert!(results.is_ok(), "failed to place order: {}", results.err().unwrap());
}

#[test]
fn encode_combo_market_order() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let order_id = 12; // get next order id
    let contract = {
        let leg_1 = ComboLeg {
            contract_id: 55928698, //WTI future June 2017
            ratio: 1,
            action: LegAction::Buy,
            exchange: "IPE".to_owned(),
            ..ComboLeg::default()
        };

        let leg_2 = ComboLeg {
            contract_id: 55850663, //COIL future June 2017
            ratio: 1,
            action: LegAction::Sell,
            exchange: "IPE".to_owned(),
            ..ComboLeg::default()
        };

        Contract {
            symbol: Symbol::from("WTI"), // WTI,COIL spread. Symbol can be defined as first leg symbol ("WTI") or currency ("USD").
            security_type: Some(SecurityType::Spread),
            currency: Currency::from("USD"),
            exchange: Exchange::from("SMART"),
            combo_legs: vec![leg_1, leg_2],
            ..Contract::default()
        }
    };
    let order = order_builder::non_guaranteed(order_builder::combo_market_order(Action::Sell, 150.0));

    let results = client.place_order(order_id, &contract, &order);

    assert_request(
        &message_bus,
        0,
        &place_order_request().order_id(order_id).contract(&contract).order(&order),
    );

    assert!(results.is_ok(), "failed to place order: {}", results.err().unwrap());
}

#[test]
fn exercise_options() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::OpenOrder,
        open_order()
            .symbol("ES")
            .security_type("FOP")
            .last_trade_date_or_contract_month("20250919")
            .strike(5800.0)
            .right("C")
            .multiplier("50")
            .exchange("CME")
            .local_symbol("ESU5C5800")
            .trading_class("ES")
            .total_quantity(1.0)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract {
        symbol: Symbol::from("ES"),
        security_type: Some(SecurityType::FuturesOption),
        exchange: Exchange::from("CME"),
        currency: Currency::from("USD"),
        last_trade_date_or_contract_month: Some("20250919".parse().unwrap()),
        strike: 5800.0,
        right: Some(OptionRight::Call),
        ..Default::default()
    };

    let subscription = client
        .exercise_options(&contract)
        .exercise(1)
        .submit()
        .expect("failed to exercise options");

    let exercise_response = subscription.next_data();
    assert!(
        matches!(exercise_response, Some(Ok(PlaceOrder::OpenOrder(_)))),
        "Expected PlaceOrder::OpenOrder, got {:?}",
        exercise_response
    );

    assert_eq!(request_message_count(&message_bus), 1);
}

// Replays one live exercise (paper TWS, server 223): 10349 must not end the stream, and every
// open order, status and execution after it arrives.
#[test]
fn exercise_options_reads_past_preset_tif_warning_to_executions() {
    use crate::orders::common::test_tables::{assert_exercise_live_items, exercise_live_frames};

    let order_id = 16;
    let frames = exercise_live_frames(order_id);
    let frame_count = frames.len();
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(frames));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::option(
        "SPY",
        crate::contracts::ExpirationDate::new(2026, 11, 20).unwrap(),
        755.0,
        OptionRight::Call,
    );

    let subscription = client.exercise_options(&contract).exercise(1).submit().expect("exercise_options failed");

    let items: Vec<_> = (0..frame_count)
        .map(|_| subscription.next().expect("stream ended early").expect("exercise item failed"))
        .collect();
    assert_exercise_live_items(&items, &contract);
}

#[test]
fn submit_order() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract {
        symbol: Symbol::from("AAPL"),
        security_type: Some(SecurityType::Stock),
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        ..Contract::default()
    };

    let order_id = 42;
    let order = order_builder::market_order(Action::Buy, 200.0);

    let result = client.submit_order(order_id, &contract, &order);

    assert_request(
        &message_bus,
        0,
        &place_order_request().order_id(order_id).contract(&contract).order(&order),
    );

    assert!(result.is_ok(), "failed to submit order: {}", result.err().unwrap());
}

#[test]
fn order_update_stream() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::OpenOrder,
            open_order().status(OrderStatusKind::PreSubmitted).encode_proto(),
        ),
        proto_response(
            IncomingMessages::OrderStatus,
            order_status().status(OrderStatusKind::PreSubmitted).remaining(100.0).encode_proto(),
        ),
        proto_response(IncomingMessages::ExecutionData, execution_data().option_exercise_type(102).encode_proto()),
        proto_response(IncomingMessages::CommissionsAndFeesReport, commission_and_fees_report().encode_proto()),
    ]));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let stream = client.order_update_stream();
    assert!(stream.is_ok(), "failed to create order update stream: {}", stream.err().unwrap());

    let notifications = stream.unwrap();

    if let Some(Ok(OrderUpdate::OpenOrder(open_order))) = notifications.next_data() {
        assert_eq!(open_order.order_id, 13, "open_order.order_id");
        assert_eq!(open_order.contract.symbol, Symbol::from("TSLA"), "contract.symbol");
        assert_eq!(open_order.order.action, Action::Buy, "order.action");
        assert_eq!(open_order.order.total_quantity, 100.0, "order.total_quantity");
        assert_eq!(open_order.order_state.status, OrderStatusKind::PreSubmitted, "order_state.status");
    } else {
        assert!(false, "expected open order notification");
    }

    if let Some(Ok(OrderUpdate::OrderStatus(status))) = notifications.next_data() {
        assert_eq!(status.order_id, 13, "order_status.order_id");
        assert_eq!(status.status, OrderStatusKind::PreSubmitted, "order_status.status");
        assert_eq!(status.filled, 0.0, "order_status.filled");
        assert_eq!(status.remaining, 100.0, "order_status.remaining");
    } else {
        assert!(false, "expected order status notification");
    }

    if let Some(Ok(OrderUpdate::ExecutionData(exec_data))) = notifications.next_data() {
        assert_eq!(exec_data.execution.order_id, 13, "execution.order_id");
        assert_eq!(exec_data.execution.shares, 100.0, "execution.shares");
        assert_eq!(exec_data.execution.price, 196.52, "execution.price");
        assert_eq!(exec_data.execution.side, ExecutionSide::Bought, "execution.side");
        assert_eq!(
            exec_data.execution.option_exercise_type,
            crate::orders::OptionExerciseType::Expired,
            "execution.option_exercise_type"
        );
    } else {
        assert!(false, "expected execution data notification");
    }

    if let Some(Ok(OrderUpdate::CommissionAndFeesReport(report))) = notifications.next_data() {
        assert_eq!(report.execution_id, "00025b46.63f8f39c.01.01", "report.execution_id");
        assert_eq!(report.commission_and_fees, 1.0, "report.commission_and_fees");
        assert_eq!(report.currency, "USD", "report.currency");
    } else {
        assert!(false, "expected commission report notification");
    }
}

#[test]
fn order_update_stream_survives_unknown_status() {
    // Regression test for #774: a status string this crate does not model
    // must arrive as OrderStatusKind::Unknown, not terminate the stream —
    // the frame queued behind it still arrives.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::OrderStatus,
            order_status()
                .order_id(1)
                .status(OrderStatusKind::Unknown("PendingReplace".into()))
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::OrderStatus,
            order_status().order_id(1).status(OrderStatusKind::Filled).encode_proto(),
        ),
    ]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let stream = client.order_update_stream().expect("failed to create stream");

    match stream.next_data() {
        Some(Ok(OrderUpdate::OrderStatus(s))) => {
            assert_eq!(s.status, OrderStatusKind::Unknown("PendingReplace".into()));
        }
        other => panic!("expected OrderStatus with Unknown status, got {other:?}"),
    }
    match stream.next_data() {
        Some(Ok(OrderUpdate::OrderStatus(s))) => {
            assert_eq!(s.status, OrderStatusKind::Filled);
        }
        other => panic!("stream did not survive the unknown status, got {other:?}"),
    }
}

#[test]
fn order_update_stream_survives_unknown_condition() {
    // #827: an OpenOrder carrying a condition type this crate does not model
    // arrives as OrderCondition::Unknown and the frame queued behind it still arrives.
    let unknown = proto::OrderCondition {
        r#type: Some(2),
        is_conjunction_connection: Some(true),
        ..Default::default()
    };
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::OpenOrder,
            open_order().order_id(1).conditions(vec![unknown]).encode_proto(),
        ),
        proto_response(IncomingMessages::OpenOrder, open_order().order_id(2).encode_proto()),
    ]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let stream = client.order_update_stream().expect("failed to create stream");

    match stream.next_data() {
        Some(Ok(OrderUpdate::OpenOrder(o))) => match &o.order.conditions[..] {
            [OrderCondition::Unknown(c)] => assert_eq!(c.condition_type, 2),
            other => panic!("expected one Unknown condition, got {other:?}"),
        },
        other => panic!("expected OpenOrder, got {other:?}"),
    }
    match stream.next_data() {
        Some(Ok(OrderUpdate::OpenOrder(o))) => assert_eq!(o.order_id, 2),
        other => panic!("stream did not survive the unknown condition, got {other:?}"),
    }
}

#[test]
fn order_update_stream_survives_unknown_sec_type() {
    // SecurityType is open: an OpenOrder frame whose contract carries a
    // sec_type this crate does not model arrives as SecurityType::Other and
    // the frame queued behind it still arrives.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::OpenOrder,
            open_order().order_id(1).security_type("NOTASECTYPE").encode_proto(),
        ),
        proto_response(IncomingMessages::OpenOrder, open_order().order_id(1).security_type("STK").encode_proto()),
    ]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let stream = client.order_update_stream().expect("failed to create stream");

    match stream.next_data() {
        Some(Ok(OrderUpdate::OpenOrder(o))) => {
            assert_eq!(o.contract.security_type, Some(SecurityType::Other("NOTASECTYPE".into())));
        }
        other => panic!("expected OpenOrder with Other sec_type, got {other:?}"),
    }
    match stream.next_data() {
        Some(Ok(OrderUpdate::OpenOrder(o))) => {
            assert_eq!(o.contract.security_type, Some(SecurityType::Stock));
        }
        other => panic!("stream did not survive the unknown sec_type, got {other:?}"),
    }
}

#[test]
fn order_update_stream_survives_unknown_tif() {
    // TimeInForce is open: an OpenOrder frame carrying a tif this crate
    // does not model arrives as TimeInForce::Unknown and the frame queued
    // behind it still arrives.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(IncomingMessages::OpenOrder, open_order().order_id(1).tif("NOTATIF").encode_proto()),
        proto_response(IncomingMessages::OpenOrder, open_order().order_id(1).tif("GTC").encode_proto()),
    ]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let stream = client.order_update_stream().expect("failed to create stream");

    match stream.next_data() {
        Some(Ok(OrderUpdate::OpenOrder(o))) => {
            assert_eq!(o.order.tif, TimeInForce::Unknown("NOTATIF".into()));
        }
        other => panic!("expected OpenOrder with Unknown tif, got {other:?}"),
    }
    match stream.next_data() {
        Some(Ok(OrderUpdate::OpenOrder(o))) => {
            assert_eq!(o.order.tif, TimeInForce::GoodTillCanceled);
        }
        other => panic!("stream did not survive the unknown tif, got {other:?}"),
    }
}

#[test]
fn order_update_stream_survives_unknown_order_type() {
    // OrderType is open: an OpenOrder frame carrying an order type this crate
    // does not list arrives as OrderType::Unknown and the frame queued behind
    // it still arrives.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::OpenOrder,
            open_order().order_id(1).order_type("SNAP MID").encode_proto(),
        ),
        proto_response(IncomingMessages::OpenOrder, open_order().order_id(1).order_type("LMT").encode_proto()),
    ]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let stream = client.order_update_stream().expect("failed to create stream");

    match stream.next_data() {
        Some(Ok(OrderUpdate::OpenOrder(o))) => {
            assert_eq!(o.order.order_type, Some(OrderType::Unknown("SNAP MID".into())));
        }
        other => panic!("expected OpenOrder with Unknown order type, got {other:?}"),
    }
    match stream.next_data() {
        Some(Ok(OrderUpdate::OpenOrder(o))) => {
            assert_eq!(o.order.order_type, Some(OrderType::Limit));
        }
        other => panic!("stream did not survive the unknown order type, got {other:?}"),
    }
}

#[test]
fn order_update_stream_survives_unknown_security_id_type() {
    // SecurityIdType is open: a contract carrying an identifier scheme this
    // crate does not model arrives as SecurityIdType::Unknown and the frame
    // queued behind it still arrives (#840).
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::OpenOrder,
            open_order().order_id(1).security_id_type("WKN").encode_proto(),
        ),
        proto_response(IncomingMessages::OpenOrder, open_order().order_id(2).encode_proto()),
    ]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let stream = client.order_update_stream().expect("failed to create stream");

    match stream.next_data() {
        Some(Ok(OrderUpdate::OpenOrder(o))) => {
            assert_eq!(o.contract.security_id_type, Some(SecurityIdType::Unknown("WKN".into())));
        }
        other => panic!("expected OpenOrder with Unknown security_id_type, got {other:?}"),
    }
    match stream.next_data() {
        Some(Ok(OrderUpdate::OpenOrder(o))) => assert_eq!(o.order_id, 2),
        other => panic!("stream did not survive the unknown security_id_type, got {other:?}"),
    }
}

#[test]
fn order_update_stream_ends_on_empty_action() {
    // action is required: an OpenOrder frame with an empty action fails
    // decode with Error::Parse, which terminates the subscription — the
    // frame queued behind it is never delivered. The survives_unknown_tif
    // test above uses the same two-frame stub and does receive its second
    // frame, so the None here comes from termination, not an empty stub.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(IncomingMessages::OpenOrder, open_order().order_id(1).action("").encode_proto()),
        proto_response(IncomingMessages::OpenOrder, open_order().order_id(1).action("SELL").encode_proto()),
    ]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let stream = client.order_update_stream().expect("failed to create stream");

    match stream.next_data() {
        Some(Err(Error::Parse { field, .. })) => assert_eq!(field, Some("action"), "expected the field name in the error"),
        other => panic!("expected Error::Parse for the empty action, got {other:?}"),
    }
    assert!(stream.next_data().is_none(), "stream should end after a decode error");
}

#[test]
fn order_update_stream_already_subscribed() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    // Create first subscription
    let stream1 = client.order_update_stream();
    assert!(stream1.is_ok(), "failed to create first order update stream");

    // Try to create second subscription - should fail
    let stream2 = client.order_update_stream();
    assert!(stream2.is_err(), "second order update stream should fail");

    match stream2.err().unwrap() {
        Error::AlreadySubscribed => {}
        other => assert!(false, "expected AlreadySubscribed error, got: {:?}", other),
    }
}

// Covers the `Client::order` delegate only. Field-by-field builder semantics are
// asserted in orders/builder/tests.rs against a mock client; what is unique here
// is that the entry point binds the real `Client` and the given contract.
#[test]
fn order_entry_point_builds_order() {
    let (client, _) = create_blocking_test_client();
    let contract = Contract::stock("AAPL").build();

    let order = client.order(&contract).buy(100).limit(50.0).build().expect("build failed");

    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.limit_price, Some(50.0));
}

// Drives the real `OrderBuilder::analyze`. A rejected what-if order arrives as
// a routed `Err`; before #735 the read
// discarded it and the caller saw `UnexpectedEndOfStream`.
#[test]
fn analyze_surfaces_rejected_what_if_order() {
    let (client, _bus) = create_blocking_test_client_with_ordered_proto_responses(vec![proto_error_response(
        9000,
        201,
        "Order rejected - reason:Insufficient buying power",
    )]);
    let contract = Contract::stock("AAPL").build();

    let err = client
        .order(&contract)
        .buy(100)
        .limit(50.0)
        .analyze()
        .expect_err("a rejected what-if order must surface the rejection");
    assert_tws_error_message(err, 201, "Insufficient buying power");
}

// Happy path for the real `OrderBuilder::analyze`, replacing the mock-client
// shadow that carried its own copy of the method (and its own copy of the bug
// #735 fixed). Asserts both halves of the contract: the OrderState comes back
// from the OpenOrder whose id matches, and the request went out what-if.
#[test]
fn analyze_returns_order_state_for_the_matching_order() {
    let (client, bus) = create_blocking_test_client_with_ordered_proto_responses(vec![proto_response(
        IncomingMessages::OpenOrder,
        open_order().order_id(9090).status(OrderStatusKind::PreSubmitted).encode_proto(),
    )]);
    client.id_manager().raise_order_id(OrderId::from(9090));
    let contract = Contract::stock("AAPL").build();

    let state = client.order(&contract).buy(100).limit(50.0).analyze().expect("analyze should succeed");
    assert_eq!(state.status, OrderStatusKind::PreSubmitted);

    let request: crate::proto::PlaceOrderRequest = decode_request_proto(&bus, 0);
    assert_eq!(request.order.expect("request carries an order").what_if, Some(true));
}

#[test]
fn analyze_reports_end_of_stream_when_no_order_arrives() {
    let (client, _bus) = create_blocking_test_client_with_ordered_proto_responses(vec![]);
    let contract = Contract::stock("AAPL").build();

    let result = client.order(&contract).buy(100).limit(50.0).analyze();
    assert!(matches!(result, Err(Error::UnexpectedEndOfStream)), "got {result:?}");
}

// The `submit` family, against `Client::stubbed` rather than a mock client that
// carried its own copy of each method. The shadows dated from before the
// builder had a real seam to test against; #735 moved `analyze` off its shadow
// after one of them turned out to carry the very bug the PR was fixing.

#[test]
fn submit_assigns_the_next_order_id_and_sends_the_order() {
    let (client, bus) = create_blocking_test_client();
    client.id_manager().raise_order_id(OrderId::from(9100));
    let contract = Contract::stock("AAPL").build();

    let order_id = client.order(&contract).buy(100).limit(50.0).submit().expect("submit should succeed");
    assert_eq!(order_id, 9100);

    assert_eq!(request_message_count(&bus), 1);
    let request: crate::proto::PlaceOrderRequest = decode_request_proto(&bus, 0);
    assert_eq!(request.order_id, Some(9100));
    let order = request.order.expect("request carries an order");
    assert_eq!(order.action.as_deref(), Some("BUY"));
    assert_eq!(order.order_type.as_deref(), Some("LMT"));
    assert_eq!(order.lmt_price, Some(50.0));
    assert!(!order.what_if.unwrap_or_default(), "submit is not a what-if order");
}

#[test]
fn submit_rejects_an_invalid_order_before_sending() {
    let (client, bus) = create_blocking_test_client();
    let contract = Contract::stock("AAPL").build();

    let err = client
        .order(&contract)
        .buy(-100)
        .market()
        .submit()
        .expect_err("a negative quantity is invalid");
    assert!(
        matches!(err, Error::Validation(ValidationError::InvalidQuantity(q)) if q == -100.0),
        "got {err:?}"
    );
    assert_eq!(request_message_count(&bus), 0, "an invalid order must not reach the wire");
}

#[test]
fn submit_all_reserves_three_ids_and_wires_the_bracket() {
    let (client, bus) = create_blocking_test_client();
    client.id_manager().raise_order_id(OrderId::from(9200));
    let contract = Contract::stock("AAPL").build();

    let ids = client
        .order(&contract)
        .buy(100)
        .good_till_canceled()
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0)
        .submit_all()
        .expect("bracket submission should succeed");

    assert_eq!((ids.parent, ids.take_profit, ids.stop_loss), (9200, 9201, 9202));
    assert_eq!(request_message_count(&bus), 3);

    let orders: Vec<crate::proto::Order> = (0..3)
        .map(|i| {
            decode_request_proto::<crate::proto::PlaceOrderRequest>(&bus, i)
                .order
                .expect("request carries an order")
        })
        .collect();

    // Parent first, then the two children pointing back at it. A proto field
    // at its default is omitted on the wire, so read them through unwrap_or_default.
    assert_eq!(
        orders.iter().map(|o| o.parent_id.unwrap_or_default()).collect::<Vec<_>>(),
        vec![0, 9200, 9200]
    );

    // Only the last order transmits, so TWS receives the trio atomically.
    assert_eq!(
        orders.iter().map(|o| o.transmit.unwrap_or_default()).collect::<Vec<_>>(),
        vec![false, false, true]
    );

    // Time in force propagates from the parent builder to all three.
    for order in &orders {
        assert_eq!(order.tif.as_deref(), Some("GTC"));
    }

    assert_eq!(orders[1].action.as_deref(), Some("SELL"));
    assert_eq!(orders[1].lmt_price, Some(55.0));
    assert_eq!(orders[2].order_type.as_deref(), Some("STP"));
    assert_eq!(orders[2].aux_price, Some(45.0));
}

#[test]
fn submit_linked_orders_preserves_ids_and_transmits_only_the_last_order() {
    let (client, bus) = create_blocking_test_client();
    client.submit_linked_orders(linked_orders()).unwrap();

    assert_linked_order_requests(&bus, 3);
    assert_eq!(client.next_order_id(), 9410);
}

#[test]
fn submit_linked_orders_checks_every_order_before_sending() {
    let (client, bus) = create_blocking_test_client();
    let empty = client.submit_linked_orders(Vec::new()).unwrap_err();
    assert!(matches!(empty, Error::InvalidArgument(message) if message == "submit_linked_orders needs at least one order"));

    for (id, reason) in [
        (0, "0 is the unset order id"),
        (LINKED_ORDER_IDS[0], "each linked order needs a distinct order id"),
    ] {
        let mut orders = linked_orders();
        orders[2].1.order_id = id;
        let e = client.submit_linked_orders(orders).unwrap_err();
        assert!(
            matches!(e, Error::Validation(ValidationError::InvalidField { field: "Order::order_id", value, reason: actual })
            if value == id.to_string() && actual == reason),
            "{id}"
        );
    }

    let mut orders = linked_orders();
    orders[2].1.order_id = REQUEST_ID_FLOOR;
    let e = client.submit_linked_orders(orders).unwrap_err();
    assert!(matches!(e, Error::OrderIdInRequestRange { order_id } if order_id == REQUEST_ID_FLOOR));
    assert_eq!(request_message_count(&bus), 0);
}

#[test]
fn submit_linked_orders_reports_ambiguous_writes_without_sending_cancels() {
    for writes in 0..LINKED_ORDER_IDS.len() {
        let bus = Arc::new(MessageBusStub::default().with_sends_failing_after(writes));
        let client = Client::stubbed(bus.clone(), server_versions::SIZE_RULES);
        let e = client.submit_linked_orders(linked_orders()).unwrap_err();
        let Error::OrdersPartiallySent { sent, not_sent, source } = e else {
            panic!("expected a partial send, received {e:?}");
        };
        let (expected_sent, expected_not_sent) = LINKED_ORDER_IDS.split_at(writes + 1);
        assert_eq!(sent, expected_sent);
        assert_eq!(not_sent, expected_not_sent);
        assert!(matches!(*source, Error::ConnectionReset));
        assert_linked_order_requests(&bus, writes + 1);
    }
}

#[test]
fn submit_linked_orders_returns_refused_first_send_bare() {
    let bus = Arc::new(MessageBusStub::default().with_refused_sends());
    let client = Client::stubbed(bus.clone(), server_versions::SIZE_RULES);
    let e = client.submit_linked_orders(linked_orders()).unwrap_err();

    assert!(matches!(e, Error::Reconnecting));
    assert!(e.is_unsent());
    assert_eq!(request_message_count(&bus), 0);
}

#[test]
fn submit_oca_orders_numbers_each_order_and_sets_the_group() {
    let (client, bus) = create_blocking_test_client();
    client.id_manager().raise_order_id(OrderId::from(9300));
    let apple = Contract::stock("AAPL").build();
    let microsoft = Contract::stock("MSFT").build();

    // No group on the first; a different one on the second, which the call overwrites.
    let first = client.order(&apple).buy(100).limit(50.0).build_order().expect("order should build");
    let second = client
        .order(&microsoft)
        .buy(100)
        .limit(45.0)
        .oca_group("Other", OcaType::ReduceWithBlock)
        .build_order()
        .expect("order should build");

    let ids = client
        .submit_oca_orders("TestOCA", OcaType::CancelWithBlock, vec![(apple, first), (microsoft, second)])
        .expect("OCA submission should succeed");

    assert_eq!(ids, vec![9300, 9301]);
    assert_eq!(request_message_count(&bus), 2);

    for (i, id) in ids.iter().enumerate() {
        let request: crate::proto::PlaceOrderRequest = decode_request_proto(&bus, i);
        assert_eq!(request.order_id, Some(*id));
        let order = request.order.expect("request carries an order");
        assert_eq!(order.oca_group.as_deref(), Some("TestOCA"));
        assert_eq!(order.oca_type, Some(1));
    }
}

#[test]
fn submit_oca_orders_rejects_an_empty_group() {
    let (client, bus) = create_blocking_test_client();
    client.id_manager().raise_order_id(OrderId::from(9300));
    let contract = Contract::stock("AAPL").build();
    let order = client.order(&contract).buy(100).limit(50.0).build_order().expect("order should build");

    let result = client.submit_oca_orders("", OcaType::CancelWithBlock, vec![(contract, order)]);

    assert!(
        matches!(
            result,
            Err(Error::Validation(ValidationError::InvalidField {
                field: "Order::oca_group",
                ..
            }))
        ),
        "{result:?}"
    );
    assert_eq!(request_message_count(&bus), 0);
    assert_eq!(client.next_order_id(), 9300, "no id is taken for a rejected group");
}

/// The second order fails its id check, so neither goes out: the first must not be left live
/// on its own.
#[test]
fn submit_oca_orders_sends_nothing_when_a_later_order_fails() {
    let (client, bus) = create_blocking_test_client();
    client.id_manager().raise_order_id(OrderId::from(REQUEST_ID_FLOOR - 1));
    let apple = Contract::stock("AAPL").build();
    let microsoft = Contract::stock("MSFT").build();
    let first = client.order(&apple).buy(100).limit(50.0).build_order().expect("order should build");
    let second = client.order(&microsoft).buy(100).limit(45.0).build_order().expect("order should build");

    let result = client.submit_oca_orders("TestOCA", OcaType::CancelWithBlock, vec![(apple, first), (microsoft, second)]);

    assert!(
        matches!(result, Err(Error::OrderIdInRequestRange { order_id }) if order_id == REQUEST_ID_FLOOR),
        "{result:?}"
    );
    assert_eq!(request_message_count(&bus), 0);
}

/// The take-profit's id fails its check after the parent's passes, so no leg goes out: a held
/// parent must not be left at TWS.
#[test]
fn submit_all_sends_nothing_when_a_later_leg_fails() {
    let (client, bus) = create_blocking_test_client();
    client.id_manager().raise_order_id(OrderId::from(REQUEST_ID_FLOOR - 1));
    let contract = Contract::stock("AAPL").build();

    let result = client
        .order(&contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0)
        .submit_all();

    assert!(
        matches!(result, Err(Error::OrderIdInRequestRange { order_id }) if order_id == REQUEST_ID_FLOOR),
        "{result:?}"
    );
    assert_eq!(request_message_count(&bus), 0);
}

/// A send that fails partway through a bracket reports which legs may be at TWS: every leg up to
/// and including the one whose write failed. Table: messages allowed through before the failure.
#[test]
fn submit_all_reports_the_legs_sent_when_a_send_fails() {
    let cases = [
        ("parent's write fails", 0, vec![9200], vec![9201, 9202]),
        ("take-profit's write fails", 1, vec![9200, 9201], vec![9202]),
        ("stop-loss's write fails", 2, vec![9200, 9201, 9202], vec![]),
    ];
    for (label, sends_before_failure, expected_sent, expected_not_sent) in cases {
        let bus = Arc::new(MessageBusStub::default().with_sends_failing_after(sends_before_failure));
        let client = Client::stubbed(bus.clone(), server_versions::SIZE_RULES);
        client.id_manager().raise_order_id(OrderId::from(9200));
        let contract = Contract::stock("AAPL").build();

        let result = client
            .order(&contract)
            .buy(100)
            .bracket()
            .entry_limit(50.0)
            .take_profit(55.0)
            .stop_loss(45.0)
            .submit_all();

        match result {
            Err(Error::OrdersPartiallySent { sent, not_sent, source }) => {
                assert_eq!(sent, expected_sent, "{label}");
                assert_eq!(not_sent, expected_not_sent, "{label}");
                assert!(matches!(*source, Error::ConnectionReset), "{label}: {source:?}");
            }
            other => panic!("{label}: expected OrdersPartiallySent, got {other:?}"),
        }
        assert_eq!(
            request_message_count(&bus),
            sends_before_failure + 1,
            "{label}: no write after the failed one"
        );
    }
}

#[test]
fn submit_oca_orders_reports_the_orders_sent_when_a_send_fails() {
    let bus = Arc::new(MessageBusStub::default().with_sends_failing_after(1));
    let client = Client::stubbed(bus.clone(), server_versions::SIZE_RULES);
    client.id_manager().raise_order_id(OrderId::from(9300));
    let apple = Contract::stock("AAPL").build();
    let microsoft = Contract::stock("MSFT").build();
    let first = client.order(&apple).buy(100).limit(50.0).build_order().expect("order should build");
    let second = client.order(&microsoft).buy(100).limit(45.0).build_order().expect("order should build");

    let result = client.submit_oca_orders("TestOCA", OcaType::CancelWithBlock, vec![(apple, first), (microsoft, second)]);

    assert!(
        matches!(&result, Err(Error::OrdersPartiallySent { sent, not_sent, .. }) if *sent == [9300, 9301] && not_sent.is_empty()),
        "{result:?}"
    );
}

/// A single order is not a batch: its send failure comes back as is.
#[test]
fn submit_order_returns_a_failed_send_unwrapped() {
    let bus = Arc::new(MessageBusStub::default().with_failing_sends());
    let client = Client::stubbed(bus.clone(), server_versions::SIZE_RULES);
    let contract = Contract::stock("AAPL").build();
    let order = client.order(&contract).buy(100).limit(50.0).build_order().expect("order should build");

    let result = client.submit_order(9400, &contract, &order);

    assert!(matches!(result, Err(Error::ConnectionReset)), "{result:?}");
}

#[test]
fn build_order_does_not_reach_the_wire() {
    let (client, bus) = create_blocking_test_client();
    let contract = Contract::stock("AAPL").build();

    let order = client
        .order(&contract)
        .sell(100)
        .trailing_stop(TrailBy::Percent(5.0), 95.0)
        .build_order()
        .expect("order should build");

    assert_eq!(order.order_type, Some(OrderType::TrailingStop));
    assert_eq!(order.trailing_percent, Some(5.0));
    assert_eq!(order.trail_stop_price, Some(95.0));
    assert_eq!(request_message_count(&bus), 0);
}

#[test]
fn submit_carries_algo_parameters_to_the_wire() {
    // The mock-client shadow asserted this against a captured `Order` struct;
    // at the real seam the assertion is what TWS receives.
    let (client, bus) = create_blocking_test_client();
    let contract = Contract::stock("AAPL").build();

    client
        .order(&contract)
        .buy(100)
        .limit(50.0)
        .algo("VWAP")
        .algo_param("startTime", "09:30:00")
        .algo_param("endTime", "16:00:00")
        .submit()
        .expect("submit should succeed");

    let order = decode_request_proto::<crate::proto::PlaceOrderRequest>(&bus, 0)
        .order
        .expect("request carries an order");
    assert_eq!(order.algo_strategy.as_deref(), Some("VWAP"));
    assert_eq!(order.algo_params.get("startTime").map(String::as_str), Some("09:30:00"));
    assert_eq!(order.algo_params.get("endTime").map(String::as_str), Some("16:00:00"));
}

#[test]
fn submit_rejects_a_non_finite_price_before_sending() {
    let (client, bus) = create_blocking_test_client();
    let contract = Contract::stock("AAPL").build();

    let err = client.order(&contract).buy(100).limit(f64::NAN).submit().expect_err("NaN is not a price");
    assert!(err.to_string().contains("Invalid price"), "got {err}");
    assert_eq!(request_message_count(&bus), 0);
}

#[test]
fn order_update_stream_delivers_order_binding() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::OrderBound,
        order_bound().encode_proto(),
    )]));
    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_REST_MESSAGES_3);
    let stream = client.order_update_stream().unwrap();
    let Some(Ok(OrderUpdate::OrderBound(bound))) = stream.next_data() else {
        panic!("expected an order binding notification");
    };
    assert_eq!(
        bound,
        crate::orders::OrderBound {
            perm_id: 9_876_543_210,
            client_id: 0,
            order_id: 42
        }
    );
}

#[test]
fn preset_legs_submit_one_request_with_attached_ids() {
    let (client, bus) = crate::common::test_utils::helpers::create_blocking_test_client_with_version(server_versions::ATTACHED_ORDERS);
    client.id_manager().raise_order_id(OrderId::from(9400));
    let contract = Contract::stock("AAPL").build();

    let ids = client
        .order(&contract)
        .buy(100)
        .limit(50.0)
        .preset_stop_loss()
        .preset_profit_taker()
        .submit()
        .expect("submission should succeed");

    assert_eq!(ids.parent, 9400);
    assert_eq!(ids.stop_loss, Some(9401));
    assert_eq!(ids.profit_taker, Some(9402));

    // One request: TWS creates the children from the parent.
    assert_eq!(request_message_count(&bus), 1);
    let request = decode_request_proto::<crate::proto::PlaceOrderRequest>(&bus, 0);
    assert_eq!(request.order_id, Some(9400));
    assert_eq!(
        request.attached_orders,
        Some(crate::proto::AttachedOrders {
            sl_order_id: Some(9401),
            sl_order_type: Some("PRESET".to_string()),
            pt_order_id: Some(9402),
            pt_order_type: Some("PRESET".to_string()),
        })
    );
}

#[test]
fn preset_legs_rejected_below_attached_orders_gate() {
    let (client, bus) = crate::common::test_utils::helpers::create_blocking_test_client_with_version(server_versions::ATTACHED_ORDERS - 1);
    let contract = Contract::stock("AAPL").build();

    let result = client.order(&contract).buy(100).limit(50.0).preset_stop_loss().submit();

    match result {
        Err(Error::ServerVersion { required, .. }) => assert_eq!(required, server_versions::ATTACHED_ORDERS),
        other => panic!("expected ServerVersion error, got {other:?}"),
    }
    assert_eq!(request_message_count(&bus), 0);
}

/// Order ids in the request range are refused before anything is sent: their
/// frames would route as a request's (#789).
#[test]
fn order_entry_points_reject_request_range_ids() {
    let floor = REQUEST_ID_FLOOR;
    let (client, message_bus) = create_blocking_test_client();
    let contract = Contract::stock("AAPL").build();
    let order = order_builder::market_order(Action::Buy, 100.0);
    let with_parent = crate::orders::Order {
        parent_id: floor,
        ..order.clone()
    };
    let rejected = |result: Result<(), Error>, what: &str| assert!(matches!(result, Err(Error::OrderIdInRequestRange { .. })), "{what}: {result:?}");

    rejected(client.place_order(floor, &contract, &order).map(|_| ()), "place_order");
    rejected(client.place_order(1, &contract, &with_parent).map(|_| ()), "place_order parent_id");
    rejected(client.submit_order(floor, &contract, &order), "submit_order");
    rejected(client.cancel_order(floor).map(|_| ()), "cancel_order");
    rejected(client.order_status_stream(floor).map(|_| ()), "order_status_stream");
    rejected(client.wait_for_fill(floor, std::time::Duration::ZERO).map(|_| ()), "wait_for_fill");

    client.id_manager().raise_order_id(OrderId::from(floor));
    rejected(client.exercise_options(&contract).exercise(1).submit().map(|_| ()), "exercise_options");

    assert_eq!(request_message_count(&message_bus), 0, "nothing reaches the wire");
}

/// A server answer in the request range is refused, and does not raise the
/// local order-id generator into it (#789).
#[test]
fn next_valid_order_id_rejects_request_range() {
    let (client, _bus) = create_blocking_test_client_with_ordered_proto_responses(vec![proto_response(
        IncomingMessages::NextValidId,
        next_valid_id().order_id(REQUEST_ID_FLOOR).encode_proto(),
    )]);

    let result = client.next_valid_order_id();
    assert!(
        matches!(result, Err(Error::OrderIdInRequestRange { order_id }) if order_id == REQUEST_ID_FLOOR),
        "{result:?}"
    );
    assert!(client.next_order_id() < REQUEST_ID_FLOOR, "generator raised into the request range");
}

/// The order methods take `impl Into<OrderId>`: ids from `BracketOrderIds`
/// and `AttachedOrderIds` pass straight in, as do plain `i32`s.
#[test]
fn order_methods_accept_typed_order_ids() {
    let (client, message_bus) = create_blocking_test_client();
    let contract = Contract::stock("AAPL").build();
    let ids = crate::orders::BracketOrderIds::new(41, 42, 43);
    let order = order_builder::market_order(Action::Buy, 100.0);

    client.submit_order(ids.parent, &contract, &order).expect("submit_order");
    let _cancel = client.cancel_order(ids.take_profit).expect("cancel_order");
    let _placed = client.place_order(ids.stop_loss, &contract, &order).expect("place_order");

    let placed_id = |index| decode_request_proto::<crate::proto::PlaceOrderRequest>(&message_bus, index).order_id;
    assert_eq!(placed_id(0), Some(41), "submit_order");
    assert_request(&message_bus, 1, &cancel_order_request().order_id(42));
    assert_eq!(placed_id(2), Some(43), "place_order");

    assert_eq!(order_builder::market_f_hedge(ids.parent, Action::Sell).parent_id, 41);
    let bracket = order_builder::bracket_order(
        ids.parent,
        Action::Buy,
        100.0,
        BracketPrices {
            entry: 50.0,
            take_profit: 55.0,
            stop_loss: 45.0,
        },
    )
    .unwrap();
    assert_eq!(bracket[1].parent_id, 41, "bracket_order");
}

#[test]
fn wait_for_fill_waits_through_partial_fills() {
    let (client, message_bus) = create_blocking_test_client_with_ordered_proto_responses(vec![
        order_status_response(order_status().status(OrderStatusKind::Submitted).filled(0.0).remaining(100.0)),
        order_status_response(order_status().status(OrderStatusKind::Submitted).filled(40.0).remaining(60.0)),
        order_status_response(order_status().status(OrderStatusKind::Filled).filled(100.0).remaining(0.0)),
    ]);

    let outcome = client.wait_for_fill(13, std::time::Duration::from_secs(5)).unwrap();

    assert!(matches!(&outcome, OrderOutcome::Filled(s) if s.filled == 100.0), "{outcome:?}");
    assert_eq!(request_message_count(&message_bus), 0, "waiting writes nothing");
}

#[test]
fn wait_for_fill_times_out_with_last_status() {
    let (client, _) = create_blocking_test_client_with_ordered_proto_responses(vec![order_status_response(
        order_status().status(OrderStatusKind::Submitted).filled(40.0).remaining(60.0),
    )]);

    let outcome = client.wait_for_fill(13, std::time::Duration::from_millis(20)).unwrap();

    assert!(matches!(&outcome, OrderOutcome::TimedOut(Some(s)) if s.filled == 40.0), "{outcome:?}");
}

/// No status seen for the order (e.g. placed on an earlier connection): the
/// wait asks TWS for this client's open orders, then waits as usual.
#[test]
fn wait_for_fill_asks_for_open_orders_when_status_unseen() {
    let (client, message_bus) = create_blocking_test_client_with_ordered_proto_responses(vec![]);

    let outcome = client.wait_for_fill(13, std::time::Duration::from_millis(20)).unwrap();

    assert_eq!(outcome, OrderOutcome::TimedOut(None));
    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &open_orders_request());
}

/// An order TWS refuses with an error and no status (live: ES limit off the
/// tick, 110) ends the wait at once instead of timing out.
#[test]
fn wait_for_fill_reports_rejection_without_status() {
    let (client, _) = create_blocking_test_client_with_ordered_proto_responses(vec![proto_error_response(
        13,
        110,
        "The price does not conform to the minimum price variation for this contract.",
    )]);

    let outcome = client.wait_for_fill(13, std::time::Duration::from_secs(5)).unwrap();

    assert!(matches!(&outcome, OrderOutcome::Rejected(n) if n.code == 110), "{outcome:?}");
}

/// TWS lost its connection to IB and named the order in a 1104 after its
/// ApiPending status (live 2026-11-26): the wait ends with `PendingCreation`
/// instead of timing out on an order TWS never created (#1322).
#[test]
fn wait_for_fill_reports_pending_creation() {
    let (client, _) = create_blocking_test_client_with_ordered_proto_responses(vec![
        order_status_response(order_status().status(OrderStatusKind::ApiPending).filled(0.0).remaining(1.0)),
        proto_error_response(-1, crate::messages::ORDER_CREATION_PENDING_CODE, "Pending to create 1 orders: 13"),
    ]);

    let outcome = client.wait_for_fill(13, std::time::Duration::from_secs(5)).unwrap();

    assert!(
        matches!(&outcome, OrderOutcome::PendingCreation(n) if n.pending_order_ids() == Some(vec![13])),
        "{outcome:?}"
    );
}

#[test]
fn wait_for_fill_fails_on_connection_reset() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]).with_connection_resets(1));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let result = client.wait_for_fill(13, std::time::Duration::from_secs(5));

    assert!(matches!(result, Err(Error::ConnectionReset)), "{result:?}");
}

#[test]
fn order_status_stream_yields_statuses_and_writes_nothing() {
    let (client, message_bus) = create_blocking_test_client_with_ordered_proto_responses(vec![
        order_status_response(order_status().status(OrderStatusKind::Submitted).filled(40.0).remaining(60.0)),
        order_status_response(order_status().status(OrderStatusKind::Filled).filled(100.0).remaining(0.0)),
    ]);

    let statuses = client.order_status_stream(13).unwrap();
    let filled: Vec<f64> = statuses.iter_data().take(2).map(|status| status.unwrap().filled).collect();

    assert_eq!(filled, [40.0, 100.0]);
    assert_eq!(request_message_count(&message_bus), 0);
}

// A caller-chosen id must keep later client-allocated ids (next_order_id, the order
// builder's submit, exercise_options) from reusing it, or TWS reads the later order as a
// modification of the earlier one.
#[test]
fn caller_chosen_order_id_raises_the_generator() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::ATTACHED_ORDERS);
    let contract = Contract::stock("AAPL").build();
    let order = order_builder::limit_order(Action::Buy, 1.0, 1.0);
    let chosen = client.next_order_id() + 1000;

    client.submit_order(chosen, &contract, &order).expect("submit_order");
    assert_eq!(client.id_manager().current_order_id(), chosen + 1);

    let bracket = Order {
        preset_stop_loss_order_id: Some(chosen + 20),
        preset_profit_taker_order_id: Some(chosen + 10),
        ..order_builder::limit_order(Action::Buy, 1.0, 1.0)
    };
    let _subscription = client.place_order(chosen + 5, &contract, &bracket).expect("place_order");
    assert_eq!(client.id_manager().current_order_id(), chosen + 21, "past the preset attached-order ids");

    client.submit_order(chosen, &contract, &order).expect("modify submit_order");
    assert_eq!(
        client.id_manager().current_order_id(),
        chosen + 21,
        "a lower id never rewinds the generator"
    );
}

// The documented flow: fetch an id from TWS, place it yourself, then let the client allocate.
// The client-allocated id must not repeat the one already placed.
#[test]
fn next_valid_order_id_then_submit_order_does_not_reissue_the_id() {
    let (client, _bus) = create_blocking_test_client_with_ordered_proto_responses(vec![proto_response(
        IncomingMessages::NextValidId,
        next_valid_id().order_id(9123).encode_proto(),
    )]);
    let contract = Contract::stock("AAPL").build();
    let order = order_builder::limit_order(Action::Buy, 1.0, 1.0);

    let order_id = client.next_valid_order_id().expect("next_valid_order_id");
    client.submit_order(order_id, &contract, &order).expect("submit_order");

    assert_eq!(client.next_order_id(), order_id + 1);
}

/// `Error::is_unsent` is `true` only when the failing order call wrote nothing. The converse
/// doesn't hold: a request refused after the client gave up or shut down writes nothing yet is
/// `false`, because `ConnectionFailed` and `Shutdown` also end writes already in progress.
#[test]
fn is_unsent_implies_the_order_call_wrote_nothing() {
    /// What the stubbed bus does with a write.
    #[derive(Clone, Copy)]
    enum Sends {
        Succeed,
        /// Recorded, then fails: the request may have reached TWS.
        Fail,
        /// Fails with `Reconnecting` without recording, as a request refused while reconnecting.
        Refuse,
    }
    type Call = fn(&Client) -> Result<(), Error>;
    fn bad_operator() -> crate::orders::OrderCancel {
        crate::orders::OrderCancel {
            ext_operator: "TRADER_7".to_owned(),
            ..Default::default()
        }
    }
    fn limit() -> Order {
        order_builder::limit_order(Action::Buy, 1.0, 1.0)
    }
    let cases: [(&str, Sends, bool, Call); 15] = [
        ("place_order, id in the request range", Sends::Succeed, true, |client| {
            client.place_order(REQUEST_ID_FLOOR, &Contract::stock("AAPL").build(), &limit()).map(drop)
        }),
        ("place_order, bad ext_operator", Sends::Succeed, true, |client| {
            let order = Order {
                ext_operator: "TRADER_7".to_owned(),
                ..limit()
            };
            client.place_order(41, &Contract::stock("AAPL").build(), &order).map(drop)
        }),
        ("place_order, field above the server version", Sends::Succeed, true, |client| {
            let order = Order {
                hedge_max_size: Some(10),
                ..limit()
            };
            client.place_order(41, &Contract::stock("AAPL").build(), &order).map(drop)
        }),
        ("cancel_order, id in the request range", Sends::Succeed, true, |client| {
            client.cancel_order(REQUEST_ID_FLOOR).map(drop)
        }),
        ("cancel_order_with, bad ext_operator", Sends::Succeed, true, |client| {
            client.cancel_order_with(41, &bad_operator()).map(drop)
        }),
        ("global_cancel_with, bad ext_operator", Sends::Succeed, true, |client| {
            client.global_cancel_with(&bad_operator())
        }),
        ("builder submit, negative quantity", Sends::Succeed, true, |client| {
            client.order(&Contract::stock("AAPL").build()).buy(-100).market().submit().map(drop)
        }),
        ("builder analyze, negative quantity", Sends::Succeed, true, |client| {
            client.order(&Contract::stock("AAPL").build()).buy(-100).market().analyze().map(drop)
        }),
        ("exercise_options, zero quantity", Sends::Succeed, true, |client| {
            client.exercise_options(&Contract::stock("AAPL").build()).exercise(0).submit().map(drop)
        }),
        (
            "bracket submit_all, a later leg's id in the request range",
            Sends::Succeed,
            true,
            |client| {
                client.id_manager().raise_order_id(OrderId::from(REQUEST_ID_FLOOR - 1));
                let contract = Contract::stock("AAPL").build();
                client
                    .order(&contract)
                    .buy(1)
                    .bracket()
                    .entry_limit(1.0)
                    .take_profit(2.0)
                    .stop_loss(0.5)
                    .submit_all()
                    .map(drop)
            },
        ),
        ("submit_order, write fails", Sends::Fail, false, |client| {
            client.submit_order(41, &Contract::stock("AAPL").build(), &limit())
        }),
        ("global_cancel, write fails", Sends::Fail, false, |client| client.global_cancel()),
        ("submit_oca_orders, write fails", Sends::Fail, false, |client| {
            let orders = vec![(Contract::stock("AAPL").build(), limit()), (Contract::stock("MSFT").build(), limit())];
            client.submit_oca_orders("G", OcaType::CancelWithBlock, orders).map(drop)
        }),
        ("submit_order, refused while reconnecting", Sends::Refuse, true, |client| {
            client.submit_order(41, &Contract::stock("AAPL").build(), &limit())
        }),
        (
            "submit_oca_orders, first send refused while reconnecting",
            Sends::Refuse,
            true,
            |client| {
                let orders = vec![(Contract::stock("AAPL").build(), limit()), (Contract::stock("MSFT").build(), limit())];
                client.submit_oca_orders("G", OcaType::CancelWithBlock, orders).map(drop)
            },
        ),
    ];

    for (name, sends, unsent, call) in cases {
        let stub = MessageBusStub::default();
        let message_bus = Arc::new(match sends {
            Sends::Succeed => stub,
            Sends::Fail => stub.with_failing_sends(),
            Sends::Refuse => stub.with_refused_sends(),
        });
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let err = call(&client).expect_err(name);

        let written = request_message_count(&message_bus);
        assert_eq!(err.is_unsent(), unsent, "{name}: {err:?}");
        assert!(!err.is_unsent() || written == 0, "{name}: unsent, yet {written} requests written");
        assert_eq!(written, usize::from(matches!(sends, Sends::Fail)), "{name}: requests written");
    }
}
