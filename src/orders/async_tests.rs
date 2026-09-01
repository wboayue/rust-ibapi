use super::*;
use crate::common::test_utils::helpers::{
    assert_request, assert_tws_error_message, create_test_client, create_test_client_with_ordered_proto_responses, decode_request_proto,
    proto_error_response, proto_response, request_message_count, TEST_REQ_ID_FIRST,
};
use crate::contracts::{Contract, SecurityType};
use crate::contracts::{Currency, Exchange, OptionRight, Symbol};
use crate::messages::IncomingMessages;
use crate::orders::OrderStatusKind;
use crate::stubs::MessageBusStub;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::orders::{
    cancel_order_request, commission_report, completed_order, completed_orders_end, completed_orders_request, execution_data, execution_data_end,
    executions_request, global_cancel_request, next_valid_order_id_request, open_order, open_order_end, open_orders_request, order_status,
    place_order_request,
};
use crate::testdata::builders::ResponseProtoEncoder;
use crate::{server_versions, Client};
use futures::StreamExt;
use std::sync::Arc;
use tokio::time::Duration;

#[tokio::test]
async fn test_place_order() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::OpenOrder,
            open_order()
                .order_id(1)
                .contract_id(637533641)
                .symbol("ES")
                .security_type("FUT")
                .last_trade_date_or_contract_month("20250919")
                .multiplier("50")
                .exchange("CME")
                .local_symbol("ESU5")
                .trading_class("ES")
                .total_quantity(1.0)
                .order_type("LMT")
                .limit_price(Some(5800.0))
                .perm_id(2126726143)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::OrderStatus,
            order_status()
                .order_id(1)
                .status(OrderStatusKind::Submitted)
                .filled(0.0)
                .remaining(1.0)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::ExecutionData,
            execution_data()
                .request_id(1)
                .order_id(1)
                .contract_id(637533641)
                .symbol("ES")
                .security_type("FUT")
                .exchange("CME")
                .execution_id("0001f4e5.58bbad52.01.01")
                .shares(1.0)
                .price(5800.0)
                .perm_id(2126726143)
                .last_liquidity(1)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::CommissionsReport,
            commission_report()
                .execution_id("0001f4e5.58bbad52.01.01")
                .commission(2.25)
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract {
        symbol: Symbol::from("ES"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("CME"),
        currency: Currency::from("USD"),
        local_symbol: "ESU5".to_string(),
        ..Default::default()
    };
    let mut order = order_builder::limit_order(Action::Buy, 1.0, 5800.0);
    order.order_id = 1;

    let mut subscription = client.place_order(1, &contract, &order).await.expect("failed to place order");

    let open_order = subscription.next().await;
    assert!(
        matches!(open_order, Some(Ok(SubscriptionItem::Data(PlaceOrder::OpenOrder(_))))),
        "Expected PlaceOrder::OpenOrder, got {:?}",
        open_order
    );

    let order_status = subscription.next().await;
    assert!(
        matches!(order_status, Some(Ok(SubscriptionItem::Data(PlaceOrder::OrderStatus(_))))),
        "Expected PlaceOrder::OrderStatus, got {:?}",
        order_status
    );

    let execution_data = subscription.next().await;
    assert!(
        matches!(execution_data, Some(Ok(SubscriptionItem::Data(PlaceOrder::ExecutionData(_))))),
        "Expected PlaceOrder::ExecutionData, got {:?}",
        execution_data
    );

    let commission_report = subscription.next().await;
    assert!(
        matches!(commission_report, Some(Ok(SubscriptionItem::Data(PlaceOrder::CommissionReport(_))))),
        "Expected PlaceOrder::CommissionReport, got {:?}",
        commission_report
    );

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &place_order_request().order_id(1).contract(&contract).order(&order));
}

