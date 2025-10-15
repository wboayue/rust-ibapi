use crate::contracts::Contract;
use crate::market_data::realtime::TickTypes;
use crate::Error;

#[cfg(test)]
mod tests;

/// Builder for creating market data subscriptions with a fluent interface
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

    /// Add generic tick types to subscribe to
    ///
    /// # Arguments
    /// * `ticks` - Array of tick type IDs as strings (e.g., ["233", "236"])
    ///
    /// # Common tick types:
    /// * "100" - Option Volume
    /// * "101" - Option Open Interest
    /// * "104" - Historical Volatility
    /// * "106" - Option Implied Volatility
    /// * "162" - Index Future Premium
    /// * "165" - Miscellaneous Stats
    /// * "221" - Mark Price
    /// * "225" - Auction Values
    /// * "233" - RTVolume
    /// * "236" - Shortable
    /// * "256" - Inventory
    /// * "258" - Fundamental Ratios
    /// * "293" - Trade Count
    /// * "294" - Trade Rate
    /// * "295" - Volume Rate
    /// * "411" - Real-time Historical Volatility
    ///
    /// See: https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#available-tick-types
    pub fn generic_ticks(mut self, ticks: &[&str]) -> Self {
        self.generic_ticks = ticks.iter().map(|s| s.to_string()).collect();
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
    /// use ibapi::market_data::realtime::TickTypes;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let subscription = client.market_data(&contract)
    ///     .generic_ticks(&["233", "236"])
    ///     .subscribe()
    ///     .expect("subscription failed");
    ///
    /// for tick in &subscription {
    ///     println!("{tick:?}");
    /// }
    /// ```
    pub fn subscribe(self) -> Result<crate::subscriptions::sync::Subscription<TickTypes>, Error> {
        let generic_ticks: Vec<&str> = self.generic_ticks.iter().map(|s| s.as_str()).collect();

        crate::market_data::realtime::blocking::market_data(self.client, self.contract, &generic_ticks, self.snapshot, self.regulatory_snapshot)
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
    /// use ibapi::prelude::*;
    /// use ibapi::client::r#async::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     let mut subscription = client.market_data(&contract)
    ///         .generic_ticks(&["233", "236"])
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

        crate::market_data::realtime::market_data(self.client, self.contract, &generic_ticks, self.snapshot, self.regulatory_snapshot).await
    }
}
