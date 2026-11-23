use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::market_data::historical::common::tick_pages::PagedTick;
#[cfg(feature = "async")]
use crate::market_data::historical::r#async::TickSubscription;
use crate::market_data::historical::{TickBidAsk, TickDecoder, TickLast, TickMidpoint, WhatToShow};
use crate::market_data::{IgnoreSize, TradingHours};
use crate::Error;

#[cfg(test)]
#[path = "ticks_tests.rs"]
mod tests;

/// Builder for the historical-ticks API.
///
/// IBKR requires at least one of [`starting`](Self::starting) /
/// [`ending`](Self::ending) to anchor the query. The builder does not
/// pre-flight this — the wire encoder accepts both unset, and IBKR
/// surfaces its own error if neither is provided.
#[must_use = "HistoricalTicksBuilder does nothing until you call .last(), .mid_point(), or .bid_ask(...)"]
pub struct HistoricalTicksBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    number_of_ticks: i32,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    trading_hours: TradingHours,
}

impl<'a, C> HistoricalTicksBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract, number_of_ticks: i32) -> Self {
        Self {
            client,
            contract,
            number_of_ticks,
            start: None,
            end: None,
            trading_hours: TradingHours::Regular,
        }
    }

    /// Anchor the query at the start date (fetch forward in time).
    pub fn starting(mut self, start: OffsetDateTime) -> Self {
        self.start = Some(start);
        self
    }

    /// Anchor the query at the end date (fetch backward in time).
    ///
    /// TWS reads the end to the whole second and excludes that second: ending at 14:30:00.5
    /// returns ticks up to 14:29:59. A reply holds whole seconds, so it can carry more ticks
    /// than requested, except that a bid/ask or midpoint reply holds one tick of its earliest
    /// second. To fetch every tick in a range, use [`between`](Self::between).
    pub fn ending(mut self, end: OffsetDateTime) -> Self {
        self.end = Some(end);
        self
    }

    /// Override regular- vs extended-hours (defaults to `TradingHours::Regular`).
    pub fn trading_hours(mut self, trading_hours: TradingHours) -> Self {
        self.trading_hours = trading_hours;
        self
    }

    /// Fetch every tick in `[start, end]`, both bounds included, by paging backward from `end`.
    ///
    /// The tick count passed to `historical_ticks` becomes the size of each request (TWS serves
    /// up to 1000). The terminals send requests until one reaches back to `start` or returns no
    /// ticks, and return the ticks oldest first in a [`TicksBetween`], which says which of the
    /// two ended the paging. A failed request returns a [`TicksBetweenError`] holding the ticks
    /// fetched before it. [`limit`](HistoricalTicksBetweenBuilder::limit) keeps only the latest
    /// ticks; [`latest`](Self::latest) pages with no start. Any [`starting`](Self::starting) or
    /// [`ending`](Self::ending) set before is ignored.
    ///
    /// # Examples
    #[cfg_attr(
        feature = "sync",
        doc = r#"
```no_run
use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::TradingHours;
use time::macros::datetime;

let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
let contract = Contract::stock("AAPL").build();

let ticks = client
    .historical_ticks(&contract, 1000)
    .between(datetime!(2026-11-13 14:30 UTC), datetime!(2026-11-13 14:35 UTC))
    .trading_hours(TradingHours::Extended)
    .last()
    .expect("historical ticks request failed")
    .ticks;

println!("{} trades", ticks.len());
```
"#
    )]
    #[cfg_attr(
        feature = "async",
        doc = r#"
