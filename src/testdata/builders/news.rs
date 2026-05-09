//! Builders for news-domain request and response messages.
//!
//! `contract_news` and `broad_tape_news` reuse
//! [`market_data::MarketDataRequestBuilder`](super::market_data::MarketDataRequestBuilder),
//! since they ultimately fan out through `encode_request_market_data`.

use super::{RequestEncoder, ResponseProtoEncoder};
use crate::common::test_utils::helpers::constants::{TEST_CONTRACT_ID, TEST_REQ_ID_FIRST};
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::proto::encoders::{some_bool, some_str};
use crate::ToField;
use time::OffsetDateTime;

empty_request_builder!(NewsProvidersRequestBuilder, NewsProvidersRequest, OutgoingMessages::RequestNewsProviders);

empty_request_builder!(
    CancelNewsBulletinsRequestBuilder,
    CancelNewsBulletins,
    OutgoingMessages::CancelNewsBulletin
);

#[derive(Clone, Copy, Debug, Default)]
pub struct NewsBulletinsRequestBuilder {
    pub all_messages: bool,
}

impl NewsBulletinsRequestBuilder {
    pub fn all_messages(mut self, v: bool) -> Self {
        self.all_messages = v;
        self
    }
}

impl RequestEncoder for NewsBulletinsRequestBuilder {
    type Proto = proto::NewsBulletinsRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestNewsBulletins;

    fn to_proto(&self) -> Self::Proto {
        proto::NewsBulletinsRequest {
            all_messages: some_bool(self.all_messages),
        }
    }
}

#[derive(Clone, Debug)]
pub struct HistoricalNewsRequestBuilder {
    pub request_id: i32,
    pub contract_id: i32,
    pub provider_codes: Vec<String>,
    pub start_time: OffsetDateTime,
    pub end_time: OffsetDateTime,
    pub total_results: u8,
}

impl Default for HistoricalNewsRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract_id: TEST_CONTRACT_ID,
            provider_codes: Vec::new(),
            start_time: OffsetDateTime::UNIX_EPOCH,
            end_time: OffsetDateTime::UNIX_EPOCH,
            total_results: 0,
        }
    }
}

impl HistoricalNewsRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract_id(mut self, v: i32) -> Self {
        self.contract_id = v;
        self
    }
    pub fn provider_codes<S: AsRef<str>>(mut self, v: &[S]) -> Self {
        self.provider_codes = v.iter().map(|s| s.as_ref().to_string()).collect();
        self
    }
    pub fn start_time(mut self, v: OffsetDateTime) -> Self {
        self.start_time = v;
        self
    }
    pub fn end_time(mut self, v: OffsetDateTime) -> Self {
        self.end_time = v;
        self
    }
    pub fn total_results(mut self, v: u8) -> Self {
        self.total_results = v;
        self
    }
}

impl RequestEncoder for HistoricalNewsRequestBuilder {
    type Proto = proto::HistoricalNewsRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestHistoricalNews;

    fn to_proto(&self) -> Self::Proto {
        proto::HistoricalNewsRequest {
            req_id: Some(self.request_id),
            con_id: Some(self.contract_id),
            provider_codes: Some(self.provider_codes.join("+")),
            start_date_time: Some(self.start_time.to_field()),
            end_date_time: Some(self.end_time.to_field()),
            total_results: Some(self.total_results as i32),
            historical_news_options: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewsArticleRequestBuilder {
    pub request_id: i32,
    pub provider_code: String,
    pub article_id: String,
}

impl Default for NewsArticleRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            provider_code: String::new(),
            article_id: String::new(),
        }
    }
}

impl NewsArticleRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn provider_code(mut self, v: impl Into<String>) -> Self {
        self.provider_code = v.into();
        self
    }
    pub fn article_id(mut self, v: impl Into<String>) -> Self {
        self.article_id = v.into();
        self
    }
}

impl RequestEncoder for NewsArticleRequestBuilder {
    type Proto = proto::NewsArticleRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestNewsArticle;

    fn to_proto(&self) -> Self::Proto {
        proto::NewsArticleRequest {
            req_id: Some(self.request_id),
            provider_code: Some(self.provider_code.clone()),
            article_id: Some(self.article_id.clone()),
            news_article_options: Default::default(),
        }
    }
}

// =============================================================================
// Response builders
// =============================================================================

/// One row of a `NewsProviders` (msg 85) response.
#[derive(Clone, Debug)]
pub struct NewsProviderEntry {
    pub code: String,
    pub name: String,
}

/// Builder for `NewsProviders` (msg 85) responses.
#[derive(Clone, Debug, Default)]
pub struct NewsProvidersResponse {
    pub providers: Vec<NewsProviderEntry>,
}

impl NewsProvidersResponse {
    pub fn provider(mut self, code: impl Into<String>, name: impl Into<String>) -> Self {
        self.providers.push(NewsProviderEntry {
            code: code.into(),
            name: name.into(),
        });
        self
    }
}

impl ResponseProtoEncoder for NewsProvidersResponse {
    type Proto = proto::NewsProviders;

    fn to_proto(&self) -> Self::Proto {
        proto::NewsProviders {
            news_providers: self
                .providers
                .iter()
                .map(|p| proto::NewsProvider {
                    provider_code: some_str(&p.code),
                    provider_name: some_str(&p.name),
                })
                .collect(),
        }
    }
}

