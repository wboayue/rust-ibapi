use crate::{contracts::Contract, news::ArticleType, server_versions, stubs::MessageBusStub, Client};
use std::sync::{Arc, RwLock};
use time::macros::datetime;

#[test]
fn test_news_providers() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["newsProviders|3|BZ|Benzinga Pro|DJ|Dow Jones|RSF|Test Provider|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let results = client.news_providers();
    assert!(results.is_ok(), "failed to request news providers: {}", results.err().unwrap());

    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages[0].encode_simple(), "85|");

    let news_providers = results.unwrap();
    assert_eq!(news_providers.len(), 3);

    assert_eq!(news_providers[0].code, "BZ");
    assert_eq!(news_providers[0].name, "Benzinga Pro");

    assert_eq!(news_providers[1].code, "DJ");
    assert_eq!(news_providers[1].name, "Dow Jones");

    assert_eq!(news_providers[2].code, "RSF");
    assert_eq!(news_providers[2].name, "Test Provider");
}

#[test]
fn test_news_bulletins() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["14|1|1|2|Message text|NASDAQ|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let results = client.news_bulletins(true);
    assert!(results.is_ok(), "failed to request news bulletins: {}", results.err().unwrap());

    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages[0].encode_simple(), "12|1|1|");

    let subscription = results.unwrap();
    if let Some(bulletin) = subscription.next() {
        assert_eq!(bulletin.message_id, 1);
        assert_eq!(bulletin.message_type, 2);
        assert_eq!(bulletin.message, "Message text");
        assert_eq!(bulletin.exchange, "NASDAQ");
    } else {
        panic!("Expected news bulletin");
    }
}

#[test]
fn test_historical_news() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "86\09000\02024-12-23 19:45:00.0\0DJ-N\0DJ-N$19985fef\0{A:800008,800008,800015:L:Chinese (Simplified and Traditional),Chinese (Simplified and Traditional),en:K:n/a:C:0.9882221817970276}These Stocks Are Moving the Most Today: Honda, Qualcomm, Broadcom, Lilly, ResMed, Tesla, Walmart, Rumble, and More -- Barrons.com\0".to_owned(),
            "87\09000\01\0".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let start_time = datetime!(2023-01-01 0:00 UTC);
    let end_time = datetime!(2023-01-02 0:00 UTC);

    let results = client.historical_news(8314, &["BZ", "DJ"], start_time, end_time, 10);
    assert!(results.is_ok(), "failed to request historical news: {}", results.err().unwrap());

    let request_messages = client.message_bus.request_messages();
    assert_eq!(
        request_messages[0].encode(),
        "86\09000\08314\0BZ+DJ\020230101 00:00:00 UTC\020230102 00:00:00 UTC\010\0\0"
    );

    let subscription = results.unwrap();
    if let Some(article) = subscription.next() {
        assert_eq!(article.provider_code, "DJ-N");
        assert_eq!(article.article_id, "DJ-N$19985fef");
        assert_eq!(article.headline, "{A:800008,800008,800015:L:Chinese (Simplified and Traditional),Chinese (Simplified and Traditional),en:K:n/a:C:0.9882221817970276}These Stocks Are Moving the Most Today: Honda, Qualcomm, Broadcom, Lilly, ResMed, Tesla, Walmart, Rumble, and More -- Barrons.com");
        assert_eq!(article.extra_data, "");
        assert_eq!(article.time.unix_timestamp(), 1734983100);
    } else {
        panic!("Expected news article");
    }
}

#[test]
fn test_news_article() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["83|9000|0|Article text content|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let results = client.news_article("BZ", "BZ$123");
    assert!(results.is_ok(), "failed to request news article: {}", results.err().unwrap());

    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages[0].encode_simple(), "84|9000|BZ|BZ$123||");

    let article = results.unwrap();
    assert_eq!(article.article_type, ArticleType::Text);
    assert_eq!(article.article_text, "Article text content");
}

#[test]
fn test_contract_news() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["84|9000|1672531200|BZ|BZ$123|Breaking news headline|TSLA:123|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = Contract::stock("TSLA");
    let results = client.contract_news(&contract, &["BZ", "DJ"]);
    assert!(results.is_ok(), "failed to request contract news: {}", results.err().unwrap());

    let request_messages = client.message_bus.request_messages();
    assert!(request_messages[0].encode().contains("mdoff,292:BZ,292:DJ"));

    let subscription = results.unwrap();
    if let Some(article) = subscription.next() {
        assert_eq!(article.provider_code, "BZ");
        assert_eq!(article.article_id, "BZ$123");
        assert_eq!(article.headline, "Breaking news headline");
        assert_eq!(article.extra_data, "TSLA:123");
        assert_eq!(article.time.unix_timestamp(), 1672531);
    } else {
        panic!("Expected news article");
    }
}

#[test]
fn test_broad_tape_news() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["84|9000|1672531200|BZ|BZ$123|Breaking news headline|TSLA:123|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let results = client.broad_tape_news("BZ");
    assert!(results.is_ok(), "failed to request broad tape news: {}", results.err().unwrap());

    let request_messages = client.message_bus.request_messages();
    assert!(request_messages[0].encode().contains("mdoff,292"));

    let subscription = results.unwrap();
    if let Some(article) = subscription.next() {
        assert_eq!(article.provider_code, "BZ");
        assert_eq!(article.article_id, "BZ$123");
        assert_eq!(article.headline, "Breaking news headline");
        assert_eq!(article.extra_data, "TSLA:123");
        assert_eq!(article.time.unix_timestamp(), 1672531);
    } else {
        panic!("Expected news article");
    }
}
