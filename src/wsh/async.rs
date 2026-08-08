//! Asynchronous implementation of Wall Street Horizon functionality

use time::Date;

use crate::{
    common::request_helpers::{self, expect_proto},
    protocol::{check_version, Features},
    subscriptions::Subscription,
    Client, Error,
};

use super::builder::{WshEventDataBuilder, WshEventFilterBuilder};
use super::{common::decoders, encoders, AutoFill, WshEventData, WshMetadata};

impl Client {
    /// Fetch Wall Street Horizon metadata table with retry semantics.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let metadata = client.wsh_metadata().await.expect("request wsh metadata failed");
    ///     println!("{metadata:?}");
    /// }
    /// ```
    pub async fn wsh_metadata(&self) -> Result<WshMetadata, Error> {
        check_version(self.server_version(), Features::WSHE_CALENDAR)?;

        request_helpers::one_shot_by_request_id(
            self,
            encoders::encode_request_wsh_metadata,
            expect_proto(decoders::decode_wsh_metadata_proto),
        )
        .await
    }

    /// Build a request for Wall Street Horizon events on one contract.
    ///
    /// Terminal: [`WshEventDataBuilder::fetch`]. Optional narrowing via
    /// `.starting()` / `.ending()` / `.limit()` / `.auto_fill()`, each of which
    /// carries its own server-version requirement.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - Contract identifier for the event request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract_id = 76792991; // TSLA
    ///     let event_data = client
    ///         .wsh_event_data_by_contract(contract_id)
    ///         .fetch()
    ///         .await
    ///         .expect("request wsh event data failed");
    ///     println!("{event_data:?}");
    /// }
    /// ```
    pub fn wsh_event_data_by_contract(&self, contract_id: i32) -> WshEventDataBuilder<'_, Self> {
        WshEventDataBuilder::new(self, contract_id)
    }

    /// Build a request for Wall Street Horizon events matching a JSON filter.
    ///
    /// Terminal: [`WshEventFilterBuilder::subscribe`].
    ///
    /// # Arguments
    ///
    /// * `filter` - Json-formatted string containing all filter values.
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
    ///     let filter = "{}"; // see https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#wsheventdata-object
    ///     let mut subscription = client
    ///         .wsh_event_data_by_filter(filter)
    ///         .subscribe()
    ///         .await
    ///         .expect("request wsh event data failed");
    ///
    ///     let mut events = subscription.filter_data();
    ///     while let Some(event) = events.next().await {
    ///         println!("{:?}", event.expect("decode error"));
    ///     }
    /// }
    /// ```
    pub fn wsh_event_data_by_filter<'a>(&'a self, filter: &'a str) -> WshEventFilterBuilder<'a, Self> {
        WshEventFilterBuilder::new(self, filter)
    }
}

/// Request events for one contract. Reached through
/// [`WshEventDataBuilder::fetch`](super::builder::WshEventDataBuilder::fetch).
pub(crate) async fn wsh_event_data_by_contract(
    client: &Client,
    contract_id: i32,
    start_date: Option<Date>,
    end_date: Option<Date>,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<WshEventData, Error> {
    check_version(client.server_version(), Features::WSHE_CALENDAR)?;

    if auto_fill.is_some() {
        check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS)?;
    }

    if start_date.is_some() || end_date.is_some() || limit.is_some() {
        check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS_DATE)?;
    }

    request_helpers::one_shot_by_request_id(
        client,
        |request_id| encoders::encode_request_wsh_event_data(request_id, Some(contract_id), None, start_date, end_date, limit, auto_fill),
        expect_proto(decoders::decode_wsh_event_data_proto),
    )
    .await
}

/// Request events matching a JSON filter. Reached through
/// [`WshEventFilterBuilder::subscribe`](super::builder::WshEventFilterBuilder::subscribe).
pub(crate) async fn wsh_event_data_by_filter(
    client: &Client,
    filter: &str,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<Subscription<WshEventData>, Error> {
    if limit.is_some() {
        check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS_DATE)?;
    }

    request_helpers::request_with_id(client, Features::WSH_EVENT_DATA_FILTERS, |request_id| {
        encoders::encode_request_wsh_event_data(
            request_id,
            None,
            Some(filter),
            None, // start_date
            None, // end_date
            limit,
            auto_fill,
        )
    })
    .await
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
