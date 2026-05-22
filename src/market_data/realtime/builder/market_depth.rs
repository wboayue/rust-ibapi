use crate::contracts::Contract;
use crate::market_data::realtime::MarketDepths;
use crate::market_data::SmartDepth;
use crate::Error;

#[cfg(test)]
#[path = "market_depth_tests.rs"]
mod tests;

/// Builder for level-2 (order book) market-depth subscriptions.
///
/// Defaults: `SmartDepth::No` (single-exchange depth). Pass
/// [`SmartDepth::Yes`] via [`.smart_depth(...)`](Self::smart_depth) to request
/// aggregated depth across exchanges. Call [`.subscribe()`](Self::subscribe)
/// (or `.subscribe().await` on async) to start streaming
/// [`MarketDepths`](crate::market_data::realtime::MarketDepths) updates.
#[must_use = "MarketDepthBuilder does nothing until you call .subscribe()"]
pub struct MarketDepthBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    number_of_rows: i32,
    smart_depth: SmartDepth,
}

impl<'a, C> MarketDepthBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract, number_of_rows: i32) -> Self {
        Self {
            client,
            contract,
            number_of_rows,
            smart_depth: SmartDepth::No,
        }
    }

    /// Override single-exchange vs aggregated depth (defaults to `SmartDepth::No`).
    pub fn smart_depth(mut self, smart_depth: SmartDepth) -> Self {
        self.smart_depth = smart_depth;
        self
    }
}

#[cfg(feature = "sync")]
impl<'a> MarketDepthBuilder<'a, crate::client::sync::Client> {
    /// Submit the subscription and return a stream of
    /// [`MarketDepths`](crate::market_data::realtime::MarketDepths) updates.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::SmartDepth;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let subscription = client
    ///     .market_depth(&contract, 5)
    ///     .smart_depth(SmartDepth::Yes)
    ///     .subscribe()
    ///     .expect("market depth request failed");
    ///
    /// for row in subscription.iter_data().take(20) {
    ///     match row {
    ///         Ok(row) => println!("{row:?}"),
    ///         Err(e) => { eprintln!("error: {e:?}"); break; }
    ///     }
    /// }
    /// ```
    pub fn subscribe(self) -> Result<crate::subscriptions::sync::Subscription<MarketDepths>, Error> {
        crate::market_data::realtime::sync::market_depth(self.client, self.contract, self.number_of_rows, self.smart_depth)
    }
}

#[cfg(feature = "async")]
impl<'a> MarketDepthBuilder<'a, crate::client::r#async::Client> {
    /// Submit the subscription and return a stream of
    /// [`MarketDepths`](crate::market_data::realtime::MarketDepths) updates.
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
    ///     let mut subscription = client
    ///         .market_depth(&contract, 5)
    ///         .smart_depth(SmartDepth::Yes)
    ///         .subscribe()
    ///         .await
    ///         .expect("market depth request failed");
    ///
    ///     while let Some(item) = subscription.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(row)) => println!("{row:?}"),
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
    ///             Err(e) => { eprintln!("error: {e}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn subscribe(self) -> Result<crate::subscriptions::Subscription<MarketDepths>, Error> {
        crate::market_data::realtime::r#async::market_depth(self.client, self.contract, self.number_of_rows, self.smart_depth).await
    }
}
