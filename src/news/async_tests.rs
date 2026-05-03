use crate::common::test_utils::helpers::{assert_request, assert_request_msg_id, request_message_count, TEST_REQ_ID_FIRST};
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::news::ArticleType;
use crate::stubs::MessageBusStub;
use crate::testdata::builders::market_data::market_data_request;
use crate::testdata::builders::news::{
    cancel_news_bulletins_request, historical_news_request, news_article_request, news_bulletins_request, news_providers_request,
};
use crate::{server_versions, Client};
use std::sync::{Arc, RwLock};
use time::macros::datetime;

#[tokio::test]
async fn test_news_providers() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["newsProviders|3|BZ|Benzinga Pro|DJ|Dow Jones|RSF|Test Provider|".to_owned()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let news_providers = client.news_providers().await.expect("request news providers failed");

    assert_request(&message_bus, 0, &news_providers_request());

    assert_eq!(news_providers.len(), 3);
    assert_eq!(news_providers[0].code, "BZ");
    assert_eq!(news_providers[0].name, "Benzinga Pro");
    assert_eq!(news_providers[1].code, "DJ");
    assert_eq!(news_providers[1].name, "Dow Jones");
    assert_eq!(news_providers[2].code, "RSF");
    assert_eq!(news_providers[2].name, "Test Provider");
}

#[tokio::test]
async fn test_news_bulletins() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["14|1|1|2|Message text|NASDAQ|".to_owned()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let mut subscription = client.news_bulletins(true).await.expect("request news bulletins failed");

    assert_request(&message_bus, 0, &news_bulletins_request().all_messages(true));

    let bulletin = subscription.next().await.expect("expected news bulletin").unwrap();
    assert_eq!(bulletin.message_id, 1);
    assert_eq!(bulletin.message_type, 2);
    assert_eq!(bulletin.message, "Message text");
    assert_eq!(bulletin.exchange, "NASDAQ");
}

#[tokio::test]
async fn test_historical_news() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "86\09000\02024-12-23 19:45:00.0\0DJ-N\0DJ-N$19985fef\0{A:800008,800008,800015:L:Chinese (Simplified and Traditional),Chinese (Simplified and Traditional),en:K:n/a:C:0.9882221817970276}These Stocks Are Moving the Most Today: Honda, Qualcomm, Broadcom, Lilly, ResMed, Tesla, Walmart, Rumble, and More -- Barrons.com\0".to_owned(),
            "87\09000\01\0".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let start_time = datetime!(2023-01-01 0:00 UTC);
    let end_time = datetime!(2023-01-02 0:00 UTC);

    let mut subscription = client
        .historical_news(8314, &["BZ", "DJ"], start_time, end_time, 10)
        .await
        .expect("request historical news failed");

    assert_request(
        &message_bus,
        0,
        &historical_news_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract_id(8314)
            .provider_codes(&["BZ", "DJ"])
            .start_time(start_time)
            .end_time(end_time)
            .total_results(10),
    );

    let article = subscription.next().await.expect("expected news article").unwrap();
    assert_eq!(article.provider_code, "DJ-N");
    assert_eq!(article.article_id, "DJ-N$19985fef");
    assert_eq!(article.headline, "{A:800008,800008,800015:L:Chinese (Simplified and Traditional),Chinese (Simplified and Traditional),en:K:n/a:C:0.9882221817970276}These Stocks Are Moving the Most Today: Honda, Qualcomm, Broadcom, Lilly, ResMed, Tesla, Walmart, Rumble, and More -- Barrons.com");
    assert_eq!(article.extra_data, "");
    assert_eq!(article.time.unix_timestamp(), 1734983100);
}

#[tokio::test]
async fn test_news_article() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["83|9000|0|Article text content|".to_owned()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let article = client.news_article("BZ", "BZ$123").await.expect("request news article failed");

    assert_request(
        &message_bus,
        0,
        &news_article_request()
            .request_id(TEST_REQ_ID_FIRST)
            .provider_code("BZ")
            .article_id("BZ$123"),
    );

    assert_eq!(article.article_type, ArticleType::Text);
    assert_eq!(article.article_text, "Article text content");
}

#[tokio::test]
async fn test_contract_news() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["84|9000|1672531200|BZ|BZ$123|Breaking news headline|TSLA:123|".to_owned()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract::stock("TSLA").build();
    let mut subscription = client
        .contract_news(&contract, &["BZ", "DJ"])
        .await
        .expect("request contract news failed");

    assert_request(
        &message_bus,
        0,
        &market_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .generic_ticks(&["mdoff", "292:BZ", "292:DJ"]),
    );

    let article = subscription.next().await.expect("expected news article").unwrap();
    assert_eq!(article.provider_code, "BZ");
    assert_eq!(article.article_id, "BZ$123");
    assert_eq!(article.headline, "Breaking news headline");
    assert_eq!(article.extra_data, "TSLA:123");
    assert_eq!(article.time.unix_timestamp(), 1672531);
}

#[tokio::test]
async fn test_broad_tape_news() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["84|9000|1672531200|BZ|BZ$123|Breaking news headline|TSLA:123|".to_owned()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let mut subscription = client.broad_tape_news("BZ").await.expect("request broad tape news failed");

    let news_contract = Contract::news("BZ");
    assert_request(
        &message_bus,
        0,
        &market_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&news_contract)
            .generic_ticks(&["mdoff", "292"]),
    );

    let article = subscription.next().await.expect("expected news article").unwrap();
    assert_eq!(article.provider_code, "BZ");
    assert_eq!(article.article_id, "BZ$123");
    assert_eq!(article.headline, "Breaking news headline");
    assert_eq!(article.extra_data, "TSLA:123");
    assert_eq!(article.time.unix_timestamp(), 1672531);
}

#[tokio::test]
async fn test_news_bulletin_cancellation() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["14|1|1|2|Message text|NASDAQ|".to_owned()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let mut subscription = client.news_bulletins(true).await.unwrap();
    let _ = subscription.next().await;

    assert_request(&message_bus, 0, &news_bulletins_request().all_messages(true));

    subscription.cancel().await;

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(&message_bus, 1, &cancel_news_bulletins_request());
}

#[tokio::test]
async fn test_contract_news_cancellation() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["84|9000|1672531200|BZ|BZ$123|Breaking news headline|TSLA:123|".to_owned()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract::stock("TSLA").build();
    let mut subscription = client.contract_news(&contract, &["BZ"]).await.unwrap();
    let _ = subscription.next().await;

    assert_eq!(request_message_count(&message_bus), 1);

    subscription.cancel().await;

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request_msg_id(&message_bus, 1, OutgoingMessages::CancelMarketData);
}
