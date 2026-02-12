use ibapi::client::blocking::Client;
use ibapi::contracts::{Contract, SecurityType};
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;

#[test]
fn contract_details_stock() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let details = client.contract_details(&contract).expect("contract_details failed");

    assert!(!details.is_empty());
    assert_eq!(details[0].contract.symbol.0, "AAPL");
}

#[test]
fn contract_details_futures() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::futures("ES").next_quarter().on_exchange("CME").build();
    let details = client.contract_details(&contract).expect("contract_details failed");

    assert!(!details.is_empty());
    assert_eq!(details[0].contract.symbol.0, "ES");
}

#[test]
fn contract_details_forex() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::forex("EUR", "USD").build();
    let details = client.contract_details(&contract).expect("contract_details failed");

    assert!(!details.is_empty());
    assert_eq!(details[0].contract.security_type, SecurityType::ForexPair);
}

#[test]
#[serial(matching_symbols)]
fn matching_symbols_exact() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let symbols: Vec<_> = client.matching_symbols("AAPL").expect("matching_symbols failed").collect();

    assert!(!symbols.is_empty());
    assert!(symbols.iter().any(|s| s.contract.symbol.0 == "AAPL"));
}

#[test]
#[serial(matching_symbols)]
fn matching_symbols_partial() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let symbols: Vec<_> = client.matching_symbols("Micro").expect("matching_symbols failed").collect();

    assert!(!symbols.is_empty());
}

#[test]
fn market_rule_returns_increments() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let rule = client.market_rule(26).expect("market_rule failed");

    assert!(!rule.price_increments.is_empty());
}

#[test]
fn option_chain_returns_data() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    // Get AAPL contract_id first
    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let details = client.contract_details(&contract).expect("contract_details failed");
    let con_id = details[0].contract.contract_id;

    rate_limit();
    let subscription = client.option_chain("AAPL", "", SecurityType::Stock, con_id).expect("option_chain failed");

    let chain = subscription.iter().next();
    assert!(chain.is_some(), "expected at least one option chain result");
    let chain = chain.unwrap();
    assert!(!chain.expirations.is_empty());
    assert!(!chain.strikes.is_empty());
}
