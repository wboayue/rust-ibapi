use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::market_data::historical::{TickBidAsk, TickLast, TickMidpoint, WhatToShow};
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
///
/// The terminals ([`last`](Self::last), [`mid_point`](Self::mid_point),
/// [`bid_ask`](Self::bid_ask)) send the request and return the whole reply as
/// a `Vec`, oldest tick first as TWS sends them. TWS returns whole seconds, so
/// the `Vec` can be much longer than asked for, except that a
/// [`mid_point`](Self::mid_point) or [`bid_ask`](Self::bid_ask) reply holds
/// one tick of its earliest second; with [`starting`](Self::starting) that
/// tick is stamped before the start. TWS rejects a request for more than
/// 1000. For a forex pair TWS answers a trades request with midpoint
/// ticks, which [`last`](Self::last) does not decode: the call then waits
/// until `one_shot_timeout`, without a limit if it is unset. Use
/// [`mid_point`](Self::mid_point) or [`bid_ask`](Self::bid_ask) (live
/// 2026-11-16, paper; `docs/wire-facts/market-data.md#historical-ticks-reply`).
///
/// # Errors
///
/// Each terminal returns:
/// - the TWS rejection, as [`Error::Notice`], if TWS refuses the request.
/// - [`Error::UnexpectedEndOfStream`] if the connection closes before TWS
///   marks the reply complete.
/// - [`Error::Timeout`] if the reply isn't complete within the client's
///   `one_shot_timeout`.
///
/// Without a `one_shot_timeout`, a terminal waits without a time limit. Any
/// error before the reply is complete cancels the request at TWS.
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
    pub fn ending(mut self, end: OffsetDateTime) -> Self {
        self.end = Some(end);
        self
    }

    /// Override regular- vs extended-hours (defaults to `TradingHours::Regular`).
    pub fn trading_hours(mut self, trading_hours: TradingHours) -> Self {
        self.trading_hours = trading_hours;
        self
    }
}

#[cfg(feature = "sync")]
impl<'a> HistoricalTicksBuilder<'a, crate::client::sync::Client> {
    /// Submit the request and return trade ticks.
    ///
    /// # Errors
    ///
    /// See [the builder's errors](HistoricalTicksBuilder#errors).
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
    /// for tick in &ticks {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn last(self) -> Result<Vec<TickLast>, Error> {
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
    /// # Errors
    ///
    /// See [the builder's errors](HistoricalTicksBuilder#errors).
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
    /// for tick in &ticks {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn mid_point(self) -> Result<Vec<TickMidpoint>, Error> {
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
    /// # Errors
    ///
    /// See [the builder's errors](HistoricalTicksBuilder#errors).
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
    /// for tick in &ticks {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn bid_ask(self, ignore_size: IgnoreSize) -> Result<Vec<TickBidAsk>, Error> {
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
    /// # Errors
    ///
    /// See [the builder's errors](HistoricalTicksBuilder#errors).
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
    ///     let ticks = client
    ///         .historical_ticks(&contract, 100)
    ///         .starting(datetime!(2023-04-15 0:00 UTC))
    ///         .last()
    ///         .await
    ///         .expect("historical ticks request failed");
    ///
    ///     for tick in &ticks {
    ///         println!("{tick:?}");
    ///     }
    /// }
    /// ```
    pub async fn last(self) -> Result<Vec<TickLast>, Error> {
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
    /// # Errors
    ///
    /// See [the builder's errors](HistoricalTicksBuilder#errors).
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
    ///     let ticks = client
    ///         .historical_ticks(&contract, 100)
    ///         .ending(datetime!(2023-04-15 0:00 UTC))
    ///         .mid_point()
    ///         .await
    ///         .expect("historical ticks request failed");
    ///
    ///     for tick in &ticks {
    ///         println!("{tick:?}");
    ///     }
    /// }
    /// ```
    pub async fn mid_point(self) -> Result<Vec<TickMidpoint>, Error> {
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
    /// # Errors
    ///
    /// See [the builder's errors](HistoricalTicksBuilder#errors).
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
    ///     let ticks = client
    ///         .historical_ticks(&contract, 100)
    ///         .starting(datetime!(2023-04-15 0:00 UTC))
    ///         .bid_ask(IgnoreSize::No)
    ///         .await
    ///         .expect("historical ticks request failed");
    ///
    ///     for tick in &ticks {
    ///         println!("{tick:?}");
    ///     }
    /// }
    /// ```
    pub async fn bid_ask(self, ignore_size: IgnoreSize) -> Result<Vec<TickBidAsk>, Error> {
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
