use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[test]
fn news_providers() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let providers = client.news_providers().expect("news_providers failed");

    assert!(!providers.is_empty(), "expected at least one news provider");
}

#[test]
fn news_bulletins() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let subscription = client.news_bulletins(true).expect("news_bulletins failed");

    // Bulletins may not arrive immediately - just verify subscription was created
    let _item = subscription.next_timeout(Duration::from_secs(3));
}

#[test]
fn historical_news() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    // Get AAPL contract_id
    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let details = client.contract_details(&contract).expect("contract_details failed");
    let con_id = details[0].contract.contract_id;

    let end = time::OffsetDateTime::now_utc();
    let start = end - time::Duration::days(7);

    rate_limit();
    let subscription = client.historical_news(con_id, &["BRFG"], start, end, 5).expect("historical_news failed");

    // Paper accounts may not have news subscriptions - just verify API works
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
fn contract_news() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let subscription = client.contract_news(&contract, &["BRFG"]).expect("contract_news failed");

    // May not receive data on paper account
    let _item = subscription.next_timeout(Duration::from_secs(3));
}

#[test]
fn broad_tape_news() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let subscription = client.broad_tape_news("BRFG").expect("broad_tape_news failed");

    // May not receive data on paper account
    let _item = subscription.next_timeout(Duration::from_secs(3));
}

#[test]
#[ignore]
fn news_article() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    // Requires a valid article_id from a news provider subscription
    rate_limit();
    let _article = client.news_article("BRFG", "BRFG$1234").expect("news_article failed");
}
