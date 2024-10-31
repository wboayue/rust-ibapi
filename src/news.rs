use std::default;

use crate::{
    client::{self, ResponseContext, SharesChannel, Subscribable, Subscription},
    messages::{IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage},
    server_versions, Client, Error,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

mod decoders;
mod encoders;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NewsProvider {
    pub code: String,
    pub name: String,
}

/// Requests news providers which the user has subscribed to.
pub(super) fn news_providers(client: &Client) -> Result<Vec<NewsProvider>, Error> {
    client.check_server_version(server_versions::REQ_NEWS_PROVIDERS, "It does not support news providers requests.")?;

    let request = encoders::encode_request_news_providers()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestNewsProviders, request)?;

    match subscription.next() {
        Some(Ok(message)) => decoders::decode_news_providers(message),
        Some(Err(Error::ConnectionReset)) => news_providers(client),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

impl SharesChannel for Vec<NewsProvider> {}

/// IB News Bulletin
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NewsBulletin {
    /// The unique identifier of the news bulletin.
    pub message_id: i32,
    /// The type of the news bulletin. One of: 1 - Regular news bulletin 2 - Exchange no longer available for trading 3 - Exchange is available for trading.
    pub message_type: i32,
    /// The text of the news bulletin.
    pub message: String,
    /// The exchange from which this news bulletin originated.
    pub exchange: String,
}

impl Subscribable<NewsBulletin> for NewsBulletin {
    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<NewsBulletin, Error> {
        match message.message_type() {
            IncomingMessages::NewsBulletins => Ok(decoders::decode_news_bulletin(message.clone())?),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_news_bulletin()
    }
}

// Subscribes to IB's News Bulletins.
pub(super) fn news_bulletins(client: &Client, all_messages: bool) -> Result<Subscription<NewsBulletin>, Error> {
    let request = encoders::encode_request_news_bulletins(all_messages)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestNewsBulletins, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

impl SharesChannel for Subscription<'_, NewsBulletin> {}

/// Returns news headlines for requested contracts.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct HistoricalNews {
    /// The article’s published time.
    pub time: OffsetDateTime,
    /// The provider code for the news article.
    pub provider_code: String,
    /// Identifier used to track the particular article. See [NewsArticle] for more detail.
    pub article_id: String,
    /// Headline of the provided news article.
    pub headline: String,
}

impl Subscribable<HistoricalNews> for HistoricalNews {
    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<HistoricalNews, Error> {
        match message.message_type() {
            IncomingMessages::HistoricalNews => Ok(decoders::decode_historical_news(client.time_zone, message.clone())?),
            IncomingMessages::HistoricalNewsEnd => Err(Error::EndOfStream),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

// Historical News Headlines
pub(super) fn historical_news<'a>(
    client: &'a Client,
    contract_id: i32,
    provider_codes: &[&str],
    start_time: OffsetDateTime,
    end_time: OffsetDateTime,
    total_results: u8,
) -> Result<Subscription<'a, HistoricalNews>, Error> {
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
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
/// The type of news article ([ArticleType::Text] - plain text or html, [ArticleType::Binary] - binary data / pdf)
pub enum ArticleType {
    /// plain text or html
    #[default]
    Text = 0,
    /// binary data / pdf
    Binary = 1,
}

impl From<i32> for ArticleType {
    fn from(value: i32) -> Self {
        match value {
            0 => ArticleType::Text,
            1 => ArticleType::Binary,
            _ => ArticleType::Text,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct NewsArticle {
    /// The type of news article ([ArticleType::Text] - plain text or html, [ArticleType::Binary] - binary data / pdf)
    article_type: ArticleType,
    /// The body of article (if [ArticleType::Binary], the binary data is encoded using the Base64 scheme)
    article_text: String,
}

pub(super) fn news_article(client: &Client, provider_code: &str, article_id: &str) -> Result<NewsArticle, Error> {
    client.check_server_version(server_versions::REQ_NEWS_ARTICLE, "It does not support news article requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_news_article(client.server_version(), request_id, provider_code, article_id)?;

    let subscription = client.send_request(request_id, request)?;
    match subscription.next() {
        Some(Ok(message)) => decoders::decode_news_article(message),
        Some(Err(Error::ConnectionReset)) => news_article(client, provider_code, article_id),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}
