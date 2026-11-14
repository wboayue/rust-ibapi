use crate::client::blocking::Client;
use crate::common::test_utils::helpers::{
    assert_request, assert_request_msg_id, proto_response, request_message_count, TEST_CONTRACT_ID, TEST_REQ_ID_FIRST,
};
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::news::{ArticleType, BulletinType};
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::testdata::builders::market_data::market_data_request;
use crate::testdata::builders::news::{
    historical_news, historical_news_end, historical_news_request, news_article, news_article_request, news_bulletin, news_bulletins_request,
    news_providers, news_providers_request, tick_news,
};
use crate::testdata::builders::ResponseProtoEncoder;
use std::sync::Arc;
use time::macros::datetime;

#[test]
fn news_providers_works() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::NewsProviders,
        news_providers()
            .provider("BZ", "Benzinga Pro")
            .provider("DJ", "Dow Jones")
            .provider("RSF", "Test Provider")
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_NEWS_DATA);

    let results = client.news_providers().expect("request news providers failed");

    assert_request(&message_bus, 0, &news_providers_request());

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].code, "BZ");
    assert_eq!(results[0].name, "Benzinga Pro");
    assert_eq!(results[1].code, "DJ");
    assert_eq!(results[1].name, "Dow Jones");
    assert_eq!(results[2].code, "RSF");
    assert_eq!(results[2].name, "Test Provider");
}

#[test]
fn news_bulletins() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::NewsBulletins,
        news_bulletin()
            .message_id(1)
            .message_type(2)
            .message("Message text")
            .exchange("NASDAQ")
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_NEWS_DATA);

    let subscription = client.news_bulletins(true).expect("request news bulletins failed");

    assert_request(&message_bus, 0, &news_bulletins_request().all_messages(true));

    let bulletin = subscription.next_data().expect("expected news bulletin").expect("subscription error");
    assert_eq!(bulletin.message_id, 1);
    assert_eq!(bulletin.message_type, BulletinType::ExchangeUnavailable);
    assert_eq!(bulletin.message, "Message text");
    assert_eq!(bulletin.exchange, "NASDAQ");
}

#[test]
fn historical_news_works() {
    let headline = "{A:800008,800008,800015:L:Chinese (Simplified and Traditional),Chinese (Simplified and Traditional),en:K:n/a:C:0.9882221817970276}These Stocks Are Moving the Most Today: Honda, Qualcomm, Broadcom, Lilly, ResMed, Tesla, Walmart, Rumble, and More -- Barrons.com";
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalNews,
            historical_news()
                .request_id(TEST_REQ_ID_FIRST)
                .time("2024-12-23 19:45:00.0")
                .provider_code("DJ-N")
                .article_id("DJ-N$19985fef")
                .headline(headline)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalNewsEnd,
            historical_news_end().request_id(TEST_REQ_ID_FIRST).encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_NEWS_DATA);

    let start_time = datetime!(2023-01-01 0:00 UTC);
    let end_time = datetime!(2023-01-02 0:00 UTC);

    let subscription = client
        .historical_news(TEST_CONTRACT_ID, &["BZ", "DJ"], start_time, end_time, 300)
        .expect("request historical news failed");

    assert_request(
        &message_bus,
        0,
        &historical_news_request()
            .request_id(TEST_REQ_ID_FIRST)
            .provider_codes(&["BZ", "DJ"])
            .start_time(start_time)
            .end_time(end_time)
            .total_results(300),
    );

    let article = subscription.next_data().expect("expected news article").expect("subscription error");
    assert_eq!(article.provider_code, "DJ-N");
    assert_eq!(article.article_id, "DJ-N$19985fef");
    assert_eq!(article.headline, headline);
    assert_eq!(article.extra_data, "");
    assert_eq!(article.time.unwrap().unix_timestamp(), 1734983100);
}

#[test]
fn historical_news_page() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::HistoricalNews,
            historical_news()
                .request_id(TEST_REQ_ID_FIRST)
                .time("2024-12-23 19:45:00.0")
                .provider_code("DJ-N")
                .article_id("DJ-N$19985fef")
                .headline("First")
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalNews,
            historical_news()
                .request_id(TEST_REQ_ID_FIRST)
                .time("2024-12-23 18:45:00.0")
                .provider_code("DJ-N")
                .article_id("DJ-N$19985fee")
                .headline("Second")
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalNewsEnd,
            historical_news_end().request_id(TEST_REQ_ID_FIRST).has_more(true).encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_NEWS_DATA);

    let start_time = datetime!(2023-01-01 0:00 UTC);
    let end_time = datetime!(2023-01-02 0:00 UTC);

    let page = client
        .historical_news_page(TEST_CONTRACT_ID, &["BZ", "DJ"], start_time, end_time, 2)
        .expect("request historical news page failed");

    assert_request(
        &message_bus,
        0,
        &historical_news_request()
            .request_id(TEST_REQ_ID_FIRST)
            .provider_codes(&["BZ", "DJ"])
            .start_time(start_time)
            .end_time(end_time)
            .total_results(2),
    );

    assert_eq!(page.articles.len(), 2);
    assert_eq!(page.articles[0].article_id, "DJ-N$19985fef");
    assert_eq!(page.articles[0].headline, "First");
    assert_eq!(page.articles[1].article_id, "DJ-N$19985fee");
    assert_eq!(page.articles[1].headline, "Second");
    assert!(page.has_more);
}

