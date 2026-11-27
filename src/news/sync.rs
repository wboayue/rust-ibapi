//! Synchronous implementation of news functionality

use super::common::{self, decoders, encoders};
use super::*;
use crate::client::blocking::{ClientRequestBuilders, Subscription, SubscriptionBuilderExt};
use crate::client::sync::Client;
use crate::common::request_helpers::{self, expect_proto};
use crate::common::retry::blocking::retry_on_connection_reset;
use crate::contracts::Contract;
use crate::contracts::ContractId;
use crate::messages::OutgoingMessages;
use crate::Error;

impl Client {
    /// Requests news providers which the user has subscribed to.
    ///
    /// # Errors
    /// [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::client::blocking::ClientBuilder::one_shot_timeout) when set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let news_providers = client.news_providers().expect("request news providers failed");
    /// for news_provider in &news_providers {
    ///   println!("news provider {news_provider:?}");
    /// }
    /// ```
    pub fn news_providers(&self) -> Result<Vec<NewsProvider>, Error> {
        request_helpers::blocking::one_shot_shared(
            self,
            OutgoingMessages::RequestNewsProviders,
            encoders::encode_request_news_providers,
            expect_proto(decoders::decode_news_providers_proto),
        )
    }

    /// Subscribes to IB's News Bulletins.
    ///
    /// # Arguments
    ///
    /// * `all_messages` - [`AllMessages::Yes`] returns all the existing bulletins for the current day; [`AllMessages::No`] only the new bulletins.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::news::AllMessages;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let news_bulletins = client.news_bulletins(AllMessages::Yes).expect("request news providers failed");
    /// for news_bulletin in &news_bulletins {
    ///   println!("news bulletin {news_bulletin:?}");
    /// }
    /// ```
    pub fn news_bulletins(&self, all_messages: AllMessages) -> Result<Subscription<NewsBulletin>, Error> {
        let request = encoders::encode_request_news_bulletins(all_messages)?;
        self.subscription().send_shared(OutgoingMessages::RequestNewsBulletins, request)
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
    /// use ibapi::client::blocking::Client;
    ///
    /// # fn main() -> Result<(), ibapi::Error> {
    /// let client = Client::connect("127.0.0.1:4002", 100)?;
    ///
    /// let page = client.historical_news(265598, &["DJ-N", "BRFG"]).limit(10).page()?; // AAPL
    /// for article in &page.articles {
    ///     println!("{:?}: {}", article.time, article.headline);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn historical_news<'a>(&'a self, contract_id: impl Into<ContractId>, provider_codes: &'a [&'a str]) -> HistoricalNewsBuilder<'a, Self> {
        HistoricalNewsBuilder::new(self, contract_id, provider_codes)
    }

    /// Requests news article body given articleId.
    ///
    /// # Arguments
    ///
    /// * `provider_code` - Short code indicating news provider, e.g. FLY.
    /// * `article_id`    - ID of the specific article.
    ///
    /// An article type this version doesn't model comes back as [`ArticleType::Unknown`], with
    /// `article_text` as TWS sent it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // can get these using the historical_news method
    /// let provider_code = "DJ-N";
    /// let article_id = "DJ-N$1915168d";
    ///
    /// let article = client.news_article(provider_code, article_id).expect("request news article failed");
    /// println!("{article:?}");
    /// ```
    pub fn news_article(&self, provider_code: &str, article_id: &str) -> Result<NewsArticleBody, Error> {
        request_helpers::blocking::one_shot_by_request_id(
            self,
            |request_id| encoders::encode_request_news_article(request_id, provider_code, article_id),
            expect_proto(decoders::decode_news_article_proto),
        )
    }

    /// Requests realtime contract specific news
    ///
    /// # Arguments
    ///
    /// * `contract`       - Contract for which news is being requested.
    /// * `provider_codes` - Short codes indicating news providers, e.g. DJ-N.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let provider_codes = ["DJ-N"];
    ///
    /// let subscription = client.contract_news(&contract, &provider_codes).expect("request contract news failed");
    /// for article in subscription.iter_data() {
    ///     match article {
    ///         Ok(article) => println!("{article:?}"),
    ///         Err(e) => eprintln!("error: {e}"),
    ///     }
    /// }
    /// ```
    pub fn contract_news(&self, contract: &Contract, provider_codes: &[&str]) -> Result<Subscription<NewsArticle>, Error> {
        let builder = self.request();
        let request = encoders::encode_contract_news_request(builder.request_id(), contract, provider_codes)?;
        builder.send_with_context(request, common::tick_news_context(self.decoder_context()))
    }

    /// Requests realtime BroadTape News
    ///
    /// # Arguments
    ///
    /// * `provider_code` - Short codes indicating news provider, e.g. DJ-N.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let provider_code = "BRFG";
    ///
    /// let subscription = client.broad_tape_news(provider_code).expect("request broad tape news failed");
    /// for article in subscription.iter_data() {
    ///     match article {
    ///         Ok(article) => println!("{article:?}"),
    ///         Err(e) => eprintln!("error: {e}"),
    ///     }
    /// }
    /// ```
    pub fn broad_tape_news(&self, provider_code: &str) -> Result<Subscription<NewsArticle>, Error> {
        let builder = self.request();
        let request = encoders::encode_broad_tape_news_request(builder.request_id(), provider_code)?;
        builder.send_with_context(request, common::tick_news_context(self.decoder_context()))
    }
}

/// The headline stream behind [`HistoricalNewsBuilder::subscribe`].
pub(super) fn historical_news_subscription(request: &HistoricalNewsBuilder<'_, Client>) -> Result<Subscription<NewsArticle>, Error> {
    let builder = request.client.request();
    let message = request.encode(builder.request_id())?;
    builder.send(message)
}

/// The page behind [`HistoricalNewsBuilder::page`].
pub(super) fn historical_news_page(request: &HistoricalNewsBuilder<'_, Client>) -> Result<HistoricalNewsPage, Error> {
    let client = request.client;
    let deadline = request_helpers::blocking::one_shot_deadline(client);
    retry_on_connection_reset(client, deadline, || {
        request_helpers::blocking::check_deadline(deadline)?;
        let builder = client.request();
        let message = request.encode(builder.request_id())?;
        let subscription = builder.send::<HistoricalNewsFrame>(message)?;

        // Only the end frame carries `has_more` and proves the page is
        // complete, so a close before it is an error.
        let mut page = HistoricalNewsPage::default();
        loop {
            match request_helpers::blocking::next_frame(&subscription, deadline)? {
                HistoricalNewsFrame::Article(article) => page.articles.push(article),
                HistoricalNewsFrame::End { has_more } => {
                    page.has_more = has_more;
                    return Ok(page);
                }
            }
        }
    })
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