// Drives the real async place_order path and decodes the captured wire bytes to confirm
// hedge_max_size rides the outbound PlaceOrderRequest proto (docs/rules/testing/exercise-production-code.md).
#[tokio::test]
async fn place_order_encodes_hedge_max_size() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::HEDGE_MAX_SIZE);

    let contract = Contract::stock("TSLA").build();
    let mut order = order_builder::market_order(Action::Buy, 100.0);
    order.hedge_max_size = Some(500);

    let _subscription = client.place_order(20, &contract, &order).await.expect("place_order should succeed");

    assert_eq!(request_message_count(&message_bus), 1);
    let request: crate::proto::PlaceOrderRequest = decode_request_proto(&message_bus, 0);
    let proto_order = request.order.expect("request carries an order");
    assert_eq!(proto_order.hedge_max_size, Some(500));
}

// Placing an order with hedge_max_size against a server below the gate is rejected
// before anything is sent (docs/rules/testing/exercise-production-code.md — real verify path).
#[tokio::test]
async fn place_order_rejects_hedge_max_size_below_gate() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::HEDGE_MAX_SIZE - 1);

    let contract = Contract::stock("TSLA").build();
    let mut order = order_builder::market_order(Action::Buy, 100.0);
    order.hedge_max_size = Some(500);

    match client.place_order(20, &contract, &order).await {
        Err(crate::Error::ServerVersion(required, _, _)) => assert_eq!(required, server_versions::HEDGE_MAX_SIZE),
        Err(other) => panic!("expected ServerVersion error, got {other:?}"),
        Ok(_) => panic!("expected place_order to be rejected below the hedge_max_size gate"),
    }
    assert_eq!(request_message_count(&message_bus), 0);
}

#[tokio::test]
async fn test_cancel_order() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::OrderStatus,
        order_status()
            .order_id(1)
            .status(OrderStatusKind::Cancelled)
            .filled(0.0)
            .remaining(1.0)
            .perm_id(2126726143)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let mut subscription = client.cancel_order(1, "").await.expect("failed to cancel order");

    let cancel_response = subscription.next().await;
    assert!(
        matches!(cancel_response, Some(Ok(SubscriptionItem::Data(CancelOrder::OrderStatus(_))))),
        "Expected CancelOrder::OrderStatus, got {:?}",
        cancel_response
    );

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &cancel_order_request().order_id(1));
}

#[tokio::test]
async fn test_cancel_order_delivers_202_as_non_terminal_notice() {
    // The cancellation confirmation (202) must arrive as a Notice and leave
    // the stream open: the OrderStatus frame queued behind it still arrives.
    let order_id = 1;
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_error_response(order_id, crate::messages::ORDER_CANCELLED_CODE, "Order Canceled - reason:"),
        proto_response(
            IncomingMessages::OrderStatus,
            order_status().order_id(order_id).status(OrderStatusKind::Cancelled).encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let mut subscription = client.cancel_order(order_id, "").await.expect("cancel_order failed");

    match subscription.next().await {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert!(notice.is_cancellation(), "expected cancellation notice, got {notice:?}");
        }
        other => panic!("expected non-terminal cancellation Notice, got {other:?}"),
    }

    match subscription.next().await {
        Some(Ok(SubscriptionItem::Data(CancelOrder::OrderStatus(status)))) => {
            assert_eq!(status.status, OrderStatusKind::Cancelled);
        }
        other => panic!("expected OrderStatus after the 202 notice, got {other:?}"),
    }
}

#[tokio::test]
async fn test_open_orders() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::OpenOrder,
            open_order()
                .order_id(1)
                .contract_id(637533641)
                .symbol("ES")
                .security_type("FUT")
                .last_trade_date_or_contract_month("20250919")
                .multiplier("50")
                .exchange("CME")
                .local_symbol("ESU5")
                .trading_class("ES")
                .total_quantity(1.0)
                .order_type("LMT")
                .limit_price(Some(5800.0))
                .perm_id(2126726143)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::OrderStatus,
            order_status()
                .order_id(1)
                .status(OrderStatusKind::Submitted)
                .filled(0.0)
                .remaining(1.0)
                .perm_id(2126726143)
                .encode_proto(),
        ),
        proto_response(IncomingMessages::OpenOrderEnd, open_order_end().encode_proto()),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let mut subscription = client.open_orders().await.expect("failed to get open orders");

    let order_data = subscription.next().await;
    assert!(
        matches!(order_data, Some(Ok(SubscriptionItem::Data(Orders::OrderData(_))))),
        "Expected Orders::OrderData, got {:?}",
        order_data
    );

    let order_status = subscription.next().await;
    assert!(
        matches!(order_status, Some(Ok(SubscriptionItem::Data(Orders::OrderStatus(_))))),
        "Expected Orders::OrderStatus, got {:?}",
        order_status
    );

    let end_response = subscription.next().await;
    assert!(end_response.is_none(), "Expected None (end of stream), got {:?}", end_response);

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &open_orders_request());
}

