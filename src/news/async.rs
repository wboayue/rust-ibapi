//! Asynchronous implementation of news functionality

use super::common::{self, decoders, encoders};
use super::*;
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

    /// Requests historical news headlines.
    ///
    /// # Arguments
    ///
    /// * `contract_id`    - Contract ID of ticker. See [contract_details](Client::contract_details) for how to retrieve contract ID.
    /// * `provider_codes` - A list of provider codes.
    /// * `start_time`     - Marks the (exclusive) start of the date range.
    /// * `end_time`       - Marks the (inclusive) end of the date range.
    /// * `total_results`  - The maximum number of headlines to fetch (1 – 300). TWS returns at most 300 for a
    ///   larger value, without a notice, and rejects 0 with error 321 (live: 2026-11-05).
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
    // Flat by design (#901): all five arguments are required with no meaningful default, so a
    // builder would buy nothing over positional args (docs/rules/style/param-budget.md).
    pub async fn historical_news(
        &self,
        contract_id: i32,
        provider_codes: &[&str],
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
        total_results: u16,
    ) -> Result<Subscription<NewsArticle>, Error> {
        let builder = self.request();
        let request =
            encoders::encode_request_historical_news(builder.request_id(), contract_id, provider_codes, start_time, end_time, total_results)?;
        builder.send(request).await
    }

    /// Requests one page of historical news headlines: the articles and TWS's
    /// `has_more` flag, which [`historical_news`](Self::historical_news)
    /// discards. Waits for TWS's end frame.
    ///
    /// # Arguments
    ///
    /// * `contract_id`    - Contract ID of ticker. See [contract_details](Client::contract_details) for how to retrieve contract ID.
    /// * `provider_codes` - A list of provider codes.
    /// * `start_time`     - Marks the (exclusive) start of the date range.
    /// * `end_time`       - Marks the (inclusive) end of the date range.
    /// * `total_results`  - The maximum number of headlines to fetch (1 – 300). TWS returns at most 300 for a
    ///   larger value, without a notice, and rejects 0 with error 321 (live: 2026-11-05).
    ///
    /// # Errors
    /// A TWS rejection as [`Error::Notice`]; [`Error::UnexpectedEndOfStream`] if the
    /// request's channel closes before the end frame; [`Error::Timeout`] if the page has
    /// not fully arrived within the one-shot timeout (30 seconds by default);
    /// [`Error::ConnectionReset`] if the connection drops on every retry (three),
    /// [`Error::Shutdown`] if the client shuts down. On the async client, a channel that
    /// overflows mid-page fails the read rather than return a short page.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ibapi::Error> {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
    ///
    ///     let page = client
    ///         .historical_news_page(
    ///             265598, // AAPL
    ///             &["DJNL", "BRFG"],
    ///             datetime!(2023-01-01 0:00 UTC),
    ///             datetime!(2023-01-02 0:00 UTC),
    ///             5,
    ///         )
    ///         .await?;
    ///
    ///     for article in &page.articles {
    ///         println!("{:?}: {}", article.time, article.headline);
    ///     }
    ///     if page.has_more {
    ///         println!("more headlines match the range");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    // Flat like `historical_news`: five required arguments, no defaults; see docs/rules/style/param-budget.md.
    pub async fn historical_news_page(
        &self,
        contract_id: i32,
        provider_codes: &[&str],
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
        total_results: u16,
    ) -> Result<HistoricalNewsPage, Error> {
        let deadline = request_helpers::one_shot_deadline(self);
        retry_on_connection_reset(self, deadline, || async {
            request_helpers::check_deadline(deadline)?;
            let builder = self.request();
            let request =
                encoders::encode_request_historical_news(builder.request_id(), contract_id, provider_codes, start_time, end_time, total_results)?;
            let mut subscription = builder.send::<HistoricalNewsFrame>(request).await?;

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

    /// Requests news article body
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] if TWS reports an article type other than text (`0`) or binary (`1`).
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

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
