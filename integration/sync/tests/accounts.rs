use std::time::Duration;

use ibapi::accounts::types::{AccountGroup, AccountId, ContractId};
use ibapi::accounts::PositionUpdate;
use ibapi::client::blocking::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};
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
#[ignore]
fn pnl_single_receives_updates() {
    let (client, account, _client_id) = connect_and_get_account();

    // Need a contract_id from a held position
    rate_limit();
    let subscription = client.positions().expect("positions failed");
    let update = subscription.next_timeout(Duration::from_secs(5));
    let update = update.expect("no positions held - cannot test pnl_single");
    let con_id = match update {
        PositionUpdate::Position(pos) => pos.contract.contract_id,
        PositionUpdate::PositionEnd => panic!("no positions held"),
    };

    rate_limit();
    let pnl_sub = client.pnl_single(&account, ContractId(con_id), None).expect("pnl_single failed");

    let item = pnl_sub.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected at least one PnL single update");
}
