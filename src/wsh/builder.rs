//! Fluent builders for the Wall Street Horizon event-data API.
//!
//! Same shape as the historical-data builders: generic over `'a` + client type,
//! defaults in `new()`, `mut self`-returning setters, per-feature terminal
//! `impl` blocks. See
//! [`HistoricalScheduleBuilder`](crate::market_data::historical::HistoricalScheduleBuilder)
//! for the canonical example.
//!
//! Both entry points took one required argument and two to four `Option`s that
//! every caller in the tree — examples and integration tests alike — passed as
//! `None`. See [param budget](../../docs/rules/style/param-budget.md).

use time::Date;

use crate::wsh::{AutoFill, WshEventData};
use crate::Error;

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;

/// Builder for Wall Street Horizon events on one contract.
#[must_use = "WshEventDataBuilder does nothing until you call .fetch()"]
pub struct WshEventDataBuilder<'a, C> {
    client: &'a C,
    contract_id: i32,
    start_date: Option<Date>,
    end_date: Option<Date>,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
}

impl<'a, C> WshEventDataBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract_id: i32) -> Self {
        Self {
            client,
            contract_id,
            start_date: None,
            end_date: None,
            limit: None,
            auto_fill: None,
        }
    }

    /// Return events on or after `start_date`.
    ///
    /// Requires `server_versions::WSH_EVENT_DATA_FILTERS_DATE`.
    pub fn starting(mut self, start_date: Date) -> Self {
        self.start_date = Some(start_date);
        self
    }

    /// Return events on or before `end_date`.
    ///
    /// Requires `server_versions::WSH_EVENT_DATA_FILTERS_DATE`.
    pub fn ending(mut self, end_date: Date) -> Self {
        self.end_date = Some(end_date);
        self
    }

    /// Cap the number of events returned. TWS allows at most 100.
    ///
    /// Requires `server_versions::WSH_EVENT_DATA_FILTERS_DATE`.
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Ask TWS to fill event data for related contracts. See [`AutoFill`].
    ///
    /// Requires `server_versions::WSH_EVENT_DATA_FILTERS`.
    pub fn auto_fill(mut self, auto_fill: AutoFill) -> Self {
        self.auto_fill = Some(auto_fill);
        self
    }
}

/// Builder for Wall Street Horizon events matching a JSON filter.
#[must_use = "WshEventFilterBuilder does nothing until you call .subscribe()"]
pub struct WshEventFilterBuilder<'a, C> {
    client: &'a C,
    filter: &'a str,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
}

impl<'a, C> WshEventFilterBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, filter: &'a str) -> Self {
        Self {
            client,
            filter,
            limit: None,
            auto_fill: None,
        }
    }

    /// Cap the number of events returned. TWS allows at most 100.
    ///
    /// Requires `server_versions::WSH_EVENT_DATA_FILTERS_DATE`.
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Ask TWS to fill event data for related contracts. See [`AutoFill`].
    pub fn auto_fill(mut self, auto_fill: AutoFill) -> Self {
        self.auto_fill = Some(auto_fill);
        self
    }
}

#[cfg(feature = "sync")]
impl<'a> WshEventDataBuilder<'a, crate::client::sync::Client> {
    /// Submit the request and return the [`WshEventData`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use time::macros::date;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // Everything WSH has for the contract:
    /// let events = client.wsh_event_data_by_contract(76792991).fetch().expect("request failed");
    /// println!("{events:?}");
    ///
    /// // Narrowed to a date range:
    /// let events = client
    ///     .wsh_event_data_by_contract(76792991)
    ///     .starting(date!(2024 - 01 - 01))
    ///     .ending(date!(2024 - 03 - 31))
    ///     .limit(50)
    ///     .fetch()
    ///     .expect("request failed");
    /// println!("{events:?}");
    /// ```
    pub fn fetch(self) -> Result<WshEventData, Error> {
        crate::wsh::sync::wsh_event_data_by_contract(self.client, self.contract_id, self.start_date, self.end_date, self.limit, self.auto_fill)
    }
}

#[cfg(feature = "sync")]
impl<'a> WshEventFilterBuilder<'a, crate::client::sync::Client> {
    /// Submit the request and return a subscription of matching events.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // See https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#wsheventdata-object
    /// let subscription = client.wsh_event_data_by_filter("{}").limit(10).subscribe().expect("request failed");
    ///
    /// for event in subscription.iter_data() {
    ///     println!("{:?}", event.expect("decode error"));
    /// }
    /// ```
    pub fn subscribe(self) -> Result<crate::subscriptions::sync::Subscription<WshEventData>, Error> {
        crate::wsh::sync::wsh_event_data_by_filter(self.client, self.filter, self.limit, self.auto_fill)
    }
}

#[cfg(feature = "async")]
impl<'a> WshEventDataBuilder<'a, crate::client::r#async::Client> {
    /// Submit the request and return the [`WshEventData`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use time::macros::date;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     // Everything WSH has for the contract:
    ///     let events = client.wsh_event_data_by_contract(76792991).fetch().await.expect("request failed");
    ///     println!("{events:?}");
    ///
    ///     // Narrowed to a date range:
    ///     let events = client
    ///         .wsh_event_data_by_contract(76792991)
    ///         .starting(date!(2024 - 01 - 01))
    ///         .ending(date!(2024 - 03 - 31))
    ///         .limit(50)
    ///         .fetch()
    ///         .await
    ///         .expect("request failed");
    ///     println!("{events:?}");
    /// }
    /// ```
    pub async fn fetch(self) -> Result<WshEventData, Error> {
        crate::wsh::r#async::wsh_event_data_by_contract(self.client, self.contract_id, self.start_date, self.end_date, self.limit, self.auto_fill)
            .await
    }
}

#[cfg(feature = "async")]
impl<'a> WshEventFilterBuilder<'a, crate::client::r#async::Client> {
    /// Submit the request and return a subscription of matching events.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::subscriptions::r#async::SubscriptionItemStreamExt;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     // See https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#wsheventdata-object
    ///     let mut subscription = client.wsh_event_data_by_filter("{}").limit(10).subscribe().await.expect("request failed");
    ///
    ///     let mut events = subscription.filter_data();
    ///     while let Some(event) = events.next().await {
    ///         println!("{:?}", event.expect("decode error"));
    ///     }
    /// }
    /// ```
    pub async fn subscribe(self) -> Result<crate::subscriptions::r#async::Subscription<WshEventData>, Error> {
        crate::wsh::r#async::wsh_event_data_by_filter(self.client, self.filter, self.limit, self.auto_fill).await
    }
}
