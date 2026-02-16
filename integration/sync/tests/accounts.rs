use std::time::Duration;

use ibapi::accounts::types::{AccountGroup, AccountId, ContractId};
use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::{Action, Order, PlaceOrder};
use ibapi_test::{rate_limit, require_market_open, ClientId, GATEWAY};
use serial_test::serial;

fn connect_and_get_account() -> (Client, AccountId, ClientId) {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let accounts = client.managed_accounts().expect("managed_accounts failed");
    assert!(!accounts.is_empty());
    let account = AccountId::from(accounts[0].as_str());
    (client, account, client_id)
}

#[test]
fn server_time_millis_returns_recent_time() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let time = client.server_time_millis().expect("server_time_millis failed");
    let now = time::OffsetDateTime::now_utc();

    assert!((now - time).whole_seconds().abs() < 60, "server time should be within 60s of local time");
}

#[test]
fn managed_accounts_returns_list() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let accounts = client.managed_accounts().expect("managed_accounts failed");
    assert!(!accounts.is_empty());
}

#[test]
fn family_codes_succeeds() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let _codes = client.family_codes().expect("family_codes failed");
}

#[test]
#[serial(account)]
fn positions_receives_data() {
    let (client, _account, _client_id) = connect_and_get_account();

    rate_limit();
    let subscription = client.positions().expect("positions failed");

    // Consume with timeout - positions may be empty on paper account
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(account)]
fn positions_multi_receives_data() {
    let (client, account, _client_id) = connect_and_get_account();

    rate_limit();
    let subscription = client.positions_multi(Some(&account), None).expect("positions_multi failed");

    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(account)]
fn pnl_receives_updates() {
    let (client, account, _client_id) = connect_and_get_account();

    rate_limit();
    let subscription = client.pnl(&account, None).expect("pnl failed");

    let item = subscription.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected at least one PnL update");
}

#[test]
#[serial(account)]
fn account_summary_all_tags() {
    let (client, _account, _client_id) = connect_and_get_account();

    rate_limit();
    let subscription = client
        .account_summary(&AccountGroup::from("All"), &["NetLiquidation", "TotalCashValue"])
        .expect("account_summary failed");

    let item = subscription.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected at least one account summary value");
}

#[test]
#[serial(account)]
fn account_summary_specific_tag() {
    let (client, _account, _client_id) = connect_and_get_account();

    rate_limit();
    let subscription = client
        .account_summary(&AccountGroup::from("All"), &["NetLiquidation"])
        .expect("account_summary failed");

    let item = subscription.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected NetLiquidation value");
}

#[test]
#[serial(account)]
fn account_updates_receives_data() {
    let (client, account, _client_id) = connect_and_get_account();

    rate_limit();
    let subscription = client.account_updates(&account).expect("account_updates failed");

    let item = subscription.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected at least one account update");
}

#[test]
#[serial(account)]
fn account_updates_multi() {
    let (client, account, _client_id) = connect_and_get_account();

    rate_limit();
    let subscription = client.account_updates_multi(Some(&account), None).expect("account_updates_multi failed");

    let _item = subscription.next_timeout(Duration::from_secs(10));
}

#[test]
#[serial(account)]
fn pnl_single_receives_updates() {
    require_market_open();
    let (client, account, _client_id) = connect_and_get_account();

    // Resolve AAPL contract_id
    let contract = Contract::stock("AAPL").build();
    rate_limit();
    let details = client.contract_details(&contract).expect("contract_details failed");
    let con_id = details[0].contract.contract_id;

    // Buy 1 share to create a position
    let buy = Order {
        action: Action::Buy,
        total_quantity: 1.0,
        order_type: "MKT".into(),
        ..Default::default()
    };
    rate_limit();
    let order_id = client.next_order_id();
    let sub = client.place_order(order_id, &contract, &buy).expect("buy failed");
    loop {
        match sub.next_timeout(Duration::from_secs(5)) {
            Some(PlaceOrder::OrderStatus(status)) if status.status == "Filled" => break,
            Some(_) => continue,
            None => panic!("buy order did not fill within 5s"),
        }
    }

    // Test pnl_single
    rate_limit();
    let pnl_sub = client.pnl_single(&account, ContractId(con_id), None).expect("pnl_single failed");
    let item = pnl_sub.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected at least one PnL single update");

    // Clean up - sell the share
    let sell = Order {
        action: Action::Sell,
        total_quantity: 1.0,
        order_type: "MKT".into(),
        ..Default::default()
    };
    rate_limit();
    let sell_id = client.next_order_id();
    let _ = client.place_order(sell_id, &contract, &sell);
}
