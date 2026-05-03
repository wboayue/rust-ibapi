use super::*;
use crate::common::test_utils::helpers::{assert_request, request_message_count, TEST_REQ_ID_FIRST};
use crate::contracts::{Contract, SecurityType};
use crate::contracts::{Currency, Exchange, Symbol};
use crate::orders::common::test_data::{COMPLETED_ORDER_ES_FUT_CANCELLED, EXERCISE_OPEN_ORDER_ES_FOP_SUBMITTED, OPEN_ORDER_ES_FUT_SUBMITTED};
use crate::stubs::MessageBusStub;
use crate::testdata::builders::orders::{
    cancel_order_request, commission_report, completed_orders_end, completed_orders_request, execution_data, execution_data_end, executions_request,
    global_cancel_request, next_valid_order_id_request, open_order_end, open_orders_request, order_status, place_order_request,
};
use crate::testdata::builders::ResponseEncoder;
use crate::{server_versions, Client};
use std::sync::Arc;
use tokio::time::Duration;

#[tokio::test]
async fn test_place_order() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![
        OPEN_ORDER_ES_FUT_SUBMITTED.to_owned(),
        order_status().order_id(1).status("Submitted").filled(0.0).remaining(1.0).encode_pipe(),
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
            .encode_pipe(),
        commission_report().execution_id("0001f4e5.58bbad52.01.01").commission(2.25).encode_pipe(),
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
        matches!(open_order, Some(Ok(PlaceOrder::OpenOrder(_)))),
        "Expected PlaceOrder::OpenOrder, got {:?}",
        open_order
    );

    let order_status = subscription.next().await;
    assert!(
        matches!(order_status, Some(Ok(PlaceOrder::OrderStatus(_)))),
        "Expected PlaceOrder::OrderStatus, got {:?}",
        order_status
    );

    let execution_data = subscription.next().await;
    assert!(
        matches!(execution_data, Some(Ok(PlaceOrder::ExecutionData(_)))),
        "Expected PlaceOrder::ExecutionData, got {:?}",
        execution_data
    );

    let commission_report = subscription.next().await;
    assert!(
        matches!(commission_report, Some(Ok(PlaceOrder::CommissionReport(_)))),
        "Expected PlaceOrder::CommissionReport, got {:?}",
        commission_report
    );

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &place_order_request().order_id(1).contract(&contract).order(&order));
}

#[tokio::test]
async fn test_cancel_order() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![order_status()
        .order_id(1)
        .status("Cancelled")
        .filled(0.0)
        .remaining(1.0)
        .perm_id(2126726143)
        .encode_pipe()]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let mut subscription = client.cancel_order(1, "").await.expect("failed to cancel order");

    let cancel_response = subscription.next().await;
    assert!(
        matches!(cancel_response, Some(Ok(CancelOrder::OrderStatus(_)))),
        "Expected CancelOrder::OrderStatus, got {:?}",
        cancel_response
    );

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &cancel_order_request().order_id(1));
}

#[tokio::test]
async fn test_open_orders() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![
        OPEN_ORDER_ES_FUT_SUBMITTED.to_owned(),
        order_status()
            .order_id(1)
            .status("Submitted")
            .filled(0.0)
            .remaining(1.0)
            .perm_id(2126726143)
            .encode_pipe(),
        open_order_end().encode_pipe(),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let mut subscription = client.open_orders().await.expect("failed to get open orders");

    let order_data = subscription.next().await;
    assert!(
        matches!(order_data, Some(Ok(Orders::OrderData(_)))),
        "Expected Orders::OrderData, got {:?}",
        order_data
    );

    let order_status = subscription.next().await;
    assert!(
        matches!(order_status, Some(Ok(Orders::OrderStatus(_)))),
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
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![
        COMPLETED_ORDER_ES_FUT_CANCELLED.to_owned(),
        completed_orders_end().encode_pipe(),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::COMPLETED_ORDERS);

    let mut subscription = client.completed_orders(true).await.expect("failed to get completed orders");

    let next = subscription.next().await;
    assert!(
        matches!(next, Some(Ok(Orders::OrderData(_)))),
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
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![
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
            .encode_pipe(),
        commission_report().execution_id("0001f4e5.58bbad52.01.01").commission(2.25).encode_pipe(),
        execution_data_end().encode_pipe(),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let filter = ExecutionFilter::default();
    let mut subscription = client.executions(filter).await.expect("failed to get executions");

    let exec_data = subscription.next().await;
    assert!(
        matches!(exec_data, Some(Ok(Executions::ExecutionData(_)))),
        "Expected Executions::ExecutionData, got {:?}",
        exec_data
    );

    let commission = subscription.next().await;
    assert!(
        matches!(commission, Some(Ok(Executions::CommissionReport(_)))),
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
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![EXERCISE_OPEN_ORDER_ES_FOP_SUBMITTED.to_owned()]));

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

    let mut subscription = client
        .exercise_options(&contract, ExerciseAction::Exercise, 1, "", false, None)
        .await
        .expect("failed to exercise options");

    let exercise_response = subscription.next().await;
    assert!(
        matches!(exercise_response, Some(Ok(ExerciseOptions::OpenOrder(_)))),
        "Expected ExerciseOptions::OpenOrder, got {:?}",
        exercise_response
    );

    assert_eq!(request_message_count(&message_bus), 1);
}

#[tokio::test]
async fn test_next_valid_order_id() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec!["4|1|123|".to_string()]));
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
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![
        order_status()
            .order_id(100)
            .status("Submitted")
            .filled(0.0)
            .remaining(1.0)
            .perm_id(2126726143)
            .encode_pipe(),
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
            .encode_pipe(),
        commission_report().execution_id("0001f4e5.58bbad52.01.01").commission(2.25).encode_pipe(),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let mut stream = client.order_update_stream().await.unwrap();

    let update = stream.next().await.unwrap().unwrap();
    assert!(matches!(update, OrderUpdate::OrderStatus(_)));

    let update = stream.next().await.unwrap().unwrap();
    assert!(matches!(update, OrderUpdate::ExecutionData(_)));

    let update = stream.next().await.unwrap().unwrap();
    assert!(matches!(update, OrderUpdate::CommissionReport(_)));
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

    client.order_update_stream().await.expect("should be re-subscribable after drop");
}

#[tokio::test]
async fn test_global_cancel() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::REQ_GLOBAL_CANCEL);

    client.global_cancel().await.expect("failed to send global cancel");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &global_cancel_request());
}
