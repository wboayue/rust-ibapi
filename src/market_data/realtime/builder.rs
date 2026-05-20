use crate::contracts::{Contract, TagValue};
use crate::market_data::realtime::{Bar, WhatToShow};
use crate::market_data::TradingHours;
use crate::Error;

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;

/// Builder for real-time 5-second bar subscriptions.
///
/// Defaults: `WhatToShow::Trades`, `TradingHours::Regular`, no extra options.
///
/// `BarSize` is intentionally absent from the public surface — TWS only accepts
/// 5-second bars on the wire. A `.bar_size(...)` method can be added
/// non-breakingly if IB ever expands support.
#[must_use = "RealtimeBarsBuilder does nothing until you call .subscribe()"]
pub struct RealtimeBarsBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    what_to_show: WhatToShow,
    trading_hours: TradingHours,
    options: Vec<TagValue>,
}

impl<'a, C> RealtimeBarsBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract) -> Self {
        Self {
            client,
            contract,
            what_to_show: WhatToShow::Trades,
            trading_hours: TradingHours::Regular,
            options: Vec::new(),
        }
    }

    /// Override the data type to stream (defaults to `WhatToShow::Trades`).
    pub fn what_to_show(mut self, what_to_show: WhatToShow) -> Self {
        self.what_to_show = what_to_show;
        self
    }

    /// Override regular- vs extended-hours (defaults to `TradingHours::Regular`).
    pub fn trading_hours(mut self, trading_hours: TradingHours) -> Self {
        self.trading_hours = trading_hours;
        self
    }

    /// Reserved real-time-bars options. Rarely populated by TWS users.
    pub fn options(mut self, options: Vec<TagValue>) -> Self {
        self.options = options;
        self
    }
}

#[cfg(feature = "sync")]
impl<'a> RealtimeBarsBuilder<'a, crate::client::sync::Client> {
    /// Submit the subscription and return a stream of [`Bar`]s.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::WhatToShow;
    /// use ibapi::market_data::TradingHours;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("TSLA").build();
    ///
    /// let subscription = client
    ///     .realtime_bars(&contract)
    ///     .what_to_show(WhatToShow::Trades)
    ///     .trading_hours(TradingHours::Extended)
    ///     .subscribe()
    ///     .expect("realtime bars request failed");
    ///
    /// for (i, bar) in subscription.iter().enumerate().take(60) {
    ///     println!("bar[{i}]: {bar:?}");
    /// }
    /// ```
    pub fn subscribe(self) -> Result<crate::subscriptions::sync::Subscription<Bar>, Error> {
        crate::market_data::realtime::sync::realtime_bars(self.client, self.contract, &self.what_to_show, self.trading_hours, &self.options)
    }
}

#[cfg(feature = "async")]
impl<'a> RealtimeBarsBuilder<'a, crate::client::r#async::Client> {
    /// Submit the subscription and return a stream of [`Bar`]s.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("TSLA").build();
    ///
    ///     let mut subscription = client
    ///         .realtime_bars(&contract)
    ///         .subscribe()
    ///         .await
    ///         .expect("realtime bars request failed");
    ///
    ///     while let Some(item) = subscription.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(bar)) => println!("{bar:?}"),
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
    ///             Err(e) => { eprintln!("error: {e}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn subscribe(self) -> Result<crate::subscriptions::Subscription<Bar>, Error> {
        self.client
            .subscribe_realtime_bars(self.contract, &self.what_to_show, self.trading_hours, &self.options)
            .await
    }
}
