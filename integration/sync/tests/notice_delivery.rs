//! Live-gateway smoke tests for Notice delivery.
//!
//! Verifies the dispatcher → `Subscription<T>` end-to-end path against a real
//! IB Gateway / TWS. Synthesized routing tests in
//! `src/transport/sync_tests.rs` cover the same matrix strictly — these exist
//! as a release-time safety net against protocol-level regressions.
//!
//! The tests are tolerant by design: TWS doesn't always emit per-subscription
//! notices in the 2100-2169 range (most farm-status notices are global /
//! `request_id == -1` and currently log-only). Any observed
//! `SubscriptionItem::Notice` is logged; tests only fail when the subscription
//! itself misbehaves.

use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::{order_builder, Action};
use ibapi::subscriptions::SubscriptionItem;
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;

const TICK_BUDGET: Duration = Duration::from_secs(15);

/// Request market data for an invalid contract. TWS responds with code 200
/// ("No security definition has been found"), which the dispatcher must
/// classify as a terminal `Error`, surfacing on the subscription as
/// `Some(Err(_))`.
#[test]
fn invalid_contract_terminates_with_error() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("DOES_NOT_EXIST_XYZ").build();
    let subscription = client.market_data(&contract).subscribe().expect("market_data subscribe failed");

    let mut saw_error = false;
    for item in subscription.timeout_iter(TICK_BUDGET) {
        match item {
            Err(e) => {
                println!("subscription terminated by TWS: {e}");
                saw_error = true;
                break;
            }
            Ok(SubscriptionItem::Notice(notice)) => {
                println!("notice (ignored for this test): code={} message={}", notice.code, notice.message);
            }
            Ok(SubscriptionItem::Data(_)) => panic!("invalid contract should not yield data"),
        }
    }

    assert!(saw_error, "expected an Err for invalid contract");
}

/// Place a non-transmit outside-RTH market order. TWS commonly emits an
/// order-channel warning in the 2100-2169 range for outside-RTH orders during
/// closed market hours; this is the most reliable in-the-wild trigger for a
/// per-subscription `SubscriptionItem::Notice`. The assertion is best-effort:
/// IBKR session settings can suppress the warning.
#[test]
#[serial(orders)]
fn outside_rth_order_subscription_smoke() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let order_id = client.next_order_id();
    let mut order = order_builder::market_order(Action::Buy, 1.0);
    order.outside_rth = true;
    order.transmit = false;

    rate_limit();
    let subscription = client.place_order(order_id, &contract, &order).expect("place_order failed");

    let mut saw_notice = false;
    for item in subscription.timeout_iter(TICK_BUDGET).take(10) {
        match item {
            Ok(SubscriptionItem::Notice(notice)) => {
                println!("order notice: code={} message={}", notice.code, notice.message);
                saw_notice = true;
                break;
            }
            Ok(SubscriptionItem::Data(_)) => {}
            Err(e) => {
                println!("order subscription error: {e}");
                break;
            }
        }
    }

    if !saw_notice {
        eprintln!(
            "outside-RTH order did not surface a SubscriptionItem::Notice within timeout — \
             current session may suppress this warning"
        );
    }
}
