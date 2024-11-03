// Wall Street Horizon: Earnings Calendar & Event Data

use std::str;

use serde::{Deserialize, Serialize};
use time::Date;

use crate::{
    client::{DataStream, ResponseContext, Subscription},
    messages::IncomingMessages,
    server_versions, Client, Error,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WshMetadata {
    data_json: String,
}

impl DataStream<WshMetadata> for WshMetadata {
    fn decode(_client: &Client, message: &mut crate::messages::ResponseMessage) -> Result<WshMetadata, Error> {
        match message.message_type() {
            IncomingMessages::WshMetaData => Ok(decoders::decode_wsh_metadata(message.clone())?),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<crate::messages::RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel wsh metadata message.");
        encoders::encode_cancel_wsh_metadata(request_id)
    }
}

pub fn wsh_metadata(client: &Client) -> Result<Subscription<WshMetadata>, Error> {
    client.check_server_version(server_versions::WSHE_CALENDAR, "It does not support WSHE Calendar API.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_wsh_metadata(request_id)?;
    let subscription = client.send_request(request_id, request)?;
    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WshEventData {
    data_json: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoFill {
    /// Automatically fill in competitor values of existing positions.
    pub competitors: bool,
    /// Automatically fill in portfolio values.
    pub portfolio: bool,
    // Automatically fill in watchlist values.
    pub watchlist: bool,
}

impl AutoFill {
    pub fn is_specified(&self) -> bool {
        self.competitors || self.portfolio || self.watchlist
    }
}

impl DataStream<WshEventData> for WshEventData {
    fn decode(_client: &Client, message: &mut crate::messages::ResponseMessage) -> Result<WshEventData, Error> {
        match message.message_type() {
            IncomingMessages::WshEventData => Ok(decoders::decode_wsh_event_data(message.clone())?),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<crate::messages::RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel wsh metadata message.");
        encoders::encode_cancel_wsh_metadata(request_id)
    }
}

pub fn wsh_event_data_by_contract(
    client: &Client,
    contract_id: i32,
    start_date: Option<Date>,
    end_date: Option<Date>,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<Subscription<WshEventData>, Error> {
    client.check_server_version(server_versions::WSHE_CALENDAR, "It does not support WSHE Calendar API.")?;

    if client.server_version < server_versions::WSH_EVENT_DATA_FILTERS {
        if !auto_fill.is_some() {
            let message = "It does not support WSH event data filters.".to_string();
            return Err(Error::ServerVersion(
                server_versions::WSH_EVENT_DATA_FILTERS,
                client.server_version,
                message,
            ));
        }
    }

    if client.server_version < server_versions::WSH_EVENT_DATA_FILTERS_DATE {
        if start_date.is_some() || end_date.is_some() || limit.is_some() {
            let message = "It does not support WSH event data date filters.".to_string();
            return Err(Error::ServerVersion(
                server_versions::WSH_EVENT_DATA_FILTERS_DATE,
                client.server_version,
                message,
            ));
        }
    }

    let request_id = client.next_request_id();
    let request = encoders::encode_request_wsh_metadata(request_id)?;
    let subscription = client.send_request(request_id, request)?;

    Err(Error::NotImplemented)
}

pub fn wsh_event_data_by_filter<'a>(
    client: &'a Client,
    filter: &str,
    limit: i32,
    auto_fill: AutoFill,
) -> Result<Subscription<'a, WshEventData>, Error> {
    Err(Error::NotImplemented)
}

pub fn wsh_event_data(client: &Client) -> Result<Subscription<WshEventData>, Error> {
    client.check_server_version(server_versions::WSHE_CALENDAR, "It does not support WSHE Calendar API.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_wsh_metadata(request_id)?;
    let subscription = client.send_request(request_id, request)?;
    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

mod encoders {
    use super::Error;

    use crate::messages::{OutgoingMessages, RequestMessage};

    pub(super) fn encode_request_wsh_metadata(request_id: i32) -> Result<RequestMessage, Error> {
        let mut message = RequestMessage::new();

        message.push_field(&OutgoingMessages::RequestWshMetaData);
        message.push_field(&request_id);

        Ok(message)
    }

    pub(super) fn encode_cancel_wsh_metadata(request_id: i32) -> Result<RequestMessage, Error> {
        let mut message = RequestMessage::new();

        message.push_field(&OutgoingMessages::CancelWshMetaData);
        message.push_field(&request_id);

        Ok(message)
    }
}

mod decoders {
    use crate::messages::ResponseMessage;

    use super::{Error, WshEventData, WshMetadata};

    pub(super) fn decode_wsh_metadata(mut message: ResponseMessage) -> Result<WshMetadata, Error> {
        message.skip(); // skip message type
        message.skip(); // skip request id

        Ok(WshMetadata {
            data_json: message.next_string()?,
        })
    }

    pub(super) fn decode_wsh_event_data(mut message: ResponseMessage) -> Result<WshEventData, Error> {
        message.skip(); // skip message type
        message.skip(); // skip request id

        Ok(WshEventData {
            data_json: message.next_string()?,
        })
    }
}
