//! Asynchronous implementation of news functionality

use super::common::{self, decoders, encoders};
use super::*;
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::subscriptions::Subscription;
use crate::{server_versions, Client, Error};

impl Client {
    /// Requests news providers which the user has subscribed to.
    pub async fn news_providers(&self) -> Result<Vec<NewsProvider>, Error> {
        self.check_server_version(server_versions::REQ_NEWS_PROVIDERS, "It does not support news providers requests.")?;

        let request = encoders::encode_request_news_providers()?;
        let mut subscription = self.send_shared_request(OutgoingMessages::RequestNewsProviders, request).await?;

        match subscription.next().await {
            Some(Ok(message)) => decoders::decode_news_providers(message),
            Some(Err(e)) => Err(e),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    /// Subscribes to IB's News Bulletins.
    pub async fn news_bulletins(&self, all_messages: bool) -> Result<Subscription<NewsBulletin>, Error> {
        let request = encoders::encode_request_news_bulletins(all_messages)?;
        let internal_subscription = self.send_shared_request(OutgoingMessages::RequestNewsBulletins, request).await?;

        Ok(Subscription::new_from_internal::<NewsBulletin>(
            internal_subscription,
            self.message_bus.clone(),
            None,
            None,
            Some(OutgoingMessages::RequestNewsBulletins),
            self.decoder_context(),
        ))
    }

    /// Historical News Headlines
    pub async fn historical_news(
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
        let internal_subscription = self.send_request(request_id, request).await?;

        Ok(Subscription::new_from_internal::<NewsArticle>(
            internal_subscription,
            self.message_bus.clone(),
            Some(request_id),
            None,
            None,
            self.decoder_context(),
        ))
    }

    /// Requests news article body
    pub async fn news_article(&self, provider_code: &str, article_id: &str) -> Result<NewsArticleBody, Error> {
        self.check_server_version(server_versions::REQ_NEWS_ARTICLE, "It does not support news article requests.")?;

        let request_id = self.next_request_id();
        let request = encoders::encode_request_news_article(request_id, provider_code, article_id)?;

        let mut subscription = self.send_request(request_id, request).await?;

        match subscription.next().await {
            Some(Ok(message)) => decoders::decode_news_article(message),
            Some(Err(e)) => Err(e),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    /// Subscribe to news for a specific contract
    pub async fn contract_news(&self, contract: &Contract, provider_codes: &[&str]) -> Result<Subscription<NewsArticle>, Error> {
        let request_id = self.next_request_id();
        let request = common::encode_contract_news_request(request_id, contract, provider_codes)?;
        let internal_subscription = self.send_request(request_id, request).await?;

        Ok(Subscription::new_from_internal::<NewsArticle>(
            internal_subscription,
            self.message_bus.clone(),
            Some(request_id),
            None,
            None,
            self.decoder_context().with_request_type(OutgoingMessages::RequestMarketData),
        ))
    }

    /// Subscribe to broad tape news
    pub async fn broad_tape_news(&self, provider_code: &str) -> Result<Subscription<NewsArticle>, Error> {
        let request_id = self.next_request_id();
        let request = common::encode_broad_tape_news_request(request_id, provider_code)?;
        let internal_subscription = self.send_request(request_id, request).await?;

        Ok(Subscription::new_from_internal::<NewsArticle>(
            internal_subscription,
            self.message_bus.clone(),
            Some(request_id),
            None,
            None,
            self.decoder_context().with_request_type(OutgoingMessages::RequestMarketData),
        ))
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