#[tokio::test]
async fn test_completed_orders() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::CompletedOrder,
            completed_order()
                .contract_id(637533641)
                .symbol("ES")
                .security_type("FUT")
                .last_trade_date_or_contract_month("20250919")
                .multiplier("50")
                .exchange("CME")
                .local_symbol("ESU5")
                .trading_class("ES")
                .total_quantity(1.0)
                .order_type("LMT")
                .limit_price(Some(5800.0))
                .perm_id(2126726143)
                .status(OrderStatusKind::Cancelled)
                .completed_time("20250708 02:34:46 America/New_York")
                .completed_status("Cancelled by Trader")
                .encode_proto(),
        ),
        proto_response(IncomingMessages::CompletedOrdersEnd, completed_orders_end().encode_proto()),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::COMPLETED_ORDERS);

    let mut subscription = client.completed_orders(true).await.expect("failed to get completed orders");

    let next = subscription.next().await;
    assert!(
        matches!(next, Some(Ok(SubscriptionItem::Data(Orders::OrderData(_))))),
        "Expected Orders::OrderData, got {:?}",
        next
    );

    let end_response = subscription.next().await;
    assert!(end_response.is_none(), "Expected None (end of stream), got {:?}", end_response);

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &completed_orders_request().api_only(true));
}

#[tokio::test]
async fn test_executions() {
    // All three responses are emitted only in response to RequestExecutions (gate 201)
    // or PlaceOrder (gate 203) — both ≤ floor 203, so the server always emits them as
    // proto and the text branch in the decoders is gone.
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::ExecutionData,
            execution_data()
                .request_id(TEST_REQ_ID_FIRST)
                .order_id(1)
                .contract_id(637533641)
                .symbol("ES")
                .security_type("FUT")
                .exchange("CME")
                .execution_id("0001f4e5.58bbad52.01.01")
                .shares(1.0)
                .price(5800.0)
                .perm_id(2126726143)
                .last_liquidity(1)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::CommissionsReport,
            commission_report()
                .execution_id("0001f4e5.58bbad52.01.01")
                .commission(2.25)
                .encode_proto(),
        ),
        proto_response(IncomingMessages::ExecutionDataEnd, execution_data_end().encode_proto()),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let filter = ExecutionFilter::default();
    let mut subscription = client.executions(filter).await.expect("failed to get executions");

    let exec_data = subscription.next().await;
    assert!(
        matches!(exec_data, Some(Ok(SubscriptionItem::Data(Executions::ExecutionData(_))))),
        "Expected Executions::ExecutionData, got {:?}",
        exec_data
    );

    let commission = subscription.next().await;
    assert!(
        matches!(commission, Some(Ok(SubscriptionItem::Data(Executions::CommissionReport(_))))),
        "Expected Executions::CommissionReport, got {:?}",
        commission
    );

    let end_response = subscription.next().await;
    assert!(end_response.is_none(), "Expected None (end of stream), got {:?}", end_response);

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &executions_request().request_id(TEST_REQ_ID_FIRST).filter(ExecutionFilter::default()),
    );
}

#[tokio::test]
async fn test_submit_order() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract {
        symbol: Symbol::from("ES"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("CME"),
        currency: Currency::from("USD"),
        local_symbol: "ESU5".to_string(),
        ..Default::default()
    };
    let mut order = order_builder::limit_order(Action::Buy, 1.0, 5800.0);
    order.order_id = 2;

    client.submit_order(2, &contract, &order).await.expect("failed to submit order");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &place_order_request().order_id(2).contract(&contract).order(&order));
}

