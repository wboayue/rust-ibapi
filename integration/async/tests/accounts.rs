use ibapi::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;

async fn connect_and_get_account() -> (Client, String) {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let accounts = client.managed_accounts().await.expect("managed_accounts failed");
    assert!(!accounts.is_empty());
    let account = accounts[0].clone();
    (client, account)
}

#[tokio::test]
async fn managed_accounts_returns_list() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let accounts = client.managed_accounts().await.expect("managed_accounts failed");
    assert!(!accounts.is_empty());
}

#[tokio::test]
async fn family_codes_succeeds() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let _codes = client.family_codes().await.expect("family_codes failed");
}

#[tokio::test]
#[serial(account)]
async fn positions_receives_data() {
    let (client, _account) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client.positions().await.expect("positions failed");

    // Consume with timeout - positions may be empty on paper account
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}

#[tokio::test]
#[serial(account)]
async fn positions_multi_receives_data() {
    let (client, account) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client.positions_multi(Some(&account), None).await.expect("positions_multi failed");

    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}

#[tokio::test]
#[serial(account)]
async fn pnl_receives_updates() {
    let (client, account) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client.pnl(&account, None).await.expect("pnl failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
    assert!(item.is_ok(), "pnl timed out");
    assert!(item.unwrap().is_some(), "expected at least one PnL update");
}

#[tokio::test]
#[serial(account)]
async fn account_summary_all_tags() {
    let (client, _account) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client
        .account_summary("All", &["NetLiquidation", "TotalCashValue"])
        .await
        .expect("account_summary failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
    assert!(item.is_ok(), "account_summary timed out");
    assert!(item.unwrap().is_some(), "expected at least one account summary value");
}

#[tokio::test]
#[serial(account)]
async fn account_summary_specific_tag() {
    let (client, _account) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client.account_summary("All", &["NetLiquidation"]).await.expect("account_summary failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
    assert!(item.is_ok(), "account_summary timed out");
    assert!(item.unwrap().is_some(), "expected NetLiquidation value");
}

#[tokio::test]
#[serial(account)]
async fn account_updates_receives_data() {
    let (client, account) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client.account_updates(&account).await.expect("account_updates failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
    assert!(item.is_ok(), "account_updates timed out");
    assert!(item.unwrap().is_some(), "expected at least one account update");
}

#[tokio::test]
#[serial(account)]
async fn account_updates_multi() {
    let (client, account) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client
        .account_updates_multi(Some(&account), None)
        .await
        .expect("account_updates_multi failed");

    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
}

#[tokio::test]
#[serial(account)]
#[ignore]
async fn pnl_single_receives_updates() {
    let (client, account) = connect_and_get_account().await;

    // Need a contract_id from a held position
    rate_limit();
    let mut subscription = client.positions().await.expect("positions failed");
    let position = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
    let position = position.expect("timeout").expect("stream ended").expect("positions error");
    let con_id = position.contract.contract_id;
    drop(subscription);

    rate_limit();
    let mut pnl_sub = client.pnl_single(&account, con_id, None).await.expect("pnl_single failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), pnl_sub.next()).await;
    assert!(item.is_ok(), "pnl_single timed out");
    assert!(item.unwrap().is_some(), "expected at least one PnL single update");
}
