use crate::contracts::Contract;
use crate::market_data::realtime::TickTypes;
use crate::Error;

#[cfg(test)]
mod tests;

/// Builder for creating market data subscriptions with a fluent interface
#[must_use = "MarketDataBuilder does nothing until you call .subscribe()"]
pub struct MarketDataBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    generic_ticks: Vec<String>,
    snapshot: bool,
    regulatory_snapshot: bool,
}

impl<'a, C> MarketDataBuilder<'a, C> {
    /// Creates a new MarketDataBuilder
    pub fn new(client: &'a C, contract: &'a Contract) -> Self {
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
    /// Each value is a numeric IB *generic tick request ID* (the
    /// `genericTickList` parameter on `reqMktData`). To add ticks one at a
    /// time, use [`Self::add_generic_tick`] instead.
    ///
    /// # Arguments
    /// * `ticks` - Slice of generic tick request IDs. Prefer the named
    ///   constants in
    ///   [`crate::market_data::realtime::generic_tick`] over raw numeric
    ///   strings.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "sync")]
    /// # {
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::generic_tick;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let subscription = client
    ///     .market_data(&contract)
    ///     .generic_ticks(&[generic_tick::RT_VOLUME, generic_tick::SHORTABLE])
    ///     .subscribe()
    ///     .expect("subscription failed");
    /// # let _ = subscription;
    /// # }
    /// ```
    ///
    /// See: <https://interactivebrokers.github.io/tws-api/tick_types.html>
    pub fn generic_ticks(mut self, ticks: &[&str]) -> Self {
        self.generic_ticks = ticks.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Append a single generic tick ID to the subscription
    ///
    /// Multiple calls accumulate; use [`Self::generic_ticks`] to replace the
    /// list in one shot. Pairs naturally with conditional composition (e.g.
    /// only add [`generic_tick::SHORTABLE`] for stocks). Prefer the named
    /// constants over raw numeric strings.
    ///
    /// See [`Self::subscribe`] for a runnable end-to-end example.
    ///
    /// [`generic_tick::SHORTABLE`]: crate::market_data::realtime::generic_tick::SHORTABLE
    pub fn add_generic_tick(mut self, tick: impl AsRef<str>) -> Self {
        self.generic_ticks.push(tick.as_ref().to_string());
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

// Sync implementation
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
    /// use ibapi::market_data::realtime::{generic_tick, TickTypes};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let subscription = client.market_data(&contract)
    ///     .add_generic_tick(generic_tick::RT_VOLUME)
    ///     .add_generic_tick(generic_tick::SHORTABLE)
    ///     .subscribe()
    ///     .expect("subscription failed");
    ///
    /// for tick in &subscription {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn subscribe(self) -> Result<crate::subscriptions::sync::Subscription<TickTypes>, Error> {
        let generic_ticks: Vec<&str> = self.generic_ticks.iter().map(|s| s.as_str()).collect();

        crate::market_data::realtime::sync::market_data(self.client, self.contract, &generic_ticks, self.snapshot, self.regulatory_snapshot)
    }
}

// Async implementation
#[cfg(feature = "async")]
impl<'a> MarketDataBuilder<'a, crate::client::r#async::Client> {
    /// Subscribe to market data
    ///
    /// Returns a subscription that yields TickTypes as market data arrives.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::market_data::realtime::generic_tick;
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     let mut subscription = client.market_data(&contract)
    ///         .add_generic_tick(generic_tick::RT_VOLUME)
    ///         .add_generic_tick(generic_tick::SHORTABLE)
    ///         .subscribe()
    ///         .await
    ///         .expect("subscription failed");
    ///
    ///     while let Some(tick) = subscription.next().await {
    ///         println!("{tick:?}");
    ///     }
    /// }
    /// ```
    pub async fn subscribe(self) -> Result<crate::subscriptions::Subscription<TickTypes>, Error> {
        let generic_ticks: Vec<&str> = self.generic_ticks.iter().map(|s| s.as_str()).collect();

        self.client
            .subscribe_market_data(self.contract, &generic_ticks, self.snapshot, self.regulatory_snapshot)
            .await
    }
}
