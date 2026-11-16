//! Asynchronous implementation of news functionality

use super::common::{self, decoders, encoders};
use super::*;
use crate::accounts::types::ContractId;
use crate::client::{ClientRequestBuilders, SubscriptionBuilderExt};
use crate::common::request_helpers::{self, expect_proto};
use crate::common::retry::retry_on_connection_reset;
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::subscriptions::Subscription;
use crate::{Client, Error};

impl Client {
    /// Requests news providers which the user has subscribed to.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::ClientBuilder::one_shot_timeout) when set.
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
    /// # Arguments
    ///
    /// * `all_messages` - [`AllMessages::Yes`] returns all the existing bulletins for the current day, then new ones; [`AllMessages::No`] only the new bulletins.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::news::AllMessages;
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let subscription = client.news_bulletins(AllMessages::Yes).await.expect("news_bulletins failed");
    ///     let mut bulletins = subscription.filter_data();
    ///     while let Some(bulletin) = bulletins.next().await {
    ///         println!("{bulletin:?}");
    ///     }
    /// }
    /// ```
    pub async fn news_bulletins(&self, all_messages: AllMessages) -> Result<Subscription<NewsBulletin>, Error> {
        let request = encoders::encode_request_news_bulletins(all_messages)?;
        self.subscription().send_shared(OutgoingMessages::RequestNewsBulletins, request).await
    }

    /// Starts a historical news request for a contract: the newest headlines from
    /// `provider_codes`, narrowed with [`before`](HistoricalNewsBuilder::before) or
    /// [`since`](HistoricalNewsBuilder::since) and sized with
    /// [`limit`](HistoricalNewsBuilder::limit). Send it with
    /// [`page`](HistoricalNewsBuilder::page) for the headlines and TWS's `has_more` flag, or
    /// [`subscribe`](HistoricalNewsBuilder::subscribe) to stream them.
    ///
    /// # Arguments
    ///
    /// * `contract_id`    - Contract ID of ticker. See [contract_details](Client::contract_details) for how to retrieve contract ID.
    /// * `provider_codes` - The provider codes to search, from [news_providers](Client::news_providers).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ibapi::Error> {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
    ///
    ///     let page = client.historical_news(265598, &["DJ-N", "BRFG"]).limit(10).page().await?; // AAPL
    ///     for article in &page.articles {
    ///         println!("{:?}: {}", article.time, article.headline);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn historical_news<'a>(&'a self, contract_id: impl Into<ContractId>, provider_codes: &'a [&'a str]) -> HistoricalNewsBuilder<'a, Self> {
        HistoricalNewsBuilder::new(self, contract_id, provider_codes)
    }

    /// Requests news article body
    ///
    /// An article type this version doesn't model comes back as [`ArticleType::Unknown`], with
    /// `article_text` as TWS sent it.
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
        let request = encoders::encode_contract_news_request(builder.request_id(), contract, provider_codes)?;
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
        let request = encoders::encode_broad_tape_news_request(builder.request_id(), provider_code)?;
        builder
            .send_with_context(request, common::tick_news_context(self.decoder_context()))
            .await
    }
}

/// The headline stream behind [`HistoricalNewsBuilder::subscribe`].
pub(super) async fn historical_news_subscription(request: &HistoricalNewsBuilder<'_, Client>) -> Result<Subscription<NewsArticle>, Error> {
    let builder = request.client.request();
    let message = request.encode(builder.request_id())?;
    builder.send(message).await
}

/// The page behind [`HistoricalNewsBuilder::page`].
pub(super) async fn historical_news_page(request: &HistoricalNewsBuilder<'_, Client>) -> Result<HistoricalNewsPage, Error> {
    let client = request.client;
    let deadline = request_helpers::one_shot_deadline(client);
    retry_on_connection_reset(client, deadline, || async {
        request_helpers::check_deadline(deadline)?;
        let builder = client.request();
        let message = request.encode(builder.request_id())?;
        let mut subscription = builder.send::<HistoricalNewsFrame>(message).await?;

        // Only the end frame carries `has_more` and proves the page is
        // complete, so a close before it is an error.
        let mut page = HistoricalNewsPage::default();
        loop {
            match request_helpers::next_frame(&mut subscription, deadline).await? {
                HistoricalNewsFrame::Article(article) => page.articles.push(article),
                HistoricalNewsFrame::End { has_more } => {
                    page.has_more = has_more;
                    return Ok(page);
                }
            }
        }
    })
    .await
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