```no_run
use ibapi::market_data::TradingHours;
use ibapi::prelude::*;
use time::macros::datetime;

#[tokio::main]
async fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    let contract = Contract::stock("AAPL").build();

    let ticks = client
        .historical_ticks(&contract, 1000)
        .between(datetime!(2026-11-13 14:30 UTC), datetime!(2026-11-13 14:35 UTC))
        .trading_hours(TradingHours::Extended)
        .last()
        .await
        .expect("historical ticks request failed")
        .ticks;

    println!("{} trades", ticks.len());
}
```
"#
    )]
    pub fn between(self, start: OffsetDateTime, end: OffsetDateTime) -> HistoricalTicksBetweenBuilder<'a, C> {
        HistoricalTicksBetweenBuilder {
            client: self.client,
            contract: self.contract,
            number_of_ticks: self.number_of_ticks,
            start: Some(start),
            end,
            limit: None,
            trading_hours: self.trading_hours,
        }
    }

    /// Fetch the latest `limit` ticks at or before `end`, by paging backward from `end`.
    ///
    /// Pages as [`between`](Self::between) does, with no start: requests go on until the pages
    /// hold `limit` ticks or one returns no ticks. The result keeps whole seconds, so it can hold
    /// more than `limit` ticks; see [`HistoricalTicksBetweenBuilder::limit`]. A `limit` of 0
    /// sends nothing and returns no ticks.
    ///
    /// # Examples
    #[cfg_attr(
        feature = "sync",
        doc = r#"
```no_run
use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use time::macros::datetime;

let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
let contract = Contract::stock("AAPL").build();

let trades = client
    .historical_ticks(&contract, 1000)
    .latest(2500, datetime!(2026-11-13 21:00 UTC))
    .last()
    .expect("historical ticks request failed");

println!("{} trades from {:?}, stopped at {:?}", trades.ticks.len(), trades.from, trades.stop);
```
"#
    )]
    #[cfg_attr(
        feature = "async",
        doc = r#"
```no_run
use ibapi::prelude::*;
use time::macros::datetime;

#[tokio::main]
async fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    let contract = Contract::stock("AAPL").build();

    let trades = client
        .historical_ticks(&contract, 1000)
        .latest(2500, datetime!(2026-11-13 21:00 UTC))
        .last()
        .await
        .expect("historical ticks request failed");

    println!("{} trades from {:?}, stopped at {:?}", trades.ticks.len(), trades.from, trades.stop);
}
```
"#
    )]
    pub fn latest(self, limit: usize, end: OffsetDateTime) -> HistoricalTicksBetweenBuilder<'a, C> {
        HistoricalTicksBetweenBuilder {
            client: self.client,
            contract: self.contract,
            number_of_ticks: self.number_of_ticks,
            start: None,
            end,
            limit: Some(limit),
            trading_hours: self.trading_hours,
        }
    }
}

/// Builder for historical ticks paged backward from an end time, returned by
/// [`HistoricalTicksBuilder::between`] and [`HistoricalTicksBuilder::latest`].
///
/// Its terminals page backward and return the ticks in `[start, end]`, or the latest ones at or
/// before `end` under a [`limit`](Self::limit), oldest first. A `start` not before `end` fails
/// with [`Error::InvalidArgument`] and nothing is sent.
///
/// A terminal returns [`TicksBetween`] once paging stops, and [`TicksBetweenError`] when a
/// request fails. `?` converts the error into [`Error`], dropping the ticks it holds.
#[must_use = "HistoricalTicksBetweenBuilder does nothing until you call .last(), .mid_point(), or .bid_ask(...)"]
pub struct HistoricalTicksBetweenBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    number_of_ticks: i32,
    start: Option<OffsetDateTime>,
    end: OffsetDateTime,
    limit: Option<usize>,
    trading_hours: TradingHours,
}

