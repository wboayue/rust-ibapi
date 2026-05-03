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
    pub async fn wsh_metadata(&self) -> Result<WshMetadata, Error> {
        check_version(self.server_version(), Features::WSHE_CALENDAR)?;

        request_helpers::one_shot_request_with_retry(
            self,
            encoders::encode_request_wsh_metadata,
            |message| decoders::decode_wsh_metadata(message.clone()),
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }

    /// Fetch WSH event data filtered by contract identifier.
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
            |message| decoders::decode_event_data_message(message.clone()),
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }

    /// Subscribe to WSH event data using a filter expression.
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
