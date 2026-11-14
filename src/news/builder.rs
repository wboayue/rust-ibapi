use time::OffsetDateTime;

use super::HistoricalNewsPage;
use crate::Error;

/// The most headlines TWS returns for one request; a larger `limit` is capped
/// to it without a notice (live: 2026-11-05).
const MAX_HEADLINES: u16 = 300;

/// Which end of the range a historical news request is anchored to.
///
/// TWS takes one bound, not both: it reads `startDateTime` as an exclusive
/// upper bound and ignores `endDateTime` when both are sent, and reads
/// `endDateTime` as an inclusive lower bound only when `startDateTime` is
/// empty (live: 2026-11-14, `docs/wire-facts/news.md`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NewsBound {
    /// Headlines strictly before the time: sent as `startDateTime`.
    Before(OffsetDateTime),
    /// Headlines at or after the time: sent as `endDateTime`.
    Since(OffsetDateTime),
}

/// Builder for a historical news request, from
/// [`Client::historical_news`](crate::Client::historical_news).
///
/// TWS returns the newest matching headlines first, at most [`limit`](Self::limit)
/// of them. With no bound they are the newest overall; [`before`](Self::before)
/// and [`since`](Self::since) set one bound, and the last call wins.
#[must_use = "HistoricalNewsBuilder does nothing until you call .page() or .subscribe()"]
pub struct HistoricalNewsBuilder<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) contract_id: i32,
    pub(crate) provider_codes: &'a [&'a str],
    pub(crate) bound: Option<NewsBound>,
    pub(crate) limit: u16,
}

impl<'a, C> HistoricalNewsBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract_id: i32, provider_codes: &'a [&'a str]) -> Self {
        Self {
            client,
            contract_id,
            provider_codes,
            bound: None,
            limit: MAX_HEADLINES,
        }
    }

    /// Only headlines strictly before `time`, newest first. Replaces [`since`](Self::since).
    ///
    /// To read the next page, pass the oldest headline's `time`. The bound is exclusive, so that
    /// headline is not repeated, but others with the same timestamp are skipped too: TWS
    /// timestamps are often whole minutes.
    pub fn before(mut self, time: OffsetDateTime) -> Self {
        self.bound = Some(NewsBound::Before(time));
        self
    }

    /// Only headlines at or after `time`, newest first. Replaces [`before`](Self::before).
    ///
    /// When more than [`limit`](Self::limit) match, TWS returns the newest of them and sets
    /// [`HistoricalNewsPage::has_more`].
    pub fn since(mut self, time: OffsetDateTime) -> Self {
        self.bound = Some(NewsBound::Since(time));
        self
    }

    /// The most headlines to return, 1 – 300 (default 300). TWS returns at most 300 for a
    /// larger value, without a notice, and rejects 0 with error 321 (live: 2026-11-05).
    pub fn limit(mut self, limit: u16) -> Self {
        self.limit = limit;
        self
    }
}

#[cfg(feature = "sync")]
impl HistoricalNewsBuilder<'_, crate::client::sync::Client> {
    /// Sends the request and waits for the whole page: the headlines and TWS's
    /// [`has_more`](HistoricalNewsPage::has_more) flag.
    ///
    /// # Errors
    /// A TWS rejection as [`Error::Notice`]; [`Error::UnexpectedEndOfStream`] if the
    /// request's channel closes before the end frame; [`Error::Timeout`] if the page has
    /// not fully arrived within the one-shot timeout (30 seconds by default);
    /// [`Error::ConnectionReset`] if the connection drops on every retry (three),
    /// [`Error::Shutdown`] if the client shuts down.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use time::macros::datetime;
    ///
    /// # fn main() -> Result<(), ibapi::Error> {
    /// let client = Client::connect("127.0.0.1:4002", 100)?;
    ///
    /// let page = client
    ///     .historical_news(265598, &["DJ-N", "BRFG"]) // AAPL
    ///     .before(datetime!(2026-11-01 0:00 UTC))
    ///     .limit(50)
    ///     .page()?;
    ///
    /// for article in &page.articles {
    ///     println!("{:?}: {}", article.time, article.headline);
    /// }
    /// if page.has_more {
    ///     println!("more headlines are available");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn page(self) -> Result<HistoricalNewsPage, Error> {
        super::sync::historical_news_page(&self)
    }

    /// Sends the request and streams the headlines; the stream ends after the last one.
    /// [`has_more`](HistoricalNewsPage::has_more) is not available here; use
    /// [`page`](Self::page) for it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use time::macros::datetime;
    ///
    /// # fn main() -> Result<(), ibapi::Error> {
    /// let client = Client::connect("127.0.0.1:4002", 100)?;
    ///
    /// let headlines = client
    ///     .historical_news(265598, &["DJ-N", "BRFG"]) // AAPL
    ///     .since(datetime!(2026-11-13 0:00 UTC))
    ///     .subscribe()?;
    ///
    /// for article in headlines.iter_data() {
    ///     let article = article?;
    ///     println!("{:?}: {}", article.time, article.headline);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn subscribe(self) -> Result<crate::subscriptions::sync::Subscription<super::NewsArticle>, Error> {
        super::sync::historical_news_subscription(&self)
    }
}

#[cfg(feature = "async")]
impl HistoricalNewsBuilder<'_, crate::client::r#async::Client> {
    /// Sends the request and waits for the whole page: the headlines and TWS's
    /// [`has_more`](HistoricalNewsPage::has_more) flag.
    ///
    /// # Errors
    /// A TWS rejection as [`Error::Notice`]; [`Error::UnexpectedEndOfStream`] if the
    /// request's channel closes before the end frame; [`Error::Timeout`] if the page has
    /// not fully arrived within the one-shot timeout (30 seconds by default);
    /// [`Error::ConnectionReset`] if the connection drops on every retry (three),
    /// [`Error::Shutdown`] if the client shuts down. A request channel that overflows
    /// mid-page (see `ClientBuilder::channel_capacity`) fails the read with an
    /// [`Error::Notice`] rather than return a short page.
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
    ///         .historical_news(265598, &["DJ-N", "BRFG"]) // AAPL
    ///         .before(datetime!(2026-11-01 0:00 UTC))
    ///         .limit(50)
    ///         .page()
    ///         .await?;
    ///
    ///     for article in &page.articles {
    ///         println!("{:?}: {}", article.time, article.headline);
    ///     }
    ///     if page.has_more {
    ///         println!("more headlines are available");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn page(self) -> Result<HistoricalNewsPage, Error> {
        super::r#async::historical_news_page(&self).await
    }

    /// Sends the request and streams the headlines; the stream ends after the last one.
    /// [`has_more`](HistoricalNewsPage::has_more) is not available here; use
    /// [`page`](Self::page) for it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use futures::StreamExt;
    /// use ibapi::prelude::*;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ibapi::Error> {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
    ///
    ///     let mut headlines = client
    ///         .historical_news(265598, &["DJ-N", "BRFG"]) // AAPL
    ///         .since(datetime!(2026-11-13 0:00 UTC))
    ///         .subscribe()
    ///         .await?
    ///         .filter_data();
    ///
    ///     while let Some(article) = headlines.next().await {
    ///         let article = article?;
    ///         println!("{:?}: {}", article.time, article.headline);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn subscribe(self) -> Result<crate::subscriptions::r#async::Subscription<super::NewsArticle>, Error> {
        super::r#async::historical_news_subscription(&self).await
    }
}