impl<C> HistoricalTicksBetweenBuilder<'_, C> {
    /// Override regular- vs extended-hours (defaults to the value set on
    /// [`HistoricalTicksBuilder`], which defaults to `TradingHours::Regular`).
    pub fn trading_hours(mut self, trading_hours: TradingHours) -> Self {
        self.trading_hours = trading_hours;
        self
    }

    /// Keep only the latest `limit` ticks, and stop paging once the pages hold that many.
    ///
    /// The ticks kept are whole seconds: the earliest second kept is never cut, so the result
    /// can hold more than `limit` ticks, by fewer than that second holds. [`TicksBetween::from`]
    /// is then the start of that second and the stop is [`PagingStop::LimitReached`]. A `limit`
    /// smaller than one second's ticks returns that whole second. A `limit` of 0 sends nothing
    /// and returns no ticks.
    ///
    /// # Examples
    #[cfg_attr(
        feature = "sync",
        doc = r#"
```no_run
use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use time::macros::datetime;

let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
let contract = Contract::stock("AAPL").build();

let trades = client
    .historical_ticks(&contract, 1000)
    .between(datetime!(2026-11-13 14:30 UTC), datetime!(2026-11-13 21:00 UTC))
    .limit(2500)
    .last()
    .expect("historical ticks request failed");

println!("{} trades from {:?}, stopped at {:?}", trades.ticks.len(), trades.from, trades.stop);
```
"#
    )]
    #[cfg_attr(
        feature = "async",
        doc = r#"
```no_run
use ibapi::prelude::*;
use time::macros::datetime;

#[tokio::main]
async fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    let contract = Contract::stock("AAPL").build();

    let trades = client
        .historical_ticks(&contract, 1000)
        .between(datetime!(2026-11-13 14:30 UTC), datetime!(2026-11-13 21:00 UTC))
        .limit(2500)
        .last()
        .await
        .expect("historical ticks request failed");

    println!("{} trades from {:?}, stopped at {:?}", trades.ticks.len(), trades.from, trades.stop);
}
```
"#
    )]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[cfg(feature = "sync")]
impl HistoricalTicksBetweenBuilder<'_, crate::client::sync::Client> {
    /// Fetch the trade ticks in the range.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let result = client
    ///     .historical_ticks(&contract, 1000)
    ///     .between(datetime!(2026-11-13 14:30 UTC), datetime!(2026-11-13 14:35 UTC))
    ///     .last();
    ///
    /// match result {
    ///     Ok(trades) => println!("{} trades, stopped at {:?}", trades.ticks.len(), trades.stop),
    ///     Err(e) => println!("{e}: kept {} trades complete from {:?}", e.ticks.len(), e.from),
    /// }
    /// ```
    pub fn last(self) -> Result<TicksBetween<TickLast>, TicksBetweenError<TickLast>> {
        self.fetch(WhatToShow::Trades, false)
    }

    /// Fetch the midpoint ticks in the range.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let ticks = client
    ///     .historical_ticks(&contract, 1000)
    ///     .between(datetime!(2026-11-13 14:30 UTC), datetime!(2026-11-13 14:35 UTC))
    ///     .mid_point()
    ///     .expect("historical ticks request failed")
    ///     .ticks;
    ///
    /// println!("{} midpoints", ticks.len());
    /// ```
    pub fn mid_point(self) -> Result<TicksBetween<TickMidpoint>, TicksBetweenError<TickMidpoint>> {
        self.fetch(WhatToShow::MidPoint, false)
    }

    /// Fetch the bid/ask ticks in the range. Pass [`IgnoreSize::Yes`] to skip updates that
    /// change only a size; see [`IgnoreSize`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::IgnoreSize;
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let ticks = client
    ///     .historical_ticks(&contract, 1000)
    ///     .between(datetime!(2026-11-13 14:30 UTC), datetime!(2026-11-13 14:35 UTC))
    ///     .bid_ask(IgnoreSize::No)
    ///     .expect("historical ticks request failed")
    ///     .ticks;
    ///
    /// println!("{} quotes", ticks.len());
    /// ```
    pub fn bid_ask(self, ignore_size: IgnoreSize) -> Result<TicksBetween<TickBidAsk>, TicksBetweenError<TickBidAsk>> {
        self.fetch(WhatToShow::BidAsk, matches!(ignore_size, IgnoreSize::Yes))
    }

    fn fetch<T: TickDecoder<T> + PagedTick>(self, what_to_show: WhatToShow, ignore_size: bool) -> Result<TicksBetween<T>, TicksBetweenError<T>> {
        crate::market_data::historical::sync::historical_ticks_between::<T>(
            self.client,
            self.contract,
            self.start,
            self.end,
            self.limit,
            self.number_of_ticks,
            what_to_show,
            self.trading_hours,
            ignore_size,
        )
    }
}

