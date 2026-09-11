use std::time::{Duration, Instant};

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::order_builder::PeggedToBenchmark;
use ibapi::orders::{Action, BracketOrderIds, CancelOrder, ExecutionFilter, Order, OrderId, OrderStatusKind, PlaceOrder};
use ibapi::subscriptions::sync::Subscription;
use ibapi::subscriptions::SubscriptionItem;
use ibapi::{Error, NoticeCategory};
use ibapi_test::{rate_limit, require_globex_open, yyyymmdd_from_now, ClientId, GATEWAY};
use serial_test::serial;

fn connect() -> (Client, ClientId) {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");
    (client, client_id)
}

fn limit_order(action: Action, quantity: f64, price: f64) -> Order {
    Order {
        action,
        total_quantity: quantity,
        order_type: "LMT".to_string(),
        limit_price: Some(price),
        ..Default::default()
    }
}

fn place_bracket_order(client: &Client, contract: &Contract) -> BracketOrderIds {
    rate_limit();
    let ids = client
        .order(contract)
        .buy(1)
        .bracket()
        .entry_limit(1.0)
        .take_profit(300.0)
        .stop_loss(0.50)
        .submit_all()
        .expect("bracket order submit failed");
    std::thread::sleep(Duration::from_millis(500));
    ids
}

fn place_and_cleanup(client: &Client, contract: &Contract) -> OrderId {
    rate_limit();
    let order_id = client
        .order(contract)
        .buy(1)
        .limit(1.0)
        .submit()
        .expect("placing order after cancel should succeed");
    assert!(order_id.0 > 0, "new order id should be positive");
    rate_limit();
    let _ = client.cancel_order(order_id.0, "").expect("cleanup cancel failed");
    order_id
}

#[test]
#[serial(orders)]
fn next_valid_order_id() {
    let (client, _client_id) = connect();

    rate_limit();
    let id = client.next_valid_order_id().expect("next_valid_order_id failed");
    assert!(id > 0, "order id should be positive");
}

