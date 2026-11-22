use std::time::Duration;

use crate::contracts::Contract;
use crate::market_data::realtime::{GenericTick, TickTypes};
use crate::Error;

#[cfg(test)]
#[path = "market_data_tests.rs"]
mod tests;

/// Builder for creating market data subscriptions with a fluent interface.
///
/// Defaults: no generic ticks, streaming (not snapshot), no regulatory
/// snapshot. Terminals: `.subscribe()`, or `.snapshot_once(timeout)` for a
/// collected one-shot snapshot.
#[must_use = "MarketDataBuilder does nothing until you call .subscribe()"]
pub struct MarketDataBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    generic_ticks: Vec<GenericTick>,
    snapshot: bool,
    regulatory_snapshot: bool,
}

impl<'a, C> MarketDataBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract) -> Self {
        Self {
            client,
            contract,
            generic_ticks: Vec::new(),
            snapshot: false,
            regulatory_snapshot: false,
        }
    }

    /// Replace the generic tick list to subscribe to
    ///
    /// Each value is an IB *generic tick request ID* (the `genericTickList`
    /// parameter on `reqMktData`). To add ticks one at a time, use
    /// [`Self::add_generic_tick`] instead.
    ///
    /// # Arguments
    /// * `ticks` - The generic tick request IDs: [`GenericTick`] constants, or
    ///   the numbers themselves (`[233, 236]`) for an ID without a constant.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "sync")]
    /// # {
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::GenericTick;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let subscription = client
    ///     .market_data(&contract)
    ///     .generic_ticks([GenericTick::RT_VOLUME, GenericTick::SHORTABLE])
    ///     .subscribe()
    ///     .expect("subscription failed");
    /// # let _ = subscription;
    /// # }
    /// ```
    ///
    /// See: <https://www.interactivebrokers.com/docs/tws-api/doc/market-data-live/available-tick-types/introduction>
    pub fn generic_ticks<I>(mut self, ticks: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<GenericTick>,
    {
        self.generic_ticks = ticks.into_iter().map(Into::into).collect();
        self
    }

    /// Append a single generic tick ID to the subscription
    ///
    /// Multiple calls accumulate; use [`Self::generic_ticks`] to replace the
    /// list in one shot. Pairs naturally with conditional composition (e.g.
    /// only add [`GenericTick::SHORTABLE`] for stocks). Takes a [`GenericTick`]
    /// constant, or the number itself for an ID without a constant.
    ///
    /// See [`Self::subscribe`] for a runnable end-to-end example.
    ///
    /// [`GenericTick::SHORTABLE`]: crate::market_data::realtime::GenericTick::SHORTABLE
    pub fn add_generic_tick(mut self, tick: impl Into<GenericTick>) -> Self {
        self.generic_ticks.push(tick.into());
        self
    }

    /// Request a one-time snapshot of market data
    ///
    /// When enabled, the subscription will receive current market data once
    /// and then automatically end with a SnapshotEnd tick type.
    pub fn snapshot(mut self) -> Self {
        self.snapshot = true;
        self
    }

    /// Request regulatory snapshot
    ///
    /// For U.S. stocks, a regulatory snapshot request requires the
    /// subscription of Market Data for US Securities and Futures Snapshot Bundle.
    pub fn regulatory_snapshot(mut self) -> Self {
        self.regulatory_snapshot = true;
        self
    }

    /// Enable real-time streaming data (default)
    ///
    /// This is the default behavior - data will stream continuously
    /// until the subscription is cancelled.
    pub fn streaming(mut self) -> Self {
        self.snapshot = false;
        self
    }
}