#[cfg(feature = "async")]
impl HistoricalTicksBetweenBuilder<'_, crate::client::r#async::Client> {
    /// Fetch the trade ticks in the range.
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
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     let result = client
    ///         .historical_ticks(&contract, 1000)
    ///         .between(datetime!(2026-11-13 14:30 UTC), datetime!(2026-11-13 14:35 UTC))
    ///         .last()
    ///         .await;
    ///
    ///     match result {
    ///         Ok(trades) => println!("{} trades, stopped at {:?}", trades.ticks.len(), trades.stop),
    ///         Err(e) => println!("{e}: kept {} trades complete from {:?}", e.ticks.len(), e.from),
    ///     }
    /// }
    /// ```
    pub async fn last(self) -> Result<TicksBetween<TickLast>, TicksBetweenError<TickLast>> {
        self.fetch(WhatToShow::Trades, false).await
    }

    /// Fetch the midpoint ticks in the range.
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
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     let ticks = client
    ///         .historical_ticks(&contract, 1000)
    ///         .between(datetime!(2026-11-13 14:30 UTC), datetime!(2026-11-13 14:35 UTC))
    ///         .mid_point()
    ///         .await
    ///         .expect("historical ticks request failed")
    ///         .ticks;
    ///
    ///     println!("{} midpoints", ticks.len());
    /// }
    /// ```
    pub async fn mid_point(self) -> Result<TicksBetween<TickMidpoint>, TicksBetweenError<TickMidpoint>> {
        self.fetch(WhatToShow::MidPoint, false).await
    }

    /// Fetch the bid/ask ticks in the range. Pass [`IgnoreSize::Yes`] to skip updates that
    /// change only a size; see [`IgnoreSize`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use ibapi::market_data::IgnoreSize;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     let ticks = client
    ///         .historical_ticks(&contract, 1000)
    ///         .between(datetime!(2026-11-13 14:30 UTC), datetime!(2026-11-13 14:35 UTC))
    ///         .bid_ask(IgnoreSize::No)
    ///         .await
    ///         .expect("historical ticks request failed")
    ///         .ticks;
    ///
    ///     println!("{} quotes", ticks.len());
    /// }
    /// ```
    pub async fn bid_ask(self, ignore_size: IgnoreSize) -> Result<TicksBetween<TickBidAsk>, TicksBetweenError<TickBidAsk>> {
        self.fetch(WhatToShow::BidAsk, matches!(ignore_size, IgnoreSize::Yes)).await
    }

    async fn fetch<T: TickDecoder<T> + PagedTick + Unpin>(
        self,
        what_to_show: WhatToShow,
        ignore_size: bool,
    ) -> Result<TicksBetween<T>, TicksBetweenError<T>> {
        crate::market_data::historical::r#async::historical_ticks_between::<T>(
            self.client,
            self.contract,
            self.start,
            self.end,
            self.limit,
            self.number_of_ticks,
            what_to_show,
            self.trading_hours,
            ignore_size,
        )
        .await
    }
}