/// The deadline bounds the wait for the end frame: the queued article is read, the
/// next read times out, and a queued article is not a page.
#[test]
fn historical_news_page_times_out_at_the_deadline() {
    let message_bus = Arc::new(
        MessageBusStub::with_ordered_responses(vec![proto_response(
            IncomingMessages::HistoricalNews,
            historical_news()
                .request_id(TEST_REQ_ID_FIRST)
                .time("2024-12-23 19:45:00.0")
                .provider_code("DJ-N")
                .article_id("DJ-N$19985fef")
                .headline("First")
                .encode_proto(),
        )])
        .keeping_channels_open(),
    );
    let client = Client::stubbed(message_bus, server_versions::PROTOBUF_NEWS_DATA).with_one_shot_timeout(std::time::Duration::from_millis(50));

    let error = client
        .historical_news_page(
            TEST_CONTRACT_ID,
            &["BZ", "DJ"],
            datetime!(2023-01-01 0:00 UTC),
            datetime!(2023-01-02 0:00 UTC),
            2,
        )
        .unwrap_err();

    assert!(matches!(error, crate::Error::Timeout), "got {error:?}");
}

/// Only the end frame proves the page is complete: a close after an article is
/// an error, not a short page, and is not retried.
#[test]
fn historical_news_page_requires_end() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::HistoricalNews,
        historical_news().request_id(TEST_REQ_ID_FIRST).article_id("DJ-N$19985fef").encode_proto(),
    )]));
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_NEWS_DATA);

    let result = client.historical_news_page(
        TEST_CONTRACT_ID,
        &["BZ"],
        datetime!(2023-01-01 0:00 UTC),
        datetime!(2023-01-02 0:00 UTC),
        2,
    );

    assert!(matches!(result, Err(crate::Error::UnexpectedEndOfStream)), "{result:?}");
    assert_eq!(request_message_count(&message_bus), 1, "a closed stream must not be retried");
}

/// A connection reset resends the request and reads the page from the resend.
#[test]
fn historical_news_page_retries_a_connection_reset() {
    let message_bus = Arc::new(
        MessageBusStub::with_ordered_responses(vec![
            proto_response(
                IncomingMessages::HistoricalNews,
                historical_news().request_id(TEST_REQ_ID_FIRST).article_id("DJ-N$19985fef").encode_proto(),
            ),
            proto_response(
                IncomingMessages::HistoricalNewsEnd,
                historical_news_end().request_id(TEST_REQ_ID_FIRST).encode_proto(),
            ),
        ])
        .with_connection_resets(1),
    );
    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_NEWS_DATA);

    let page = client
        .historical_news_page(
            TEST_CONTRACT_ID,
            &["BZ"],
            datetime!(2023-01-01 0:00 UTC),
            datetime!(2023-01-02 0:00 UTC),
            2,
        )
        .expect("the reset must be retried, not surfaced");

    assert_eq!(page.articles.len(), 1);
    assert_eq!(page.articles[0].article_id, "DJ-N$19985fef");
    assert!(!page.has_more, "an end frame without the field reads as false");
    assert_eq!(request_message_count(&message_bus), 2, "the retry must re-send the request");
}

#[test]
fn news_article_works() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::NewsArticle,
        news_article()
            .request_id(TEST_REQ_ID_FIRST)
            .article_type(0)
            .article_text("Article text content")
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_NEWS_DATA);

    let article = client.news_article("BZ", "BZ$123").expect("request news article failed");

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

#[test]
fn contract_news() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::TickNews,
        tick_news()
            .request_id(9000)
            .timestamp_millis(1_672_531_200_000)
            .provider_code("BZ")
            .article_id("BZ$123")
            .headline("Breaking news headline")
            .extra_data("TSLA:123")
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract::stock("TSLA").build();
    let subscription = client.contract_news(&contract, &["BZ", "DJ"]).expect("request contract news failed");

    assert_request(
        &message_bus,
        0,
        &market_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .generic_ticks(&["mdoff", "292:BZ", "292:DJ"]),
    );

    let article = subscription.next_data().expect("expected news article").expect("subscription error");
    assert_eq!(article.provider_code, "BZ");
    assert_eq!(article.article_id, "BZ$123");
    assert_eq!(article.headline, "Breaking news headline");
    assert_eq!(article.extra_data, "TSLA:123");
    assert_eq!(article.time.unwrap().unix_timestamp(), 1_672_531_200);
}

#[test]
fn broad_tape_news() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::TickNews,
        tick_news()
            .request_id(9000)
            .timestamp_millis(1_672_531_200_000)
            .provider_code("BZ")
            .article_id("BZ$123")
            .headline("Breaking news headline")
            .extra_data("TSLA:123")
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let subscription = client.broad_tape_news("BZ").expect("request broad tape news failed");

    let news_contract = Contract::news("BZ");
    assert_request(
        &message_bus,
        0,
        &market_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&news_contract)
            .generic_ticks(&["mdoff", "292"]),
    );

    let article = subscription.next_data().expect("expected news article").expect("subscription error");
    assert_eq!(article.provider_code, "BZ");
    assert_eq!(article.article_id, "BZ$123");
    assert_eq!(article.headline, "Breaking news headline");
    assert_eq!(article.extra_data, "TSLA:123");
    assert_eq!(article.time.unwrap().unix_timestamp(), 1_672_531_200);
}

#[test]
fn contract_news_cancellation() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract::stock("TSLA").build();
    let subscription = client.contract_news(&contract, &["BZ"]).unwrap();
    assert_eq!(request_message_count(&message_bus), 1);

    subscription.cancel();

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request_msg_id(&message_bus, 1, OutgoingMessages::CancelMarketData);
}

#[test]
fn broad_tape_news_cancellation() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let subscription = client.broad_tape_news("BZ").unwrap();
    assert_eq!(request_message_count(&message_bus), 1);

    subscription.cancel();

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request_msg_id(&message_bus, 1, OutgoingMessages::CancelMarketData);
}