#[tokio::test]
async fn test_exercise_options() {
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
        right: Some(OptionRight::Call),
        ..Default::default()
    };

    let mut subscription = client
        .exercise_options(&contract, ExerciseAction::Exercise, 1, "", false, None)
        .await
        .expect("failed to exercise options");

    let exercise_response = subscription.next().await;
    assert!(
        matches!(exercise_response, Some(Ok(SubscriptionItem::Data(ExerciseOptions::OpenOrder(_))))),
        "Expected ExerciseOptions::OpenOrder, got {:?}",
        exercise_response
    );

    assert_eq!(request_message_count(&message_bus), 1);
}

#[tokio::test]
async fn test_next_valid_order_id() {
    let next_valid_id_proto = crate::proto::NextValidId { order_id: Some(123) };
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::NextValidId,
        prost::Message::encode_to_vec(&next_valid_id_proto),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let initial_order_id = client.next_order_id();

    let order_id = client.next_valid_order_id().await.expect("failed to get next valid order id");

    assert_eq!(order_id, 123, "Expected order ID 123");
    assert_eq!(client.next_order_id(), 123, "Client's order ID should be updated to 123");
    assert_ne!(client.next_order_id(), initial_order_id, "Client's order ID should have changed");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &next_valid_order_id_request());
}

#[tokio::test]
async fn test_order_update_stream() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::OrderStatus,
            order_status()
                .order_id(100)
                .status(OrderStatusKind::Submitted)
                .filled(0.0)
                .remaining(1.0)
                .perm_id(2126726143)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::ExecutionData,
            execution_data()
                .request_id(1)
                .order_id(1)
                .contract_id(637533641)
                .symbol("ES")
                .security_type("FUT")
                .exchange("CME")
                .execution_id("0001f4e5.58bbad52.01.01")
                .shares(1.0)
                .price(5800.0)
                .perm_id(2126726143)
                .last_liquidity(1)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::CommissionsReport,
            commission_report()
                .execution_id("0001f4e5.58bbad52.01.01")
                .commission(2.25)
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let mut stream = client.order_update_stream().await.unwrap();

    let Some(Ok(SubscriptionItem::Data(update))) = stream.next().await else {
        panic!("expected Data");
    };
    assert!(matches!(update, OrderUpdate::OrderStatus(_)));

    let Some(Ok(SubscriptionItem::Data(update))) = stream.next().await else {
        panic!("expected Data");
    };
    assert!(matches!(update, OrderUpdate::ExecutionData(_)));

    let Some(Ok(SubscriptionItem::Data(update))) = stream.next().await else {
        panic!("expected Data");
    };
    assert!(matches!(update, OrderUpdate::CommissionReport(_)));
}

#[tokio::test]
async fn test_order_update_stream_survives_unknown_status() {
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
    let mut stream = client.order_update_stream().await.unwrap();

    match stream.next().await {
        Some(Ok(SubscriptionItem::Data(OrderUpdate::OrderStatus(s)))) => {
            assert_eq!(s.status, OrderStatusKind::Unknown("PendingReplace".into()));
        }
        other => panic!("expected OrderStatus with Unknown status, got {other:?}"),
    }
    match stream.next().await {
        Some(Ok(SubscriptionItem::Data(OrderUpdate::OrderStatus(s)))) => {
            assert_eq!(s.status, OrderStatusKind::Filled);
        }
        other => panic!("stream did not survive the unknown status, got {other:?}"),
    }
}

#[tokio::test]
async fn test_order_update_stream_already_subscribed() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let stream1 = client.order_update_stream().await;
    assert!(stream1.is_ok(), "failed to create first order update stream");

    let stream2 = client.order_update_stream().await;
    assert!(stream2.is_err(), "second order update stream should fail with AlreadySubscribed");
    assert!(
        matches!(stream2.err().unwrap(), Error::AlreadySubscribed),
        "expected AlreadySubscribed error"
    );
}

#[tokio::test]
async fn test_order_update_stream_drop_releases_subscription() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let stream1 = client.order_update_stream().await.expect("failed to create initial order update stream");
    drop(stream1);

    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(10)).await;

    let _stream2 = client.order_update_stream().await.expect("should be re-subscribable after drop");
}