/// Builder for `NewsBulletins` (msg 14) responses.
#[derive(Clone, Debug, Default)]
pub struct NewsBulletinResponse {
    pub message_id: i32,
    pub message_type: i32,
    pub message: String,
    pub exchange: String,
}

impl NewsBulletinResponse {
    pub fn message_id(mut self, v: i32) -> Self {
        self.message_id = v;
        self
    }
    pub fn message_type(mut self, v: i32) -> Self {
        self.message_type = v;
        self
    }
    pub fn message(mut self, v: impl Into<String>) -> Self {
        self.message = v.into();
        self
    }
    pub fn exchange(mut self, v: impl Into<String>) -> Self {
        self.exchange = v.into();
        self
    }
}

impl ResponseProtoEncoder for NewsBulletinResponse {
    type Proto = proto::NewsBulletin;

    fn to_proto(&self) -> Self::Proto {
        proto::NewsBulletin {
            news_msg_id: Some(self.message_id),
            news_msg_type: Some(self.message_type),
            news_message: some_str(&self.message),
            originating_exch: some_str(&self.exchange),
        }
    }
}

/// Builder for `HistoricalNews` (msg 86) responses.
#[derive(Clone, Debug)]
pub struct HistoricalNewsResponse {
    pub request_id: i32,
    pub time: String,
    pub provider_code: String,
    pub article_id: String,
    pub headline: String,
}

impl Default for HistoricalNewsResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            time: String::new(),
            provider_code: String::new(),
            article_id: String::new(),
            headline: String::new(),
        }
    }
}

impl HistoricalNewsResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn time(mut self, v: impl Into<String>) -> Self {
        self.time = v.into();
        self
    }
    pub fn provider_code(mut self, v: impl Into<String>) -> Self {
        self.provider_code = v.into();
        self
    }
    pub fn article_id(mut self, v: impl Into<String>) -> Self {
        self.article_id = v.into();
        self
    }
    pub fn headline(mut self, v: impl Into<String>) -> Self {
        self.headline = v.into();
        self
    }
}

impl ResponseProtoEncoder for HistoricalNewsResponse {
    type Proto = proto::HistoricalNews;

    fn to_proto(&self) -> Self::Proto {
        proto::HistoricalNews {
            req_id: Some(self.request_id),
            time: some_str(&self.time),
            provider_code: some_str(&self.provider_code),
            article_id: some_str(&self.article_id),
            headline: some_str(&self.headline),
        }
    }
}

/// Builder for `HistoricalNewsEnd` (msg 87) responses.
#[derive(Clone, Copy, Debug)]
pub struct HistoricalNewsEndResponse {
    pub request_id: i32,
}

impl Default for HistoricalNewsEndResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
        }
    }
}

impl HistoricalNewsEndResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
}

impl ResponseProtoEncoder for HistoricalNewsEndResponse {
    type Proto = proto::HistoricalNewsEnd;

    fn to_proto(&self) -> Self::Proto {
        proto::HistoricalNewsEnd {
            req_id: Some(self.request_id),
            has_more: None,
        }
    }
}

/// Builder for `NewsArticle` (msg 83) responses.
#[derive(Clone, Debug)]
pub struct NewsArticleResponse {
    pub request_id: i32,
    pub article_type: i32,
    pub article_text: String,
}

impl Default for NewsArticleResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            article_type: 0,
            article_text: String::new(),
        }
    }
}

impl NewsArticleResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn article_type(mut self, v: i32) -> Self {
        self.article_type = v;
        self
    }
    pub fn article_text(mut self, v: impl Into<String>) -> Self {
        self.article_text = v.into();
        self
    }
}

impl ResponseProtoEncoder for NewsArticleResponse {
    type Proto = proto::NewsArticle;

    fn to_proto(&self) -> Self::Proto {
        proto::NewsArticle {
            req_id: Some(self.request_id),
            article_type: Some(self.article_type),
            article_text: some_str(&self.article_text),
        }
    }
}

// =============================================================================
// Entry-point functions
// =============================================================================

pub fn news_providers_request() -> NewsProvidersRequestBuilder {
    NewsProvidersRequestBuilder
}

pub fn news_bulletins_request() -> NewsBulletinsRequestBuilder {
    NewsBulletinsRequestBuilder::default()
}

pub fn cancel_news_bulletins_request() -> CancelNewsBulletinsRequestBuilder {
    CancelNewsBulletinsRequestBuilder
}

pub fn historical_news_request() -> HistoricalNewsRequestBuilder {
    HistoricalNewsRequestBuilder::default()
}

pub fn news_article_request() -> NewsArticleRequestBuilder {
    NewsArticleRequestBuilder::default()
}

pub fn news_providers() -> NewsProvidersResponse {
    NewsProvidersResponse::default()
}

pub fn news_bulletin() -> NewsBulletinResponse {
    NewsBulletinResponse::default()
}

pub fn historical_news() -> HistoricalNewsResponse {
    HistoricalNewsResponse::default()
}

pub fn historical_news_end() -> HistoricalNewsEndResponse {
    HistoricalNewsEndResponse::default()
}

pub fn news_article() -> NewsArticleResponse {
    NewsArticleResponse::default()
}
