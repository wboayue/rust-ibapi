//! Synchronous implementation of Wall Street Horizon functionality

use time::Date;

use crate::client::blocking::ClientRequestBuilders;
use crate::client::sync::Client;
use crate::subscriptions::sync::Subscription;
use crate::{common::request_helpers, Error};

use super::builder::{WshEventDataBuilder, WshEventFilterBuilder};
use super::common::stream_decoders::{WshEventDataRequest, WshMetadataRequest};
use super::{encoders, AutoFill, WshEventData, WshMetadata};

impl Client {
    /// Requests metadata from the WSH calendar.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let metadata = client.wsh_metadata().expect("request wsh metadata failed");
    /// println!("{metadata:?}");
    /// ```
    pub fn wsh_metadata(&self) -> Result<WshMetadata, Error> {
        request_helpers::blocking::one_shot_cancelling::<WshMetadataRequest>(self, encoders::encode_request_wsh_metadata)
    }

    /// Build a request for Wall Street Horizon events on one contract.
    ///
    /// Terminal: [`WshEventDataBuilder::fetch`]. Optional narrowing via
    /// `.starting()` / `.ending()` / `.limit()` / `.auto_fill()`.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - Contract identifier for the event request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract_id = 76792991; // TSLA
    /// let event_data = client.wsh_event_data_by_contract(contract_id).fetch().expect("request wsh event data failed");
    /// println!("{event_data:?}");
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
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let filter = "{}"; // see https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#wsheventdata-object
    /// let subscription = client.wsh_event_data_by_filter(filter).subscribe().expect("request wsh event data failed");
    /// for event in subscription.iter_data() {
    ///     println!("{:?}", event.expect("decode error"));
    /// }
    /// ```
    pub fn wsh_event_data_by_filter<'a>(&'a self, filter: &'a str) -> WshEventFilterBuilder<'a, Self> {
        WshEventFilterBuilder::new(self, filter)
    }
}

/// Request events for one contract. Reached through
/// [`WshEventDataBuilder::fetch`](super::builder::WshEventDataBuilder::fetch).
pub(crate) fn wsh_event_data_by_contract(
    client: &Client,
    contract_id: i32,
    start_date: Option<Date>,
    end_date: Option<Date>,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<WshEventData, Error> {
    request_helpers::blocking::one_shot_cancelling::<WshEventDataRequest>(client, |request_id| {
        encoders::encode_request_wsh_event_data(request_id, Some(contract_id), None, start_date, end_date, limit, auto_fill)
    })
}

/// Request events matching a JSON filter. Reached through
/// [`WshEventFilterBuilder::subscribe`](super::builder::WshEventFilterBuilder::subscribe).
pub(crate) fn wsh_event_data_by_filter(
    client: &Client,
    filter: &str,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<Subscription<WshEventData>, Error> {
    let builder = client.request();
    let request = encoders::encode_request_wsh_event_data(
        builder.request_id(),
        None,
        Some(filter),
        None, // start_date
        None, // end_date
        limit,
        auto_fill,
    )?;
    builder.send(request)
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