#[tokio::test]
async fn test_global_cancel() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::REQ_GLOBAL_CANCEL);

    client.global_cancel().await.expect("failed to send global cancel");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &global_cancel_request());
}

// Covers the `Client::order` delegate only. Field-by-field builder semantics are
// asserted in orders/builder/tests.rs against a mock client; what is unique here
// is that the entry point binds the real `Client` and the given contract.
#[tokio::test]
async fn order_entry_point_builds_order() {
    let (client, _) = create_test_client();
    let contract = Contract::stock("AAPL").build();

    let order = client.order(&contract).buy(100).limit(50.0).build().expect("build failed");

    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.limit_price, Some(50.0));
}

// Async twin of `analyze_surfaces_rejected_what_if_order`; see that test for why
// this path is otherwise uncovered.
#[tokio::test]
async fn analyze_surfaces_rejected_what_if_order() {
    let (client, _bus) =
        create_test_client_with_ordered_proto_responses(vec![proto_error_response(9000, 201, "Order rejected - reason:Insufficient buying power")]);
    let contract = Contract::stock("AAPL").build();

    let err = client
        .order(&contract)
        .buy(100)
        .limit(50.0)
        .analyze()
        .await
        .expect_err("a rejected what-if order must surface the rejection");
    assert_tws_error_message(err, 201, "Insufficient buying power");
}

// Async twin of `analyze_returns_order_state_for_the_matching_order`; see the
// blocking test for why this replaced a mock-client shadow.
#[tokio::test]
async fn analyze_returns_order_state_for_the_matching_order() {
    let (client, bus) = create_test_client_with_ordered_proto_responses(vec![proto_response(
        IncomingMessages::OpenOrder,
        open_order().order_id(90).status(OrderStatusKind::PreSubmitted).encode_proto(),
    )]);
    client.set_next_order_id(90);
    let contract = Contract::stock("AAPL").build();

    let state = client
        .order(&contract)
        .buy(100)
        .limit(50.0)
        .analyze()
        .await
        .expect("analyze should succeed");
    assert_eq!(state.status, OrderStatusKind::PreSubmitted);

    let request: crate::proto::PlaceOrderRequest = decode_request_proto(&bus, 0);
    assert_eq!(request.order.expect("request carries an order").what_if, Some(true));
}

#[tokio::test]
async fn analyze_reports_end_of_stream_when_no_order_arrives() {
    let (client, _bus) = create_test_client_with_ordered_proto_responses(vec![]);
    let contract = Contract::stock("AAPL").build();

    let result = client.order(&contract).buy(100).limit(50.0).analyze().await;
    assert!(matches!(result, Err(Error::UnexpectedEndOfStream)), "got {result:?}");
}

// The `submit` family, against `Client::stubbed` rather than a mock client that
// carried its own copy of each method. The shadows dated from before the
// builder had a real seam to test against; #735 moved `analyze` off its shadow
// after one of them turned out to carry the very bug the PR was fixing.

#[tokio::test]
async fn submit_assigns_the_next_order_id_and_sends_the_order() {
    let (client, bus) = create_test_client();
    client.set_next_order_id(100);
    let contract = Contract::stock("AAPL").build();

    let order_id = client
        .order(&contract)
        .buy(100)
        .limit(50.0)
        .submit()
        .await
        .expect("submit should succeed");
    assert_eq!(order_id.value(), 100);

    assert_eq!(request_message_count(&bus), 1);
    let request: crate::proto::PlaceOrderRequest = decode_request_proto(&bus, 0);
    assert_eq!(request.order_id, Some(100));
    let order = request.order.expect("request carries an order");
    assert_eq!(order.action.as_deref(), Some("BUY"));
    assert_eq!(order.order_type.as_deref(), Some("LMT"));
    assert_eq!(order.lmt_price, Some(50.0));
    assert!(!order.what_if.unwrap_or_default(), "submit is not a what-if order");
}

