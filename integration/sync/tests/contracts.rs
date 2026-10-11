use ibapi::client::blocking::Client;
use ibapi::contracts::{Contract, Currency, Exchange, OptionRight, SecurityType, Symbol};
use ibapi::subscriptions::{Drained, SubscriptionItem};
use ibapi::Error;
use ibapi_integration_sync::read_to_terminal;
use ibapi_test::{rate_limit, yyyymm_months_from_now, ClientId, AAPL_CON_ID, GATEWAY};
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

    let broad = spy_calls_one_month();

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

/// SPY calls for one expiry month two months out: several hundred rows, which
/// TWS prepares in full before sending.
fn spy_calls_one_month() -> Contract {
    Contract {
        symbol: Symbol::from("SPY"),
        security_type: SecurityType::Option,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        last_trade_date_or_contract_month: yyyymm_months_from_now(2),
        right: Some(OptionRight::Call),
        ..Default::default()
    }
}

#[test]
fn contract_details_stream_buffer_limit_fails_a_stalled_reader() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let subscription = client
        .contract_details_stream(&spy_calls_one_month())
        .buffer_limit(5)
        .subscribe()
        .expect("subscribe failed");
    std::thread::sleep(std::time::Duration::from_secs(2)); // stall: let TWS send far more than 5 rows

    let (rows, outcome) = read_to_terminal(|| subscription.next());
    // At least the 5 queued rows; more if TWS was still sending when the
    // reader woke and freed slots before the overflow.
    assert!(rows >= 5, "every queued row is delivered before the error, got {rows}");
    assert!(matches!(outcome, Some(Err(Error::BufferLimitExceeded { limit: 5 }))), "got {outcome:?}");
}

#[test]
fn contract_details_stream_cancel_and_drain_ends() {
    // Observed live: TWS sends the rest of the result after the cancel, then
    // the end marker, so the drain ends with `Ended`.
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let subscription = client
        .contract_details_stream(&spy_calls_one_month())
        .subscribe()
        .expect("subscribe failed");
    for _ in 0..3 {
        subscription.next().expect("a row").expect("row");
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let outcome = subscription.cancel_and_drain(deadline).expect("drain failed");
    assert_eq!(outcome, Drained::Ended);
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

#[test]
fn option_chain_request_id_and_buffer_limit_read_to_end() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let request = client.option_chain("AAPL", SecurityType::Stock, AAPL_CON_ID).buffer_limit(256);
    assert!(request.request_id() > 0, "request id is known before sending");

    let subscription = request.subscribe().expect("option_chain failed");
    let (chains, outcome) = read_to_terminal(|| subscription.next());
    // A reader that keeps up never hits the cap; the stream ends on TWS's end marker.
    assert!(outcome.is_none(), "got {outcome:?}");
    assert!(chains > 0, "expected at least one chain");
}

#[test]
fn option_chain_buffer_limit_fails_a_stalled_reader() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let subscription = client
        .option_chain("AAPL", SecurityType::Stock, AAPL_CON_ID)
        .buffer_limit(3)
        .subscribe()
        .expect("option_chain failed");
    std::thread::sleep(std::time::Duration::from_secs(2)); // stall: AAPL lists far more than 3 exchanges

    let (chains, outcome) = read_to_terminal(|| subscription.next());
    assert!(chains >= 3, "every queued chain is delivered before the error, got {chains}");
    assert!(matches!(outcome, Some(Err(Error::BufferLimitExceeded { limit: 3 }))), "got {outcome:?}");
}
