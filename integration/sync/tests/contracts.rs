use ibapi::client::blocking::Client;
use ibapi::contracts::{Contract, Currency, Exchange, OptionRight, SecurityType, Symbol};
use ibapi::subscriptions::SubscriptionItem;
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
fn contract_details_stream_matches_collect() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");
    let contract = Contract::stock("AAPL").build();

    rate_limit();
    let collected = client.contract_details(&contract).expect("contract_details failed");

    rate_limit();
    let request = client.contract_details_stream(&contract);
    let request_id = request.request_id();
    let subscription = request.subscribe().expect("subscribe failed");
    assert_eq!(subscription.request_id(), Some(request_id));

    let mut streamed = Vec::new();
    while let Some(item) = subscription.next() {
        match item.expect("stream item") {
            SubscriptionItem::Data(details) => streamed.push(details.contract.contract_id),
            SubscriptionItem::Notice(notice) => eprintln!("notice: {notice}"),
        }
    }

    let collected: Vec<i32> = collected.iter().map(|d| d.contract.contract_id).collect();
    assert!(!streamed.is_empty());
    assert_eq!(streamed, collected);
}

#[test]
fn contract_details_stream_early_drop_leaves_client_usable() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    // SPY calls for one expiry month: far more rows than we read.
    let broad = Contract {
        symbol: Symbol::from("SPY"),
        security_type: SecurityType::Option,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        last_trade_date_or_contract_month: "202611".into(),
        right: Some(OptionRight::Call),
        ..Default::default()
    };

    rate_limit();
    let subscription = client.contract_details_stream(&broad).subscribe().expect("subscribe failed");
    let rows = subscription.iter_data().take(5).collect::<Result<Vec<_>, _>>().expect("rows");
    assert_eq!(rows.len(), 5);
    drop(subscription); // writes cancelContractData

    rate_limit();
    let details = client
        .contract_details(&Contract::stock("AAPL").build())
        .expect("client unusable after early drop");
    assert!(!details.is_empty());
}

#[test]
fn contract_details_bond() {
    // TWS answers bond queries with BondContractData (msg 18); until #876's fix
    // every row was dropped and this returned an empty Vec.
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    let bonds = Contract {
        symbol: Symbol::from("AAPL"),
        security_type: SecurityType::Bond,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        ..Default::default()
    };

    rate_limit();
    let details = client.contract_details(&bonds).expect("contract_details failed");

    assert!(!details.is_empty(), "bond query returned no rows");
    assert!(details.iter().all(|d| d.contract.security_type == SecurityType::Bond));
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
fn contract_details_continuous_futures() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::continuous_futures("ES").on_exchange("CME").build();
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
    let symbols = client.matching_symbols("AAPL").expect("matching_symbols failed");

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
    let symbols = client.matching_symbols("Micro").expect("matching_symbols failed");

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
fn cancel_contract_details_succeeds() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    // Cancel with an arbitrary request_id - should not error even if no request is pending
    rate_limit();
    let result = client.cancel_contract_details(99999);
    assert!(result.is_ok(), "cancel_contract_details failed: {:?}", result.err());
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
    let subscription = client
        .option_chain("AAPL", SecurityType::Stock, con_id)
        .subscribe()
        .expect("option_chain failed");

    let chain = subscription
        .iter_data()
        .next()
        .expect("expected at least one option chain result")
        .expect("option chain subscription error");
    assert!(!chain.expirations.is_empty());
    assert!(!chain.strikes.is_empty());
}