#[tokio::test]
async fn submit_rejects_an_invalid_order_before_sending() {
    let (client, bus) = create_test_client();
    let contract = Contract::stock("AAPL").build();

    let err = client
        .order(&contract)
        .buy(-100)
        .market()
        .submit()
        .await
        .expect_err("a negative quantity is invalid");
    assert!(err.to_string().contains("Invalid quantity"), "got {err}");
    assert_eq!(request_message_count(&bus), 0, "an invalid order must not reach the wire");
}

#[tokio::test]
async fn submit_all_reserves_three_ids_and_wires_the_bracket() {
    let (client, bus) = create_test_client();
    client.set_next_order_id(200);
    let contract = Contract::stock("AAPL").build();

    let ids = client
        .order(&contract)
        .buy(100)
        .good_till_cancel()
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0)
        .submit_all()
        .await
        .expect("bracket submission should succeed");

    assert_eq!((ids.parent.value(), ids.take_profit.value(), ids.stop_loss.value()), (200, 201, 202));
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
        vec![0, 200, 200]
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

#[tokio::test]
async fn submit_oca_orders_numbers_each_order_and_keeps_the_group() {
    let (client, bus) = create_test_client();
    client.set_next_order_id(300);
    let apple = Contract::stock("AAPL").build();
    let microsoft = Contract::stock("MSFT").build();

    let first = client
        .order(&apple)
        .buy(100)
        .limit(50.0)
        .oca_group("TestOCA", 1)
        .build_order()
        .expect("order should build");
    let second = client
        .order(&microsoft)
        .buy(100)
        .limit(45.0)
        .oca_group("TestOCA", 1)
        .build_order()
        .expect("order should build");

    let ids = client
        .submit_oca_orders(vec![(apple, first), (microsoft, second)])
        .await
        .expect("OCA submission should succeed");

    assert_eq!(ids.iter().map(|id| id.value()).collect::<Vec<_>>(), vec![300, 301]);
    assert_eq!(request_message_count(&bus), 2);

    for i in 0..2 {
        let request: crate::proto::PlaceOrderRequest = decode_request_proto(&bus, i);
        let order = request.order.expect("request carries an order");
        assert_eq!(order.oca_group.as_deref(), Some("TestOCA"));
        assert_eq!(order.oca_type, Some(1));
    }
}

#[tokio::test]
async fn build_order_does_not_reach_the_wire() {
    let (client, bus) = create_test_client();
    let contract = Contract::stock("AAPL").build();

    let order = client
        .order(&contract)
        .sell(100)
        .trailing_stop(5.0, 95.0)
        .build_order()
        .expect("order should build");

    assert_eq!(order.order_type, "TRAIL");
    assert_eq!(order.trailing_percent, Some(5.0));
    assert_eq!(order.trail_stop_price, Some(95.0));
    assert_eq!(request_message_count(&bus), 0);
}

#[tokio::test]
async fn submit_carries_algo_parameters_to_the_wire() {
    // The mock-client shadow asserted this against a captured `Order` struct;
    // at the real seam the assertion is what TWS receives.
    let (client, bus) = create_test_client();
    let contract = Contract::stock("AAPL").build();

    client
        .order(&contract)
        .buy(100)
        .limit(50.0)
        .algo("VWAP")
        .algo_param("startTime", "09:30:00")
        .algo_param("endTime", "16:00:00")
        .submit()
        .await
        .expect("submit should succeed");

    let order = decode_request_proto::<crate::proto::PlaceOrderRequest>(&bus, 0)
        .order
        .expect("request carries an order");
    assert_eq!(order.algo_strategy.as_deref(), Some("VWAP"));
    assert_eq!(order.algo_params.get("startTime").map(String::as_str), Some("09:30:00"));
    assert_eq!(order.algo_params.get("endTime").map(String::as_str), Some("16:00:00"));
}

#[tokio::test]
async fn submit_rejects_a_non_finite_price_before_sending() {
    let (client, bus) = create_test_client();
    let contract = Contract::stock("AAPL").build();

    let err = client
        .order(&contract)
        .buy(100)
        .limit(f64::NAN)
        .submit()
        .await
        .expect_err("NaN is not a price");
    assert!(err.to_string().contains("Invalid price"), "got {err}");
    assert_eq!(request_message_count(&bus), 0);
}
