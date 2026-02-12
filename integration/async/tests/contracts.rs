use ibapi::contracts::{Contract, SecurityType};
use ibapi::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;

#[tokio::test]
async fn contract_details_stock() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let details = client.contract_details(&contract).await.expect("contract_details failed");

    assert!(!details.is_empty());
    assert_eq!(details[0].contract.symbol.0, "AAPL");
}

#[tokio::test]
async fn contract_details_futures() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::futures("ES").next_quarter().on_exchange("CME").build();
    let details = client.contract_details(&contract).await.expect("contract_details failed");

    assert!(!details.is_empty());
    assert_eq!(details[0].contract.symbol.0, "ES");
}

#[tokio::test]
async fn contract_details_forex() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::forex("EUR", "USD").build();
    let details = client.contract_details(&contract).await.expect("contract_details failed");

    assert!(!details.is_empty());
    assert_eq!(details[0].contract.security_type, SecurityType::ForexPair);
}

#[tokio::test]
#[serial(matching_symbols)]
async fn matching_symbols_exact() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let symbols = client.matching_symbols("AAPL").await.expect("matching_symbols failed");

    assert!(!symbols.is_empty());
    assert!(symbols.iter().any(|s| s.contract.symbol.0 == "AAPL"));
}

#[tokio::test]
#[serial(matching_symbols)]
async fn matching_symbols_partial() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let symbols = client.matching_symbols("Micro").await.expect("matching_symbols failed");

    assert!(!symbols.is_empty());
}

#[tokio::test]
async fn market_rule_returns_increments() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let rule = client.market_rule(26).await.expect("market_rule failed");

    assert!(!rule.price_increments.is_empty());
}

#[tokio::test]
async fn option_chain_returns_data() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    // Get AAPL contract_id first
    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let details = client.contract_details(&contract).await.expect("contract_details failed");
    let con_id = details[0].contract.contract_id;

    rate_limit();
    let mut subscription = client
        .option_chain("AAPL", "", SecurityType::Stock, con_id)
        .await
        .expect("option_chain failed");

    let item = subscription.next().await;
    assert!(item.is_some(), "expected at least one option chain result");
    let chain = item.unwrap().expect("option chain error");
    assert!(!chain.expirations.is_empty());
    assert!(!chain.strikes.is_empty());
}
