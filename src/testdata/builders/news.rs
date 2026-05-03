//! Builders for news-domain request messages.
//!
//! Response builders are intentionally absent: news responses use IB's text
//! wire format, and the existing inline literals in the migrated sync/async
//! tests already exercise the production decoders end-to-end.
//!
//! `contract_news` and `broad_tape_news` reuse
//! [`market_data::MarketDataRequestBuilder`](super::market_data::MarketDataRequestBuilder),
//! since they ultimately fan out through `encode_request_market_data`.

use super::RequestEncoder;
use crate::common::test_utils::helpers::constants::{TEST_CONTRACT_ID, TEST_REQ_ID_FIRST};
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::proto::encoders::some_bool;
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
