use ibapi::contracts::Contract;
use ibapi::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[tokio::test]
async fn news_providers() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let providers = client.news_providers().await.expect("news_providers failed");

    assert!(!providers.is_empty(), "expected at least one news provider");
}

#[tokio::test]
async fn news_bulletins() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let mut subscription = client.news_bulletins(true).await.expect("news_bulletins failed");

    // Bulletins may not arrive immediately - just verify subscription was created
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(3), subscription.next()).await;
}

#[tokio::test]
async fn historical_news() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    // Get AAPL contract_id
    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let details = client.contract_details(&contract).await.expect("contract_details failed");
    let con_id = details[0].contract.contract_id;

    let end = time::OffsetDateTime::now_utc();
    let start = end - time::Duration::days(7);

    rate_limit();
    let mut subscription = client
        .historical_news(con_id, &["BRFG"], start, end, 5)
        .await
        .expect("historical_news failed");

    // Paper accounts may not have news subscriptions - just verify API works
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}

#[tokio::test]
async fn contract_news() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let mut subscription = client.contract_news(&contract, &["BRFG"]).await.expect("contract_news failed");

    // May not receive data on paper account
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(3), subscription.next()).await;
}

#[tokio::test]
async fn broad_tape_news() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let mut subscription = client.broad_tape_news("BRFG").await.expect("broad_tape_news failed");

    // May not receive data on paper account
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(3), subscription.next()).await;
}

#[tokio::test]
#[ignore]
async fn news_article() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    // Requires a valid article_id from a news provider subscription
    rate_limit();
    let _article = client.news_article("BRFG", "BRFG$1234").await.expect("news_article failed");
}
