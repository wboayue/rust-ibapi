//! Asynchronous implementation of news functionality

use super::common::{self, decoders, encoders};
use super::*;
use crate::client::{ClientRequestBuilders, SubscriptionBuilderExt};
use crate::common::request_helpers::{self, expect_proto};
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::subscriptions::Subscription;
use crate::{server_versions, Client, Error};

impl Client {
    /// Requests news providers which the user has subscribed to.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let providers = client.news_providers().await.expect("news_providers failed");
    ///     for provider in providers {
    ///         println!("{provider:?}");
    ///     }
    /// }
    /// ```
    pub async fn news_providers(&self) -> Result<Vec<NewsProvider>, Error> {
        self.check_server_version(server_versions::REQ_NEWS_PROVIDERS, "It does not support news providers requests.")?;

        request_helpers::one_shot_shared(
            self,
            OutgoingMessages::RequestNewsProviders,
            encoders::encode_request_news_providers,
            expect_proto(decoders::decode_news_providers_proto),
        )
        .await
    }

    /// Subscribes to IB's News Bulletins.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let subscription = client.news_bulletins(true).await.expect("news_bulletins failed");
    ///     let mut bulletins = subscription.filter_data();
    ///     while let Some(bulletin) = bulletins.next().await {
    ///         println!("{bulletin:?}");
    ///     }
    /// }
    /// ```
    pub async fn news_bulletins(&self, all_messages: bool) -> Result<Subscription<NewsBulletin>, Error> {
        let request = encoders::encode_request_news_bulletins(all_messages)?;
        self.subscription().send_shared(OutgoingMessages::RequestNewsBulletins, request).await
    }

    /// Historical News Headlines
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let subscription = client
    ///         .historical_news(
    ///             8314, // IBM
    ///             &["BRFG"],
    ///             datetime!(2025-01-01 0:00 UTC),
    ///             datetime!(2025-01-31 23:59 UTC),
    ///             10,
    ///         )
    ///         .await
    ///         .expect("historical_news failed");
    ///
    ///     let mut articles = subscription.filter_data();
    ///     while let Some(article) = articles.next().await {
    ///         println!("{article:?}");
    ///     }
    /// }
    /// ```
    pub async fn historical_news(
        &self,
        contract_id: i32,
        provider_codes: &[&str],
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
        total_results: u8,
    ) -> Result<Subscription<NewsArticle>, Error> {
        self.check_server_version(server_versions::REQ_HISTORICAL_NEWS, "It does not support historical news requests.")?;

        let builder = self.request();
        let request =
            encoders::encode_request_historical_news(builder.request_id(), contract_id, provider_codes, start_time, end_time, total_results)?;
        builder.send(request).await
    }

    /// Requests news article body
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let article = client
    ///         .news_article("BRFG", "BRFG$0a3e3f54")
    ///         .await
    ///         .expect("news_article failed");
    ///     println!("{article:?}");
    /// }
    /// ```
    pub async fn news_article(&self, provider_code: &str, article_id: &str) -> Result<NewsArticleBody, Error> {
        self.check_server_version(server_versions::REQ_NEWS_ARTICLE, "It does not support news article requests.")?;

        request_helpers::one_shot_by_request_id(
            self,
            |request_id| encoders::encode_request_news_article(request_id, provider_code, article_id),
            expect_proto(decoders::decode_news_article_proto),
        )
        .await
    }

    /// Subscribe to news for a specific contract
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///     let subscription = client.contract_news(&contract, &["BRFG"]).await.expect("contract_news failed");
    ///     let mut articles = subscription.filter_data();
    ///     while let Some(article) = articles.next().await {
    ///         println!("{article:?}");
    ///     }
    /// }
    /// ```
    pub async fn contract_news(&self, contract: &Contract, provider_codes: &[&str]) -> Result<Subscription<NewsArticle>, Error> {
        let builder = self.request();
        let request = common::encode_contract_news_request(builder.request_id(), contract, provider_codes)?;
        builder
            .send_with_context(request, common::tick_news_context(self.decoder_context()))
            .await
    }

    /// Subscribe to broad tape news
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let subscription = client.broad_tape_news("BRFG").await.expect("broad_tape_news failed");
    ///     let mut articles = subscription.filter_data();
    ///     while let Some(article) = articles.next().await {
    ///         println!("{article:?}");
    ///     }
    /// }
    /// ```
    pub async fn broad_tape_news(&self, provider_code: &str) -> Result<Subscription<NewsArticle>, Error> {
        let builder = self.request();
        let request = common::encode_broad_tape_news_request(builder.request_id(), provider_code)?;
        builder
            .send_with_context(request, common::tick_news_context(self.decoder_context()))
            .await
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
