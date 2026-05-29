//! Asynchronous implementation of Wall Street Horizon functionality

use time::Date;

use crate::{
    common::request_helpers,
    protocol::{check_version, Features},
    subscriptions::Subscription,
    Client, Error,
};

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

        request_helpers::one_shot_request_with_retry(self, encoders::encode_request_wsh_metadata, decoders::decode_metadata_message, || {
            Err(Error::UnexpectedEndOfStream)
        })
        .await
    }

    /// Fetch WSH event data filtered by contract identifier.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - Contract identifier for the event request.
    /// * `start_date`  - Start date of the event request.
    /// * `end_date`    - End date of the event request.
    /// * `limit`       - Maximum number of events to return. Maximum of 100.
    /// * `auto_fill`   - Fields to automatically fill in. See [`AutoFill`] for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract_id = 76792991; // TSLA
    ///     let event_data = client
    ///         .wsh_event_data_by_contract(contract_id, None, None, None, None)
    ///         .await
    ///         .expect("request wsh event data failed");
    ///     println!("{event_data:?}");
    /// }
    /// ```
    pub async fn wsh_event_data_by_contract(
        &self,
        contract_id: i32,
        start_date: Option<Date>,
        end_date: Option<Date>,
        limit: Option<i32>,
        auto_fill: Option<AutoFill>,
    ) -> Result<WshEventData, Error> {
        check_version(self.server_version(), Features::WSHE_CALENDAR)?;

        if auto_fill.is_some() {
            check_version(self.server_version(), Features::WSH_EVENT_DATA_FILTERS)?;
        }

        if start_date.is_some() || end_date.is_some() || limit.is_some() {
            check_version(self.server_version(), Features::WSH_EVENT_DATA_FILTERS_DATE)?;
        }

        request_helpers::one_shot_request_with_retry(
            self,
            |request_id| encoders::encode_request_wsh_event_data(request_id, Some(contract_id), None, start_date, end_date, limit, auto_fill),
            decoders::decode_event_data_message,
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }

    /// Subscribe to WSH event data using a filter expression.
    ///
    /// # Arguments
    ///
    /// * `filter`    - JSON-formatted string containing all filter values.
    /// * `limit`     - Maximum number of events to return. Maximum of 100.
    /// * `auto_fill` - Fields to automatically fill in. See [`AutoFill`] for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let filter = ""; // see https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#wsheventdata-object
    ///     let subscription = client
    ///         .wsh_event_data_by_filter(filter, None, None)
    ///         .await
    ///         .expect("request wsh event data failed");
    ///     let mut data = subscription.filter_data();
    ///     while let Some(result) = data.next().await {
    ///         println!("{result:?}");
    ///     }
    /// }
    /// ```
    pub async fn wsh_event_data_by_filter(
        &self,
        filter: &str,
        limit: Option<i32>,
        auto_fill: Option<AutoFill>,
    ) -> Result<Subscription<WshEventData>, Error> {
        if limit.is_some() {
            check_version(self.server_version(), Features::WSH_EVENT_DATA_FILTERS_DATE)?;
        }

        request_helpers::request_with_id(self, Features::WSH_EVENT_DATA_FILTERS, |request_id| {
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
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
