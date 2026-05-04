//! Synchronous implementation of news functionality

use std::sync::Arc;

use time::OffsetDateTime;

use super::common::{self, decoders, encoders};
use super::*;
use crate::client::blocking::{SharesChannel, Subscription};
use crate::client::sync::Client;
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::{server_versions, Error};

impl SharesChannel for Vec<NewsProvider> {}
impl SharesChannel for Subscription<NewsBulletin> {}

impl Client {
    /// Requests news providers which the user has subscribed to.
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
        self.check_server_version(server_versions::REQ_NEWS_PROVIDERS, "It does not support news providers requests.")?;

        let request = encoders::encode_request_news_providers()?;
        let subscription = self.send_shared_request(OutgoingMessages::RequestNewsProviders, request)?;

        match subscription.next() {
            Some(Ok(message)) => decoders::decode_news_providers(message),
            Some(Err(Error::ConnectionReset)) => self.news_providers(),
            Some(Err(e)) => Err(e),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    /// Subscribes to IB's News Bulletins.
    ///
    /// # Arguments
    ///
    /// * `all_messages` - If set to true, will return all the existing bulletins for the current day, set to false to receive only the new bulletins.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let news_bulletins = client.news_bulletins(true).expect("request news providers failed");
    /// for news_bulletin in &news_bulletins {
    ///   println!("news bulletin {news_bulletin:?}");
    /// }
    /// ```
    pub fn news_bulletins(&self, all_messages: bool) -> Result<Subscription<NewsBulletin>, Error> {
        let request = encoders::encode_request_news_bulletins(all_messages)?;
        let subscription = self.send_shared_request(OutgoingMessages::RequestNewsBulletins, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Requests historical news headlines.
    ///
    /// # Arguments
    ///
    /// * `contract_id`    - Contract ID of ticker. See [contract_details](Client::contract_details) for how to retrieve contract ID.
    /// * `provider_codes` - A list of provider codes.
    /// * `start_time`     - Marks the (exclusive) start of the date range.
    /// * `end_time`       - Marks the (inclusive) end of the date range.
    /// * `total_results`  - The maximum number of headlines to fetch (1 – 300)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract; // Or remove if conId is always known
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // Example: Fetch historical news for a known contract ID (e.g., AAPL's conId)
    /// let contract_id = 265598;
    /// let provider_codes = &["DJNL", "BRFG"]; // Example provider codes
    /// // Define a past date range for the news query
    /// let start_time = datetime!(2023-01-01 0:00 UTC);
    /// let end_time = datetime!(2023-01-02 0:00 UTC);
    /// let total_results = 5u8; // Request a small number of articles for the example
    ///
    /// let articles_subscription = client.historical_news(
    ///     contract_id,
    ///     provider_codes,
    ///     start_time,
    ///     end_time,
    ///     total_results,
    /// ).expect("request historical news failed");
    ///
    /// println!("Requested historical news articles:");
    /// for article in articles_subscription.iter_data().take(total_results as usize) {
    ///     let article = article?;
    ///     println!("- Headline: {}, ID: {}, Provider: {}, Time: {}",
    ///              article.headline, article.article_id, article.provider_code, article.time);
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn historical_news(
        &self,
        contract_id: i32,
        provider_codes: &[&str],
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
        total_results: u8,
    ) -> Result<Subscription<NewsArticle>, Error> {
        self.check_server_version(server_versions::REQ_HISTORICAL_NEWS, "It does not support historical news requests.")?;

        let request_id = self.next_request_id();
        let request = encoders::encode_request_historical_news(request_id, contract_id, provider_codes, start_time, end_time, total_results)?;
        let subscription = self.send_request(request_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Requests news article body given articleId.
    ///
    /// # Arguments
    ///
    /// * `provider_code` - Short code indicating news provider, e.g. FLY.
    /// * `article_id`    - ID of the specific article.
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
        self.check_server_version(server_versions::REQ_NEWS_ARTICLE, "It does not support news article requests.")?;

        let request_id = self.next_request_id();
        let request = encoders::encode_request_news_article(request_id, provider_code, article_id)?;

        let subscription = self.send_request(request_id, request)?;
        match subscription.next() {
            Some(Ok(message)) => decoders::decode_news_article(message),
            Some(Err(Error::ConnectionReset)) => self.news_article(provider_code, article_id),
            Some(Err(e)) => Err(e),
            None => Err(Error::UnexpectedEndOfStream),
        }
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
    /// for article in &subscription {
    ///     println!("{article:?}");
    /// }
    /// ```
    pub fn contract_news(&self, contract: &Contract, provider_codes: &[&str]) -> Result<Subscription<NewsArticle>, Error> {
        let request_id = self.next_request_id();
        let request = common::encode_contract_news_request(request_id, contract, provider_codes)?;
        let subscription = self.send_request(request_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
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
    /// for article in &subscription {
    ///     println!("{article:?}");
    /// }
    /// ```
    pub fn broad_tape_news(&self, provider_code: &str) -> Result<Subscription<NewsArticle>, Error> {
        let request_id = self.next_request_id();
        let request = common::encode_broad_tape_news_request(request_id, provider_code)?;
        let subscription = self.send_request(request_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