/// The ticks a [`HistoricalTicksBetweenBuilder`] terminal fetched, and how the paging stopped.
#[derive(Debug, Clone, PartialEq)]
pub struct TicksBetween<T> {
    /// The ticks in the range, oldest first: in `[start, end]`, or at or before `end` with no
    /// start. Under a limit, the latest whole seconds holding it; see
    /// [`HistoricalTicksBetweenBuilder::limit`].
    pub ticks: Vec<T>,
    /// The time from which [`ticks`](Self::ticks) holds every tick TWS serves up to `end`:
    /// `start` when paging reached it, the earliest second kept when the limit dropped older
    /// ticks, and otherwise the earliest complete second fetched. Always the start of a complete
    /// second unless it is `start`. `None` when no page held a tick.
    pub from: Option<OffsetDateTime>,
    /// Why paging stopped.
    pub stop: PagingStop,
}

/// Why a [`HistoricalTicksBetweenBuilder`] terminal stopped paging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingStop {
    /// A page reached back to `start`: the ticks cover `[start, end]`. A range with no trading,
    /// such as a weekend, ends here with no ticks, as TWS answers with earlier ticks.
    ReachedStart,
    /// A request returned no ticks before its end: [`TicksBetween::from`], or just after `end`
    /// when no page held a tick. TWS serves no earlier ticks for these trading hours, as when
    /// the range starts before the history TWS keeps.
    EmptyPage,
    /// The pages held the [`limit`](HistoricalTicksBetweenBuilder::limit) of ticks. Older ticks
    /// may exist before [`TicksBetween::from`]. Also reported when the pages reached `start` but
    /// the limit dropped ticks after it.
    LimitReached,
}

/// A failed request in a [`HistoricalTicksBetweenBuilder`] terminal, with the ticks fetched
/// before it.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{error} ({} ticks fetched before the failed request)", .ticks.len())]
pub struct TicksBetweenError<T> {
    /// Why the request failed.
    pub error: Error,
    /// The ticks in the range from the pages read before the failure, oldest first: complete
    /// from [`from`](Self::from) to `end`. Fewer than the limit, if one was set.
    pub ticks: Vec<T>,
    /// The earliest complete second fetched, or `None` when no page held a tick.
    pub from: Option<OffsetDateTime>,
}

impl<T> From<TicksBetweenError<T>> for Error {
    fn from(error: TicksBetweenError<T>) -> Self {
        error.error
    }
}