#[cfg(feature = "sync")]
impl<'a> MarketDataBuilder<'a, crate::client::sync::Client> {
    /// Subscribe to market data
    ///
    /// Returns a subscription that yields TickTypes as market data arrives.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{GenericTick, TickTypes};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let subscription = client.market_data(&contract)
    ///     .add_generic_tick(GenericTick::RT_VOLUME)
    ///     .add_generic_tick(GenericTick::SHORTABLE)
    ///     .subscribe()
    ///     .expect("subscription failed");
    ///
    /// for tick in subscription.iter_data() {
    ///     match tick {
    ///         Ok(tick) => println!("{tick:?}"),
    ///         Err(e) => eprintln!("error: {e}"),
    ///     }
    /// }
    /// ```
    pub fn subscribe(self) -> Result<crate::subscriptions::sync::Subscription<TickTypes>, Error> {
        crate::market_data::realtime::sync::market_data(self.client, self.contract, &self.generic_ticks, self.snapshot, self.regulatory_snapshot)
    }

    /// Request a one-shot snapshot and collect the resulting ticks.
    ///
    /// Forces [`snapshot`](Self::snapshot) mode, subscribes, and collects every
    /// tick until the snapshot completes (or `timeout` elapses), returning the
    /// accumulated [`TickTypes`]; fold them with
    /// [`Quote::from_ticks`](crate::market_data::realtime::Quote::from_ticks).
    /// This replaces the hand-written collect-with-timeout loop. See
    /// [`Subscription::collect_for`](crate::subscriptions::sync::Subscription::collect_for)
    /// for the underlying terminal and its stop conditions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::Quote;
    /// use std::time::Duration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let ticks = client.market_data(&contract).snapshot_once(Duration::from_secs(5)).expect("snapshot failed");
    /// let quote = Quote::from_ticks(&ticks);
    /// println!("bid {:?} ask {:?} last {:?}", quote.bid, quote.ask, quote.last);
    /// ```
    pub fn snapshot_once(mut self, timeout: Duration) -> Result<Vec<TickTypes>, Error> {
        self.snapshot = true;
        let subscription = self.subscribe()?;
        Ok(subscription.collect_for(timeout))
    }
}

#[cfg(feature = "async")]
impl<'a> MarketDataBuilder<'a, crate::client::r#async::Client> {
    /// Subscribe to market data
    ///
    /// Returns a subscription that yields TickTypes as market data arrives.
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
    ///     let mut subscription = client.market_data(&contract)
    ///         .add_generic_tick(GenericTick::RT_VOLUME)
    ///         .add_generic_tick(GenericTick::SHORTABLE)
    ///         .subscribe()
    ///         .await
    ///         .expect("subscription failed");
    ///
    ///     while let Some(tick) = subscription.next().await {
    ///         println!("{tick:?}");
    ///     }
    /// }
    /// ```
    pub async fn subscribe(self) -> Result<crate::subscriptions::r#async::Subscription<TickTypes>, Error> {
        crate::market_data::realtime::r#async::market_data(self.client, self.contract, &self.generic_ticks, self.snapshot, self.regulatory_snapshot)
            .await
    }

    /// Request a one-shot snapshot and collect the resulting ticks.
    ///
    /// Forces [`snapshot`](Self::snapshot) mode, subscribes, and collects every
    /// tick until the snapshot completes (or `timeout` elapses), returning the
    /// accumulated [`TickTypes`]; fold them with
    /// [`Quote::from_ticks`](crate::market_data::realtime::Quote::from_ticks).
    /// This replaces the hand-written collect-with-timeout loop. See
    /// [`Subscription::collect_for`](crate::subscriptions::Subscription::collect_for)
    /// for the underlying terminal and its stop conditions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     let ticks = client.market_data(&contract).snapshot_once(Duration::from_secs(5)).await.expect("snapshot failed");
    ///     let quote = Quote::from_ticks(&ticks);
    ///     println!("bid {:?} ask {:?} last {:?}", quote.bid, quote.ask, quote.last);
    /// }
    /// ```
    pub async fn snapshot_once(mut self, timeout: Duration) -> Result<Vec<TickTypes>, Error> {
        self.snapshot = true;
        let mut subscription = self.subscribe().await?;
        Ok(subscription.collect_for(timeout).await)
    }
}
