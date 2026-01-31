//! Asynchronous implementation of news functionality

use super::common::{decoders, encoders};
use super::*;
use crate::contracts::Contract;
use crate::market_data::realtime;
use crate::messages::OutgoingMessages;
#[cfg(not(feature = "sync"))]
use crate::messages::{IncomingMessages, RequestMessage, ResponseMessage};
use crate::subscriptions::Subscription;
#[cfg(not(feature = "sync"))]
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::{server_versions, Client, Error};

#[cfg(not(feature = "sync"))]
impl StreamDecoder<NewsBulletin> for NewsBulletin {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::NewsBulletins];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<NewsBulletin, Error> {
        match message.message_type() {
            IncomingMessages::NewsBulletins => Ok(decoders::decode_news_bulletin(message.clone())?),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_news_bulletin()
    }
}

#[cfg(not(feature = "sync"))]
impl StreamDecoder<NewsArticle> for NewsArticle {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::HistoricalNews,
        IncomingMessages::HistoricalNewsEnd,
        IncomingMessages::TickNews,
    ];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<NewsArticle, Error> {
        match message.message_type() {
            IncomingMessages::HistoricalNews => Ok(decoders::decode_historical_news(None, message.clone())?),
            IncomingMessages::HistoricalNewsEnd => Err(Error::EndOfStream),
            IncomingMessages::TickNews => Ok(decoders::decode_tick_news(message.clone())?),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        // News articles can come from market data subscriptions, so use the appropriate cancel
        if context.and_then(|c| c.request_type) == Some(OutgoingMessages::RequestMarketData) {
            let request_id = request_id.expect("Request ID required to encode cancel market data");
            realtime::common::encoders::encode_cancel_market_data(request_id)
        } else {
            // Historical news requests don't need cancellation (they end with HistoricalNewsEnd)
            Err(Error::NotImplemented)
        }
    }
}

