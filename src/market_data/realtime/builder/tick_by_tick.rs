use crate::contracts::Contract;
use crate::market_data::realtime::{BidAsk, MidPoint, Trade};
use crate::market_data::IgnoreSize;
use crate::Error;

#[cfg(test)]
#[path = "tick_by_tick_tests.rs"]
mod tests;

/// Builder for tick-by-tick real-time subscriptions.
///
/// Pick a tick stream with the terminal method matching the data you want:
/// [`.last()`](Self::last) / [`.all_last()`](Self::all_last) /
/// [`.bid_ask(...)`](Self::bid_ask) / [`.mid_point()`](Self::mid_point). Each
/// terminal returns its own correctly-typed [`Subscription`] —
/// [`Trade`](crate::market_data::realtime::Trade) for the two trade variants,
/// [`BidAsk`](crate::market_data::realtime::BidAsk) for bid/ask, and
/// [`MidPoint`](crate::market_data::realtime::MidPoint) for mid-point.
///
/// `ignore_size` is only meaningful for the bid/ask stream — IBKR ignores the
/// flag on the trade and mid-point streams — so it lives on the
/// [`.bid_ask(...)`](Self::bid_ask) terminal alone.
///
/// [`Subscription`]: crate::subscriptions::Subscription
#[must_use = "TickByTickBuilder does nothing until you call .last(), .all_last(), .bid_ask(...), or .mid_point()"]
pub struct TickByTickBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    number_of_ticks: i32,
}

impl<'a, C> TickByTickBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract, number_of_ticks: i32) -> Self {
        Self {
            client,
            contract,
            number_of_ticks,
        }
    }
}

#[cfg(feature = "sync")]
impl<'a> TickByTickBuilder<'a, crate::client::sync::Client> {
    /// Subscribe to the `Last` trade stream.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let trades = client
    ///     .tick_by_tick(&contract, 10)
    ///     .last()
    ///     .expect("tick-by-tick last request failed");
    ///
    /// for trade in trades.iter().take(10) {
    ///     println!("{trade:?}");
    /// }
    /// ```
    pub fn last(self) -> Result<crate::subscriptions::sync::Subscription<Trade>, Error> {
        crate::market_data::realtime::sync::tick_by_tick::<Trade>(self.client, self.contract, "Last", self.number_of_ticks, false)
    }

    /// Subscribe to the `AllLast` trade stream (includes special-condition trades).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let trades = client
    ///     .tick_by_tick(&contract, 10)
    ///     .all_last()
    ///     .expect("tick-by-tick all-last request failed");
    ///
    /// for trade in trades.iter().take(10) {
    ///     println!("{trade:?}");
    /// }
    /// ```
    pub fn all_last(self) -> Result<crate::subscriptions::sync::Subscription<Trade>, Error> {
        crate::market_data::realtime::sync::tick_by_tick::<Trade>(self.client, self.contract, "AllLast", self.number_of_ticks, false)
    }

    /// Subscribe to the `BidAsk` stream. Pass [`IgnoreSize::Yes`] to drop tick
    /// sizes from the response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::IgnoreSize;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let quotes = client
    ///     .tick_by_tick(&contract, 10)
    ///     .bid_ask(IgnoreSize::No)
    ///     .expect("tick-by-tick bid/ask request failed");
    ///
    /// for quote in quotes.iter().take(10) {
    ///     println!("{quote:?}");
    /// }
    /// ```
    pub fn bid_ask(self, ignore_size: IgnoreSize) -> Result<crate::subscriptions::sync::Subscription<BidAsk>, Error> {
        crate::market_data::realtime::sync::tick_by_tick::<BidAsk>(
            self.client,
            self.contract,
            "BidAsk",
            self.number_of_ticks,
            matches!(ignore_size, IgnoreSize::Yes),
        )
    }

    /// Subscribe to the `MidPoint` stream.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let midpoints = client
    ///     .tick_by_tick(&contract, 10)
    ///     .mid_point()
    ///     .expect("tick-by-tick mid-point request failed");
    ///
    /// for mp in midpoints.iter().take(10) {
    ///     println!("{mp:?}");
    /// }
    /// ```
    pub fn mid_point(self) -> Result<crate::subscriptions::sync::Subscription<MidPoint>, Error> {
        crate::market_data::realtime::sync::tick_by_tick::<MidPoint>(self.client, self.contract, "MidPoint", self.number_of_ticks, false)
    }
}

#[cfg(feature = "async")]
impl<'a> TickByTickBuilder<'a, crate::client::r#async::Client> {
    /// Subscribe to the `Last` trade stream.
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
    ///
    ///     let mut trades = client
    ///         .tick_by_tick(&contract, 10)
    ///         .last()
    ///         .await
    ///         .expect("tick-by-tick last request failed");
    ///
    ///     while let Some(item) = trades.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(t)) => println!("{t:?}"),
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
    ///             Err(e) => { eprintln!("error: {e}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn last(self) -> Result<crate::subscriptions::Subscription<Trade>, Error> {
        crate::market_data::realtime::r#async::tick_by_tick::<Trade>(self.client, self.contract, "Last", self.number_of_ticks, false).await
    }

    /// Subscribe to the `AllLast` trade stream (includes special-condition trades).
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
    ///
    ///     let mut trades = client
    ///         .tick_by_tick(&contract, 10)
    ///         .all_last()
    ///         .await
    ///         .expect("tick-by-tick all-last request failed");
    ///
    ///     while let Some(item) = trades.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(t)) => println!("{t:?}"),
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
    ///             Err(e) => { eprintln!("error: {e}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn all_last(self) -> Result<crate::subscriptions::Subscription<Trade>, Error> {
        crate::market_data::realtime::r#async::tick_by_tick::<Trade>(self.client, self.contract, "AllLast", self.number_of_ticks, false).await
    }

    /// Subscribe to the `BidAsk` stream. Pass [`IgnoreSize::Yes`] to drop tick
    /// sizes from the response.
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
    ///
    ///     let mut quotes = client
    ///         .tick_by_tick(&contract, 10)
    ///         .bid_ask(IgnoreSize::No)
    ///         .await
    ///         .expect("tick-by-tick bid/ask request failed");
    ///
    ///     while let Some(item) = quotes.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(q)) => println!("{q:?}"),
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
    ///             Err(e) => { eprintln!("error: {e}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn bid_ask(self, ignore_size: IgnoreSize) -> Result<crate::subscriptions::Subscription<BidAsk>, Error> {
        crate::market_data::realtime::r#async::tick_by_tick::<BidAsk>(
            self.client,
            self.contract,
            "BidAsk",
            self.number_of_ticks,
            matches!(ignore_size, IgnoreSize::Yes),
        )
        .await
    }

    /// Subscribe to the `MidPoint` stream.
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
    ///
    ///     let mut midpoints = client
    ///         .tick_by_tick(&contract, 10)
    ///         .mid_point()
    ///         .await
    ///         .expect("tick-by-tick mid-point request failed");
    ///
    ///     while let Some(item) = midpoints.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(m)) => println!("{m:?}"),
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
    ///             Err(e) => { eprintln!("error: {e}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn mid_point(self) -> Result<crate::subscriptions::Subscription<MidPoint>, Error> {
        crate::market_data::realtime::r#async::tick_by_tick::<MidPoint>(self.client, self.contract, "MidPoint", self.number_of_ticks, false).await
    }
}
