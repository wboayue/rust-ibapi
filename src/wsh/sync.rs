//! Synchronous implementation of Wall Street Horizon functionality

use time::Date;

use crate::client::sync::Client;
use crate::subscriptions::sync::Subscription;
use crate::{
    common::request_helpers,
    protocol::{check_version, Features},
    Error,
};

use super::{common::decoders, encoders, AutoFill, WshEventData, WshMetadata};

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
        check_version(self.server_version, Features::WSHE_CALENDAR)?;

        request_helpers::blocking::one_shot_request_with_retry(
            self,
            encoders::encode_request_wsh_metadata,
            |message| decoders::decode_wsh_metadata(message.clone()),
            || Err(Error::UnexpectedEndOfStream),
        )
    }

    /// Requests event data for a specified contract from the Wall Street Horizons (WSH) calendar.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - Contract identifier for the event request.
    /// * `start_date`  - Start date of the event request.
    /// * `end_date`    - End date of the event request.
    /// * `limit`       - Maximum number of events to return. Maximum of 100.
    /// * `auto_fill`   - Fields to automatically fill in. See [AutoFill] for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract_id = 76792991; // TSLA
    /// let event_data = client.wsh_event_data_by_contract(contract_id, None, None, None, None).expect("request wsh event data failed");
    /// println!("{event_data:?}");
    /// ```
    pub fn wsh_event_data_by_contract(
        &self,
        contract_id: i32,
        start_date: Option<Date>,
        end_date: Option<Date>,
        limit: Option<i32>,
        auto_fill: Option<AutoFill>,
    ) -> Result<WshEventData, Error> {
        check_version(self.server_version, Features::WSHE_CALENDAR)?;

        if auto_fill.is_some() {
            check_version(self.server_version, Features::WSH_EVENT_DATA_FILTERS)?;
        }

        if start_date.is_some() || end_date.is_some() || limit.is_some() {
            check_version(self.server_version, Features::WSH_EVENT_DATA_FILTERS_DATE)?;
        }

        request_helpers::blocking::one_shot_request_with_retry(
            self,
            |request_id| encoders::encode_request_wsh_event_data(request_id, Some(contract_id), None, start_date, end_date, limit, auto_fill),
            |message| decoders::decode_event_data_message(message.clone()),
            || Err(Error::UnexpectedEndOfStream),
        )
    }

    /// Requests event data from the Wall Street Horizons (WSH) calendar using a JSON filter.
    ///
    /// # Arguments
    ///
    /// * `filter`    - Json-formatted string containing all filter values.
    /// * `limit`     - Maximum number of events to return. Maximum of 100.
    /// * `auto_fill` - Fields to automatically fill in. See [AutoFill] for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let filter = ""; // see https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#wsheventdata-object
    /// let event_data = client.wsh_event_data_by_filter(filter, None, None).expect("request wsh event data failed");
    /// for result in event_data {
    ///     println!("{result:?}");
    /// }
    /// ```
    pub fn wsh_event_data_by_filter(
        &self,
        filter: &str,
        limit: Option<i32>,
        auto_fill: Option<AutoFill>,
    ) -> Result<Subscription<WshEventData>, Error> {
        if limit.is_some() {
            check_version(self.server_version, Features::WSH_EVENT_DATA_FILTERS_DATE)?;
        }

        request_helpers::blocking::request_with_id(self, Features::WSH_EVENT_DATA_FILTERS, |request_id| {
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
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