/// Requests news providers which the user has subscribed to.
pub(crate) async fn news_providers(client: &Client) -> Result<Vec<NewsProvider>, Error> {
    client.check_server_version(server_versions::REQ_NEWS_PROVIDERS, "It does not support news providers requests.")?;

    let request = encoders::encode_request_news_providers()?;
    let mut subscription = client.send_shared_request(OutgoingMessages::RequestNewsProviders, request).await?;

    match subscription.next().await {
        Some(Ok(message)) => decoders::decode_news_providers(message),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

/// Subscribes to IB's News Bulletins.
pub(crate) async fn news_bulletins(client: &Client, all_messages: bool) -> Result<Subscription<NewsBulletin>, Error> {
    let request = encoders::encode_request_news_bulletins(all_messages)?;
    let internal_subscription = client.send_shared_request(OutgoingMessages::RequestNewsBulletins, request).await?;

    Ok(Subscription::new_from_internal::<NewsBulletin>(
        internal_subscription,
        client.message_bus.clone(),
        None,
        None,
        Some(OutgoingMessages::RequestNewsBulletins),
        client.decoder_context(),
    ))
}

/// Historical News Headlines
pub(crate) async fn historical_news(
    client: &Client,
    contract_id: i32,
    provider_codes: &[&str],
    start_time: OffsetDateTime,
    end_time: OffsetDateTime,
    total_results: u8,
) -> Result<Subscription<NewsArticle>, Error> {
    client.check_server_version(server_versions::REQ_HISTORICAL_NEWS, "It does not support historical news requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_historical_news(
        client.server_version(),
        request_id,
        contract_id,
        provider_codes,
        start_time,
        end_time,
        total_results,
    )?;
    let internal_subscription = client.send_request(request_id, request).await?;

    Ok(Subscription::new_from_internal::<NewsArticle>(
        internal_subscription,
        client.message_bus.clone(),
        Some(request_id),
        None,
        None,
        client.decoder_context(),
    ))
}

/// Requests news article body
pub(crate) async fn news_article(client: &Client, provider_code: &str, article_id: &str) -> Result<NewsArticleBody, Error> {
    client.check_server_version(server_versions::REQ_NEWS_ARTICLE, "It does not support news article requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_news_article(client.server_version(), request_id, provider_code, article_id)?;

    let mut subscription = client.send_request(request_id, request).await?;

    match subscription.next().await {
        Some(Ok(message)) => decoders::decode_news_article(message),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

/// Subscribe to news for a specific contract
pub(crate) async fn contract_news(client: &Client, contract: &Contract, provider_codes: &[&str]) -> Result<Subscription<NewsArticle>, Error> {
    let mut generic_ticks = vec!["mdoff".to_string()];
    for provider in provider_codes {
        generic_ticks.push(format!("292:{provider}"));
    }
    let generic_ticks: Vec<_> = generic_ticks.iter().map(|s| s.as_str()).collect();

    let request_id = client.next_request_id();
    let request = realtime::common::encoders::encode_request_market_data(
        client.server_version(),
        request_id,
        contract,
        generic_ticks.as_slice(),
        false,
        false,
    )?;
    let internal_subscription = client.send_request(request_id, request).await?;

    Ok(Subscription::new_from_internal::<NewsArticle>(
        internal_subscription,
        client.message_bus.clone(),
        Some(request_id),
        None,
        None,
        client.decoder_context().with_request_type(OutgoingMessages::RequestMarketData),
    ))
}

/// Subscribe to broad tape news
pub(crate) async fn broad_tape_news(client: &Client, provider_code: &str) -> Result<Subscription<NewsArticle>, Error> {
    let contract = Contract::news(provider_code);
    let generic_ticks = &["mdoff", "292"];

    let request_id = client.next_request_id();
    let request =
        realtime::common::encoders::encode_request_market_data(client.server_version(), request_id, &contract, generic_ticks, false, false)?;
    let internal_subscription = client.send_request(request_id, request).await?;

    Ok(Subscription::new_from_internal::<NewsArticle>(
        internal_subscription,
        client.message_bus.clone(),
        Some(request_id),
        None,
        None,
        client.decoder_context().with_request_type(OutgoingMessages::RequestMarketData),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::Contract;
    use crate::news::ArticleType;
    use crate::stubs::MessageBusStub;
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

        let results = news_providers(&client).await;
        assert!(results.is_ok(), "failed to request news providers: {}", results.err().unwrap());

        let request_messages = message_bus.request_messages.read().unwrap();
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

    #[tokio::test]
    async fn test_news_bulletins() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["14|1|1|2|Message text|NASDAQ|".to_owned()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let results = news_bulletins(&client, true).await;
        assert!(results.is_ok(), "failed to request news bulletins: {}", results.err().unwrap());

        {
            let request_messages = message_bus.request_messages.read().unwrap();
            assert_eq!(request_messages[0].encode_simple(), "12|1|1|");
        }

        let mut subscription = results.unwrap();
        if let Some(bulletin) = subscription.next().await {
            let bulletin = bulletin.unwrap();
            assert_eq!(bulletin.message_id, 1);
            assert_eq!(bulletin.message_type, 2);
            assert_eq!(bulletin.message, "Message text");
            assert_eq!(bulletin.exchange, "NASDAQ");
        } else {
            panic!("Expected news bulletin");
        }
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

        let results = historical_news(&client, 8314, &["BZ", "DJ"], start_time, end_time, 10).await;
        assert!(results.is_ok(), "failed to request historical news: {}", results.err().unwrap());

        {
            let request_messages = message_bus.request_messages.read().unwrap();
            assert_eq!(
                request_messages[0].encode(),
                "86\09000\08314\0BZ+DJ\020230101 00:00:00 UTC\020230102 00:00:00 UTC\010\0\0"
            );
        }

        let mut subscription = results.unwrap();
        if let Some(article) = subscription.next().await {
            let article = article.unwrap();
            assert_eq!(article.provider_code, "DJ-N");
            assert_eq!(article.article_id, "DJ-N$19985fef");
            assert_eq!(article.headline, "{A:800008,800008,800015:L:Chinese (Simplified and Traditional),Chinese (Simplified and Traditional),en:K:n/a:C:0.9882221817970276}These Stocks Are Moving the Most Today: Honda, Qualcomm, Broadcom, Lilly, ResMed, Tesla, Walmart, Rumble, and More -- Barrons.com");
            assert_eq!(article.extra_data, "");
            assert_eq!(article.time.unix_timestamp(), 1734983100);
        } else {
            panic!("Expected news article");
        }
    }

    #[tokio::test]
    async fn test_news_article() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["83|9000|0|Article text content|".to_owned()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let results = news_article(&client, "BZ", "BZ$123").await;
        assert!(results.is_ok(), "failed to request news article: {}", results.err().unwrap());

        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages[0].encode_simple(), "84|9000|BZ|BZ$123||");

        let article = results.unwrap();
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
        let results = contract_news(&client, &contract, &["BZ", "DJ"]).await;
        assert!(results.is_ok(), "failed to request contract news: {}", results.err().unwrap());

        {
            let request_messages = message_bus.request_messages.read().unwrap();
            assert!(request_messages[0].encode().contains("mdoff,292:BZ,292:DJ"));
        }

        let mut subscription = results.unwrap();
        if let Some(article) = subscription.next().await {
            let article = article.unwrap();
            assert_eq!(article.provider_code, "BZ");
            assert_eq!(article.article_id, "BZ$123");
            assert_eq!(article.headline, "Breaking news headline");
            assert_eq!(article.extra_data, "TSLA:123");
            assert_eq!(article.time.unix_timestamp(), 1672531);
        } else {
            panic!("Expected news article");
        }
    }

    #[tokio::test]
    async fn test_broad_tape_news() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["84|9000|1672531200|BZ|BZ$123|Breaking news headline|TSLA:123|".to_owned()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let results = broad_tape_news(&client, "BZ").await;
        assert!(results.is_ok(), "failed to request broad tape news: {}", results.err().unwrap());

        {
            let request_messages = message_bus.request_messages.read().unwrap();
            assert!(request_messages[0].encode().contains("mdoff,292"));
        }

        let mut subscription = results.unwrap();
        if let Some(article) = subscription.next().await {
            let article = article.unwrap();
            assert_eq!(article.provider_code, "BZ");
            assert_eq!(article.article_id, "BZ$123");
            assert_eq!(article.headline, "Breaking news headline");
            assert_eq!(article.extra_data, "TSLA:123");
            assert_eq!(article.time.unix_timestamp(), 1672531);
        } else {
            panic!("Expected news article");
        }
    }

    #[tokio::test]
    async fn test_news_bulletin_cancellation() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["14|1|1|2|Message text|NASDAQ|".to_owned()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let mut subscription = client.news_bulletins(true).await.unwrap();

        // Read one message to ensure subscription is active
        let _ = subscription.next().await;

        // Verify initial request was sent
        {
            let request_messages = message_bus.request_messages.read().unwrap();
            assert_eq!(request_messages.len(), 1, "Expected 1 request message");
            assert_eq!(request_messages[0].encode_simple(), "12|1|1|");
        }

        // Explicitly cancel the subscription
        subscription.cancel().await;

        // Verify cancel request was sent
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 2, "Expected 2 messages (request + cancel)");
        assert_eq!(request_messages[1].encode_simple(), "13|1|");
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

        // Read one message to ensure subscription is active
        let _ = subscription.next().await;

        // Verify initial request was sent
        {
            let request_messages = message_bus.request_messages.read().unwrap();
            assert_eq!(request_messages.len(), 1, "Expected 1 request message");
        }

        // Explicitly cancel the subscription
        subscription.cancel().await;

        // Verify cancel request was sent (market data cancel)
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 2, "Expected 2 messages (request + cancel)");
        assert_eq!(request_messages[1].encode_simple(), "2|1|9000|");
    }
}
