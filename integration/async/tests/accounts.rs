use futures::StreamExt;
use ibapi::accounts::types::{AccountGroup, AccountId, ContractId};
use ibapi::contracts::Contract;
use ibapi::orders::{Action, Order, OrderStatusKind, PlaceOrder};
use ibapi::subscriptions::SubscriptionItem;
use ibapi::Client;
use ibapi_test::{rate_limit, require_market_open, ClientId, GATEWAY};
use serial_test::serial;

async fn connect_and_get_account() -> (Client, AccountId, ClientId) {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let accounts = client.managed_accounts().await.expect("managed_accounts failed");
    assert!(!accounts.is_empty());
    let account = AccountId::from(accounts[0].as_str());
    (client, account, client_id)
}

#[tokio::test]
async fn server_time_millis_returns_recent_time() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let time = client.server_time_millis().await.expect("server_time_millis failed");
    let now = time::OffsetDateTime::now_utc();

    assert!((now - time).whole_seconds().abs() < 60, "server time should be within 60s of local time");
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
    let (client, _account, _client_id) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client.positions().await.expect("positions failed");

    // Consume with timeout - positions may be empty on paper account
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}

#[tokio::test]
#[serial(account)]
async fn positions_multi_receives_data() {
    let (client, account, _client_id) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client.positions_multi(Some(&account), None).await.expect("positions_multi failed");

    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}

#[tokio::test]
#[serial(account)]
async fn pnl_receives_updates() {
    let (client, account, _client_id) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client.pnl(&account, None).await.expect("pnl failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
    assert!(item.is_ok(), "pnl timed out");
    assert!(item.unwrap().is_some(), "expected at least one PnL update");
}

#[tokio::test]
#[serial(account)]
async fn account_summary_all_tags() {
    let (client, _account, _client_id) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client
        .account_summary(&AccountGroup::from("All"), &["NetLiquidation", "TotalCashValue"])
        .await
        .expect("account_summary failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
    assert!(item.is_ok(), "account_summary timed out");
    assert!(item.unwrap().is_some(), "expected at least one account summary value");
}

#[tokio::test]
#[serial(account)]
async fn account_summary_specific_tag() {
    let (client, _account, _client_id) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client
        .account_summary(&AccountGroup::from("All"), &["NetLiquidation"])
        .await
        .expect("account_summary failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
    assert!(item.is_ok(), "account_summary timed out");
    assert!(item.unwrap().is_some(), "expected NetLiquidation value");
}

#[tokio::test]
#[serial(account)]
async fn account_updates_receives_data() {
    let (client, account, _client_id) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client.account_updates(&account).await.expect("account_updates failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
    assert!(item.is_ok(), "account_updates timed out");
    assert!(item.unwrap().is_some(), "expected at least one account update");
}

#[tokio::test]
#[serial(account)]
async fn account_updates_multi() {
    let (client, account, _client_id) = connect_and_get_account().await;

    rate_limit();
    let mut subscription = client
        .account_updates_multi(Some(&account), None)
        .await
        .expect("account_updates_multi failed");

    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
}

#[tokio::test]
#[serial(account)]
async fn pnl_single_receives_updates() {
    require_market_open();
    let (client, account, _client_id) = connect_and_get_account().await;

    // Resolve AAPL contract_id
    let contract = Contract::stock("AAPL").build();
    rate_limit();
    let details = client.contract_details(&contract).await.expect("contract_details failed");
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
    let mut sub = client.place_order(order_id, &contract, &buy).await.expect("buy failed");
    let filled = tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
        while let Some(Ok(SubscriptionItem::Data(event))) = sub.next().await {
            if let PlaceOrder::OrderStatus(status) = &event {
                if status.status == OrderStatusKind::Filled {
                    return;
                }
            }
        }
    })
    .await;
    assert!(filled.is_ok(), "buy order did not fill within 5s");

    // Test pnl_single
    rate_limit();
    let mut pnl_sub = client.pnl_single(&account, ContractId(con_id), None).await.expect("pnl_single failed");
    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), pnl_sub.next()).await;
    assert!(item.is_ok(), "pnl_single timed out");
    assert!(item.unwrap().is_some(), "expected at least one PnL single update");

    // Clean up - sell the share
    let sell = Order {
        action: Action::Sell,
        total_quantity: 1.0,
        order_type: "MKT".into(),
        ..Default::default()
    };
    rate_limit();
    let sell_id = client.next_order_id();
    let _ = client.place_order(sell_id, &contract, &sell).await;
}

#[tokio::test]
async fn soft_dollar_tiers_succeeds() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let _tiers = client.soft_dollar_tiers().await.expect("soft_dollar_tiers failed");
}

#[tokio::test]
async fn user_info_succeeds() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let _info = client.user_info().await.expect("user_info failed");
}

#[tokio::test]
async fn set_server_log_level_succeeds() {
    use ibapi::accounts::ServerLogLevel;

    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    client
        .set_server_log_level(ServerLogLevel::Detail)
        .await
        .expect("set_server_log_level failed");
}

/// Requires an FA (Financial Advisor) account; paper trading accounts return
/// an error code 321 "Server error when validating an API client request".
#[tokio::test]
#[ignore]
async fn request_fa_groups() {
    use ibapi::accounts::FaDataType;

    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let cfg = client.request_fa(FaDataType::Groups).await.expect("request_fa failed");
    assert_eq!(cfg.fa_data_type, FaDataType::Groups);
}

/// Requires an FA account AND mutates server-side state — only run against a
/// dedicated FA test account where the existing groups XML has been backed up.
#[tokio::test]
#[ignore]
async fn replace_fa_round_trip() {
    use ibapi::accounts::FaDataType;

    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let original = client.request_fa(FaDataType::Groups).await.expect("request_fa failed");

    rate_limit();
    let result = client.replace_fa(FaDataType::Groups, &original.xml).await.expect("replace_fa failed");
    assert!(!result.text.is_empty());
}

/// Requires IB Linking extension authentication to be configured on the
/// account. Without it, TWS rejects verify_request.
#[tokio::test]
#[ignore]
async fn verify_handshake() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let _challenge = client.verify_request("rust-ibapi-test", "1.0").await.expect("verify_request failed");
}
