use time::OffsetDateTime;

use crate::contracts::Contract;
#[cfg(feature = "async")]
use crate::market_data::historical::r#async::TickSubscription;
use crate::market_data::historical::{TickBidAsk, TickLast, TickMidpoint, WhatToShow};
use crate::market_data::TradingHours;
use crate::Error;

#[cfg(test)]
#[path = "ticks_tests.rs"]
mod tests;

/// Whether the `bid_ask` terminal should drop tick size information.
///
/// This is a wire flag — IBKR only honors it for `BidAsk` ticks, so it lives
/// on the [`HistoricalTicksBuilder::bid_ask`] terminal rather than as a setter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IgnoreSize {
    /// Tick sizes are omitted from the response.
    Yes,
    /// Tick sizes are included in the response.
    No,
}

/// Builder for the historical-ticks API.
///
/// IBKR requires at least one of [`starting`](Self::starting) /
/// [`ending`](Self::ending) to anchor the query. The builder does not
/// pre-flight this — the wire encoder accepts both unset, and IBKR
/// surfaces its own error if neither is provided.
#[must_use = "HistoricalTicksBuilder does nothing until you call .trade(), .mid_point(), or .bid_ask(...)"]
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
    ///     .trade()
    ///     .expect("historical ticks request failed");
    ///
    /// for tick in ticks {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn trade(self) -> Result<crate::market_data::historical::sync::TickSubscription<TickLast>, Error> {
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
    /// for tick in ticks {
    ///     println!("{tick:?}");
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
    /// to drop tick sizes from the response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::IgnoreSize;
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
    /// for tick in ticks {
    ///     println!("{tick:?}");
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
    ///         .trade()
    ///         .await
    ///         .expect("historical ticks request failed");
    ///
    ///     while let Some(tick) = ticks.next().await {
    ///         println!("{tick:?}");
    ///     }
    /// }
    /// ```
    pub async fn trade(self) -> Result<TickSubscription<TickLast>, Error> {
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
    ///         .expect("historical ticks request failed");
    ///
    ///     while let Some(tick) = ticks.next().await {
    ///         println!("{tick:?}");
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
    /// to drop tick sizes from the response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use ibapi::market_data::historical::IgnoreSize;
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
    ///         .expect("historical ticks request failed");
    ///
    ///     while let Some(tick) = ticks.next().await {
    ///         println!("{tick:?}");
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
