//! Synchronous implementation of news functionality

use std::sync::Arc;

use time::OffsetDateTime;

use super::common::{decoders, encoders};
use super::*;
use crate::client::blocking::{SharesChannel, Subscription};
use crate::client::sync::Client;
use crate::contracts::Contract;
use crate::market_data::realtime;
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
    /// for article in articles_subscription.iter().take(total_results as usize) {
    ///     println!("- Headline: {}, ID: {}, Provider: {}, Time: {}",
    ///              article.headline, article.article_id, article.provider_code, article.time);
    /// }
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
        let request = encoders::encode_request_historical_news(
            self.server_version,
            request_id,
            contract_id,
            provider_codes,
            start_time,
            end_time,
            total_results,
        )?;
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
        let request = encoders::encode_request_news_article(self.server_version, request_id, provider_code, article_id)?;

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
        let mut generic_ticks = vec!["mdoff".to_string()];
        for provider in provider_codes {
            generic_ticks.push(format!("292:{provider}"));
        }
        let generic_ticks: Vec<_> = generic_ticks.iter().map(|s| s.as_str()).collect();

        let request_id = self.next_request_id();
        let request = realtime::common::encoders::encode_request_market_data(
            self.server_version,
            request_id,
            contract,
            generic_ticks.as_slice(),
            false,
            false,
        )?;
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
        let contract = Contract::news(provider_code);
        let generic_ticks = &["mdoff", "292"];

        let request_id = self.next_request_id();
        let request =
            realtime::common::encoders::encode_request_market_data(self.server_version, request_id, &contract, generic_ticks, false, false)?;
        let subscription = self.send_request(request_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }
}

#[cfg(test)]
mod tests {
    use crate::client::blocking::Client;
    use crate::contracts::Contract;
    use crate::news::ArticleType;
    use crate::{server_versions, stubs::MessageBusStub};
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

        let contract = Contract::stock("TSLA").build();
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
}