#[cfg(feature = "sync")]
impl<'a> HistoricalTicksBuilder<'a, crate::client::sync::Client> {
    /// Submit the request and return trade ticks.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("TSLA").build();
    ///
    /// let ticks = client
    ///     .historical_ticks(&contract, 100)
    ///     .starting(datetime!(2023-04-15 0:00 UTC))
    ///     .last()
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in ticks.iter_data() {
    ///     println!("{:?}", tick.expect("decode error"));
    /// }
    /// ```
    pub fn last(self) -> Result<crate::market_data::historical::sync::TickSubscription<TickLast>, Error> {
        crate::market_data::historical::sync::historical_ticks::<TickLast>(
            self.client,
            self.contract,
            self.start,
            self.end,
            self.number_of_ticks,
            WhatToShow::Trades,
            self.trading_hours,
            false,
        )
    }

    /// Submit the request and return midpoint ticks.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("TSLA").build();
    ///
    /// let ticks = client
    ///     .historical_ticks(&contract, 100)
    ///     .ending(datetime!(2023-04-15 0:00 UTC))
    ///     .mid_point()
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in ticks.iter_data() {
    ///     println!("{:?}", tick.expect("decode error"));
    /// }
    /// ```
    pub fn mid_point(self) -> Result<crate::market_data::historical::sync::TickSubscription<TickMidpoint>, Error> {
        crate::market_data::historical::sync::historical_ticks::<TickMidpoint>(
            self.client,
            self.contract,
            self.start,
            self.end,
            self.number_of_ticks,
            WhatToShow::MidPoint,
            self.trading_hours,
            false,
        )
    }

    /// Submit the request and return bid/ask ticks. Pass [`IgnoreSize::Yes`]
    /// to skip updates that change only a size; see [`IgnoreSize`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::IgnoreSize;
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("TSLA").build();
    ///
    /// let ticks = client
    ///     .historical_ticks(&contract, 100)
    ///     .starting(datetime!(2023-04-15 0:00 UTC))
    ///     .bid_ask(IgnoreSize::No)
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in ticks.iter_data() {
    ///     println!("{:?}", tick.expect("decode error"));
    /// }
    /// ```
    pub fn bid_ask(self, ignore_size: IgnoreSize) -> Result<crate::market_data::historical::sync::TickSubscription<TickBidAsk>, Error> {
        crate::market_data::historical::sync::historical_ticks::<TickBidAsk>(
            self.client,
            self.contract,
            self.start,
            self.end,
            self.number_of_ticks,
            WhatToShow::BidAsk,
            self.trading_hours,
            matches!(ignore_size, IgnoreSize::Yes),
        )
    }
}

#[cfg(feature = "async")]
impl<'a> HistoricalTicksBuilder<'a, crate::client::r#async::Client> {
    /// Submit the request and return trade ticks.
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
    ///     let contract = Contract::stock("TSLA").build();
    ///
    ///     let mut ticks = client
    ///         .historical_ticks(&contract, 100)
    ///         .starting(datetime!(2023-04-15 0:00 UTC))
    ///         .last()
    ///         .await
    ///         .expect("historical ticks request failed")
    ///         .filter_data();
    ///
    ///     while let Some(tick) = ticks.next().await {
    ///         println!("{:?}", tick.expect("decode error"));
    ///     }
    /// }
    /// ```
    pub async fn last(self) -> Result<TickSubscription<TickLast>, Error> {
        crate::market_data::historical::r#async::historical_ticks::<TickLast>(
            self.client,
            self.contract,
            self.start,
            self.end,
            self.number_of_ticks,
            WhatToShow::Trades,
            self.trading_hours,
            false,
        )
        .await
    }

    /// Submit the request and return midpoint ticks.
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
    ///     let contract = Contract::stock("TSLA").build();
    ///
    ///     let mut ticks = client
    ///         .historical_ticks(&contract, 100)
    ///         .ending(datetime!(2023-04-15 0:00 UTC))
    ///         .mid_point()
    ///         .await
    ///         .expect("historical ticks request failed")
    ///         .filter_data();
    ///
    ///     while let Some(tick) = ticks.next().await {
    ///         println!("{:?}", tick.expect("decode error"));
    ///     }
    /// }
    /// ```
    pub async fn mid_point(self) -> Result<TickSubscription<TickMidpoint>, Error> {
        crate::market_data::historical::r#async::historical_ticks::<TickMidpoint>(
            self.client,
            self.contract,
            self.start,
            self.end,
            self.number_of_ticks,
            WhatToShow::MidPoint,
            self.trading_hours,
            false,
        )
        .await
    }

    /// Submit the request and return bid/ask ticks. Pass [`IgnoreSize::Yes`]
    /// to skip updates that change only a size; see [`IgnoreSize`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use ibapi::market_data::IgnoreSize;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("TSLA").build();
    ///
    ///     let mut ticks = client
    ///         .historical_ticks(&contract, 100)
    ///         .starting(datetime!(2023-04-15 0:00 UTC))
    ///         .bid_ask(IgnoreSize::No)
    ///         .await
    ///         .expect("historical ticks request failed")
    ///         .filter_data();
    ///
    ///     while let Some(tick) = ticks.next().await {
    ///         println!("{:?}", tick.expect("decode error"));
    ///     }
    /// }
    /// ```
    pub async fn bid_ask(self, ignore_size: IgnoreSize) -> Result<TickSubscription<TickBidAsk>, Error> {
        crate::market_data::historical::r#async::historical_ticks::<TickBidAsk>(
            self.client,
            self.contract,
            self.start,
            self.end,
            self.number_of_ticks,
            WhatToShow::BidAsk,
            self.trading_hours,
            matches!(ignore_size, IgnoreSize::Yes),
        )
        .await
    }
}