#[test]
#[serial(orders)]
fn open_orders() {
    let (client, _client_id) = connect();

    rate_limit();
    let subscription = client.open_orders().expect("open_orders failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn all_open_orders() {
    let (client, _client_id) = connect();

    rate_limit();
    let subscription = client.all_open_orders().expect("all_open_orders failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn completed_orders() {
    let (client, _client_id) = connect();

    rate_limit();
    let subscription = client.completed_orders(false).expect("completed_orders failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn completed_orders_api_only() {
    let (client, _client_id) = connect();

    rate_limit();
    let subscription = client.completed_orders(true).expect("completed_orders api_only failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn place_limit_buy() {
    let (client, _client_id) = connect();

    let contract = Contract::stock("AAPL").build();
    let order = limit_order(Action::Buy, 1.0, 1.0); // Far below market

    rate_limit();
    let order_id = client.next_order_id();
    let subscription = client.place_order(order_id, &contract, &order).expect("place_order failed");

    // Should receive order status
    let item = subscription.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected order status update");

    // Cancel the order
    rate_limit();
    let _ = client.cancel_order(order_id, "").expect("cancel_order failed");
}

#[test]
#[serial(orders)]
fn place_limit_sell() {
    let (client, _client_id) = connect();

    let contract = Contract::stock("AAPL").build();
    let order = limit_order(Action::Sell, 1.0, 9999.0); // Far above market

    rate_limit();
    let order_id = client.next_order_id();
    let subscription = client.place_order(order_id, &contract, &order).expect("place_order failed");

    let item = subscription.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected order status update");

    rate_limit();
    let _ = client.cancel_order(order_id, "").expect("cancel_order failed");
}

#[test]
#[serial(orders)]
fn cancel_order_succeeds() {
    let (client, _client_id) = connect();

    let contract = Contract::stock("AAPL").build();
    let order = limit_order(Action::Buy, 1.0, 1.0);

    rate_limit();
    let order_id = client.next_order_id();
    let _subscription = client.place_order(order_id, &contract, &order).expect("place_order failed");

    std::thread::sleep(Duration::from_millis(500));

    rate_limit();
    let cancel_sub = client.cancel_order(order_id, "").expect("cancel_order failed");
    let _item = cancel_sub.next_timeout(Duration::from_secs(10));
}

#[test]
#[serial(orders)]
fn global_cancel() {
    let (client, _client_id) = connect();

    rate_limit();
    client.global_cancel().expect("global_cancel failed");
}

#[test]
#[serial(orders)]
fn order_builder_limit() {
    let (client, _client_id) = connect();

    let contract = Contract::stock("AAPL").build();

    rate_limit();
    let order_id = client.order(&contract).buy(1).limit(1.0).submit().expect("order builder submit failed");

    assert!(order_id.0 > 0, "order id should be positive");

    // Cancel the placed order
    rate_limit();
    let _ = client.cancel_order(order_id.0, "").expect("cancel_order failed");
}

// Regression test for https://github.com/wboayue/rust-ibapi/issues/426
#[test]
#[serial(orders)]
fn cancel_bracket_order() {
    let (client, _client_id) = connect();
    let contract = Contract::stock("AAPL").build();
    let ids = place_bracket_order(&client, &contract);

    rate_limit();
    let cancel_sub = client.cancel_order(ids.parent.0, "").expect("cancel_order failed");

    // TWS may push the parent's current working status (Submitted / PendingCancel)
    // before the terminal Cancelled confirmation, so drain status updates until
    // cancellation is observed rather than asserting on the first item.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut cancelled = false;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        let Some(item) = cancel_sub.next_timeout(remaining) else {
            break; // timed out
        };
        match item.expect("cancel subscription error") {
            SubscriptionItem::Data(CancelOrder::OrderStatus(s)) => {
                if matches!(
                    s.status,
                    OrderStatusKind::Cancelled | OrderStatusKind::ApiCancelled | OrderStatusKind::Inactive
                ) {
                    cancelled = true;
                    break;
                }
                // ignore transitional statuses (Submitted, PreSubmitted, PendingCancel, ...)
            }
            SubscriptionItem::Notice(n) => {
                // Only a cancellation confirmation (code 202) counts; an unrelated
                // warning could otherwise mask a failed cancel.
                if n.category() == NoticeCategory::Cancellation {
                    cancelled = true;
                    break;
                }
            }
        }
    }
    assert!(cancelled, "parent order should be cancelled");
}

// Regression test for https://github.com/wboayue/rust-ibapi/issues/426
#[test]
#[serial(orders)]
fn cancel_bracket_order_then_place_new_order() {
    let (client, _client_id) = connect();
    let contract = Contract::stock("AAPL").build();
    let ids = place_bracket_order(&client, &contract);

    rate_limit();
    let _cancel_sub = client.cancel_order(ids.parent.0, "").expect("cancel_order failed");
    std::thread::sleep(Duration::from_millis(500));

    place_and_cleanup(&client, &contract);
}

// Regression test for https://github.com/wboayue/rust-ibapi/issues/426
#[test]
#[serial(orders)]
fn global_cancel_bracket_order_then_place_new_order() {
    let (client, _client_id) = connect();
    let contract = Contract::stock("AAPL").build();
    place_bracket_order(&client, &contract);

    rate_limit();
    client.global_cancel().expect("global_cancel failed");
    std::thread::sleep(Duration::from_millis(500));

    place_and_cleanup(&client, &contract);
}

#[test]
#[serial(orders)]
fn executions_returns_subscription() {
    let (client, _client_id) = connect();

    rate_limit();
    let subscription = client.executions(ExecutionFilter::default()).expect("executions failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn place_pegged_to_benchmark() {
    let (client, _client_id) = connect();

    let contract = Contract::stock("AAPL").build();

    rate_limit();
    let details = client.contract_details(&contract).expect("contract_details failed");
    let reference_id = details[0].contract.contract_id;

    let order = PeggedToBenchmark::new(Action::Buy, 1.0, 1.0)
        .reference_contract(reference_id, "ISLAND")
        .pegged_change_amount(0.01)
        .reference_change_amount(0.01)
        .stock_reference_price(1.0)
        .reference_range(0.5, 9999.0)
        .build()
        .expect("PeggedToBenchmark build failed");

    rate_limit();
    let order_id = client.next_order_id();
    let subscription = client.place_order(order_id, &contract, &order).expect("place_order failed");

    let mut acknowledged = false;
    while let Some(result) = subscription.next_timeout(Duration::from_secs(5)) {
        match result {
            Ok(SubscriptionItem::Data(PlaceOrder::OrderStatus(_) | PlaceOrder::OpenOrder(_))) => {
                acknowledged = true;
                break;
            }
            Ok(SubscriptionItem::Notice(notice)) => {
                if notice.message.contains("rejected") {
                    panic!("TWS rejected pegged-to-benchmark order: {}", notice.message);
                }
                acknowledged = true;
                break;
            }
            Ok(SubscriptionItem::Data(_)) => continue,
            Err(Error::Notice(n)) if n.code == 201 => panic!("TWS rejected pegged-to-benchmark order [201]: {}", n.message),
            Err(Error::Notice(_)) => {
                acknowledged = true;
                break;
            }
            Err(e) => panic!("subscription error: {e}"),
        }
    }
    assert!(acknowledged, "no acknowledgement from TWS within timeout");

    rate_limit();
    let _ = client.cancel_order(order_id, "");
}

fn market_order(action: Action, quantity: f64) -> Order {
    Order {
        action,
        total_quantity: quantity,
        order_type: "MKT".to_string(),
        ..Default::default()
    }
}

/// Front-month ES: the CME listing with the earliest last-trade date that is
/// still more than a week out, so a test never trades into expiry.
fn front_month_es(client: &Client) -> Contract {
    let query = Contract::futures("ES").on_exchange("CME").any_month().build();
    rate_limit();
    let details = client.contract_details(&query).expect("contract_details failed");
    let cutoff = yyyymmdd_from_now(7);
    details
        .into_iter()
        .map(|d| d.contract)
        .filter(|c| c.last_trade_date_or_contract_month > cutoff)
        .min_by(|a, b| a.last_trade_date_or_contract_month.cmp(&b.last_trade_date_or_contract_month))
        .expect("no ES listing beyond the cutoff")
}

/// Drain `sub` until both the `ExecutionData` and its `CommissionReport` have
/// arrived, and check they share an `execution_id`. Returns `Err` instead of
/// panicking so the caller can flatten the position before failing.
fn observe_execution_then_commission(sub: &Subscription<PlaceOrder>) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut execution_id: Option<String> = None;
    let mut commission_id: Option<String> = None;
    while execution_id.is_none() || commission_id.is_none() {
        match sub.next_timeout(deadline.saturating_duration_since(Instant::now())) {
            Some(Ok(SubscriptionItem::Data(PlaceOrder::ExecutionData(exec)))) => execution_id = Some(exec.execution.execution_id),
            Some(Ok(SubscriptionItem::Data(PlaceOrder::CommissionReport(report)))) => commission_id = Some(report.execution_id),
            Some(Ok(SubscriptionItem::Notice(notice))) if notice.is_order_rejection() => return Err(format!("order rejected: {notice}")),
            Some(Ok(_)) => continue,
            Some(Err(e)) => return Err(format!("subscription error: {e}")),
            // A commission that reached TWS's wire before its execution is dropped
            // by routing (#788), so that ordering shows up here as a missing commission.
            None => {
                return Err(format!(
                    "fill incomplete after 15s: execution={execution_id:?} commission={commission_id:?}"
                ))
            }
        }
    }
    if execution_id != commission_id {
        return Err(format!(
            "commission keyed to a different execution: {execution_id:?} vs {commission_id:?}"
        ));
    }
    Ok(())
}

/// Sell one contract at market and wait for the fill, so a failing assertion
/// never leaves the paper account long ES.
fn flatten(client: &Client, contract: &Contract) {
    rate_limit();
    let sell_id = client.next_order_id();
    let sell = client
        .place_order(sell_id, contract, &market_order(Action::Sell, 1.0))
        .expect("sell failed");
    let deadline = Instant::now() + Duration::from_secs(15);
    while let Some(item) = sell.next_timeout(deadline.saturating_duration_since(Instant::now())) {
        if let Ok(SubscriptionItem::Data(PlaceOrder::OrderStatus(status))) = item {
            if status.status == OrderStatusKind::Filled {
                return;
            }
        }
    }
    eprintln!("warning: ES sell did not report Filled within 15s; check the paper account");
}

/// A live fill delivers `ExecutionData` and then its `CommissionReport` on the
/// `place_order` subscription. The commission is routed by the `execution_id`
/// mapping the execution establishes, so this also guards against the
/// commission-first wire order described in #788.
#[test]
#[serial(orders)]
fn es_fill_delivers_execution_then_commission() {
    require_globex_open();
    let (client, _client_id) = connect();
    let contract = front_month_es(&client);

    rate_limit();
    let order_id = client.next_order_id();
    let sub = client
        .place_order(order_id, &contract, &market_order(Action::Buy, 1.0))
        .expect("buy failed");

    let outcome = observe_execution_then_commission(&sub);
    flatten(&client, &contract);
    if let Err(reason) = outcome {
        panic!("{reason}");
    }
}
