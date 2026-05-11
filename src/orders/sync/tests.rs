use std::sync::Arc;

use crate::common::test_utils::helpers::{assert_request, proto_response, request_message_count};
use crate::contracts::{ComboLeg, Contract, Currency, Exchange, LegAction, SecurityType, Symbol};
use crate::messages::IncomingMessages;
use crate::orders::{Action, OrderStatusKind};
use crate::stubs::MessageBusStub;
use crate::testdata::builders::orders::{
    all_open_orders_request, auto_open_orders_request, cancel_order_request, commission_report, completed_order, completed_orders_end,
    completed_orders_request, execution_data, executions_request, global_cancel_request, next_valid_order_id_request, open_order,
    open_orders_request, order_status, place_order_request,
};
use crate::testdata::builders::{ResponseEncoder, ResponseProtoEncoder};

use super::*;
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
        proto_response(IncomingMessages::CommissionsReport, commission_report().encode_proto()),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract {
        symbol: Symbol::from("TSLA"),
        security_type: SecurityType::Stock,
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

    if let Some(Ok(PlaceOrder::CommissionReport(report))) = notifications.next_data() {
        assert_eq!(report.execution_id, "00025b46.63f8f39c.01.01", "report.execution_id");
        assert_eq!(report.commission, 1.0, "report.commission");
        assert_eq!(report.currency, "USD", "report.currency");
    } else {
        assert!(false, "message[6] expected a commission report notification");
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
    let results = client.cancel_order(order_id, "");

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
fn global_cancel() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let results = client.global_cancel();

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &global_cancel_request());
    assert!(results.is_ok(), "failed to cancel order: {}", results.err().unwrap());
}

#[test]
fn cancel_order_cme_tagging() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::OrderStatus,
        order_status()
            .order_id(41)
            .status(OrderStatusKind::Cancelled)
            .remaining(100.0)
            .perm_id(71270927)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::CME_TAGGING_FIELDS);

    let order_id = 41;
    let results = client.cancel_order(order_id, "");

    assert_request(&message_bus, 0, &cancel_order_request().order_id(order_id));
    assert!(results.is_ok(), "failed to cancel order: {}", results.err().unwrap());
}

#[test]
fn global_cancel_cme_tagging() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::CME_TAGGING_FIELDS);

    let results = client.global_cancel();

    assert_request(&message_bus, 0, &global_cancel_request());
    assert!(results.is_ok(), "failed to cancel order: {}", results.err().unwrap());
}

#[test]
fn next_valid_order_id() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec!["9|1|43||".to_owned()]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let results = client.next_valid_order_id();

    assert_request(&message_bus, 0, &next_valid_order_id_request());

    assert!(results.is_ok(), "failed to request next order id: {}", results.err().unwrap());
    assert_eq!(43, results.unwrap(), "next order id");
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

    let api_only = true;
    let results = client.completed_orders(api_only);

    assert_request(&message_bus, 0, &completed_orders_request().api_only(api_only));

    assert!(results.is_ok(), "failed to request completed orders: {}", results.err().unwrap());

    let results = results.unwrap();
    if let Some(Ok(Orders::OrderData(order_data))) = results.next_data() {
        assert_eq!(order_data.contract.symbol, Symbol::from("AAPL"), "contract.symbol");
        assert_eq!(order_data.contract.security_type, SecurityType::Stock, "contract.security_type");
        assert_eq!(order_data.order.action, Action::Buy, "order.action");
        assert_eq!(order_data.order.total_quantity, 100.0, "order.total_quantity");
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
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![
        crate::testdata::builders::orders::open_order_end().encode_pipe()
    ]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let results = client.open_orders();

    assert_request(&message_bus, 0, &open_orders_request());
    assert!(results.is_ok(), "failed to request open orders: {}", results.err().unwrap());
}

#[test]
fn all_open_orders() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![
        crate::testdata::builders::orders::open_order_end().encode_pipe()
    ]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let results = client.all_open_orders();

    assert_request(&message_bus, 0, &all_open_orders_request());
    assert!(results.is_ok(), "failed to request all open orders: {}", results.err().unwrap());
}

#[test]
fn auto_open_orders() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![
        crate::testdata::builders::orders::open_order_end().encode_pipe()
    ]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let api_only = true;
    let results = client.auto_open_orders(api_only);

    assert_request(&message_bus, 0, &auto_open_orders_request().auto_bind(api_only));
    assert!(results.is_ok(), "failed to request auto open orders: {}", results.err().unwrap());
}

#[test]
fn executions() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![
        crate::testdata::builders::orders::execution_data_end().encode_pipe(),
    ]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let filter = ExecutionFilter {
        client_id: Some(100),
        account_code: "xyz".to_owned(),
        time: "yyyymmdd hh:mm:ss EST".to_owned(),
        symbol: "TSLA".to_owned(),
        security_type: "STK".to_owned(),
        exchange: "ISLAND".to_owned(),
        side: "BUY".to_owned(),
        ..Default::default()
    };
    let expected_filter = ExecutionFilter {
        client_id: Some(100),
        account_code: "xyz".to_owned(),
        time: "yyyymmdd hh:mm:ss EST".to_owned(),
        symbol: "TSLA".to_owned(),
        security_type: "STK".to_owned(),
        exchange: "ISLAND".to_owned(),
        side: "BUY".to_owned(),
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

    assert!(results.is_ok(), "failed to request executions: {}", results.err().unwrap());
}

#[test]
fn encode_limit_order() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let order_id = 12;
    let contract = Contract {
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        local_symbol: "FGBL MAR 23".to_owned(),
        last_trade_date_or_contract_month: "202303".to_owned(),
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
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
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
            security_type: SecurityType::Spread,
            currency: Currency::from("USD"),
            exchange: Exchange::from("SMART"),
            combo_legs: vec![leg_1, leg_2],
            ..Contract::default()
        }
    };
    let order = order_builder::combo_market_order(Action::Sell, 150.0, true);

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
        security_type: SecurityType::FuturesOption,
        exchange: Exchange::from("CME"),
        currency: Currency::from("USD"),
        last_trade_date_or_contract_month: "20250919".to_string(),
        strike: 5800.0,
        right: "C".to_string(),
        ..Default::default()
    };

    let subscription = client
        .exercise_options(&contract, ExerciseAction::Exercise, 1, "", false, None)
        .expect("failed to exercise options");

    let exercise_response = subscription.next_data();
    assert!(
        matches!(exercise_response, Some(Ok(ExerciseOptions::OpenOrder(_)))),
        "Expected ExerciseOptions::OpenOrder, got {:?}",
        exercise_response
    );

    assert_eq!(request_message_count(&message_bus), 1);
}

#[test]
fn submit_order() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract {
        symbol: Symbol::from("AAPL"),
        security_type: SecurityType::Stock,
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
        proto_response(IncomingMessages::ExecutionData, execution_data().encode_proto()),
        proto_response(IncomingMessages::CommissionsReport, commission_report().encode_proto()),
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
        assert_eq!(exec_data.execution.side, "BOT", "execution.side");
    } else {
        assert!(false, "expected execution data notification");
    }

    if let Some(Ok(OrderUpdate::CommissionReport(report))) = notifications.next_data() {
        assert_eq!(report.execution_id, "00025b46.63f8f39c.01.01", "report.execution_id");
        assert_eq!(report.commission, 1.0, "report.commission");
        assert_eq!(report.currency, "USD", "report.currency");
    } else {
        assert!(false, "expected commission report notification");
    }
}

#[test]
fn order_update_stream_already_subscribed() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
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
