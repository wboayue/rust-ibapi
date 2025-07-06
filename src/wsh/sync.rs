//! Synchronous implementation of Wall Street Horizon functionality

use time::Date;

use crate::{
    client::{DataStream, ResponseContext, Subscription},
    messages::IncomingMessages,
    protocol::{check_version, Features},
    Client, Error,
};

use super::{decoders, encoders, AutoFill, WshEventData, WshMetadata};

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

impl DataStream<WshEventData> for WshEventData {
    fn decode(_client: &Client, message: &mut crate::messages::ResponseMessage) -> Result<WshEventData, Error> {
        decode_event_data_message(message.clone())
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<crate::messages::RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel wsh metadata message.");
        encoders::encode_cancel_wsh_event_data(request_id)
    }
}

fn decode_event_data_message(message: crate::messages::ResponseMessage) -> Result<WshEventData, Error> {
    match message.message_type() {
        IncomingMessages::WshEventData => decoders::decode_wsh_event_data(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::UnexpectedResponse(message)),
    }
}

pub fn wsh_metadata(client: &Client) -> Result<WshMetadata, Error> {
    check_version(client.server_version, Features::WSHE_CALENDAR)?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_wsh_metadata(request_id)?;
    let subscription = client.send_request(request_id, request)?;

    match subscription.next() {
        Some(Ok(message)) => Ok(decoders::decode_wsh_metadata(message)?),
        Some(Err(Error::ConnectionReset)) => wsh_metadata(client),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

pub fn wsh_event_data_by_contract(
    client: &Client,
    contract_id: i32,
    start_date: Option<Date>,
    end_date: Option<Date>,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<WshEventData, Error> {
    check_version(client.server_version, Features::WSHE_CALENDAR)?;

    if auto_fill.is_some() {
        check_version(client.server_version, Features::WSH_EVENT_DATA_FILTERS)?;
    }

    if start_date.is_some() || end_date.is_some() || limit.is_some() {
        check_version(client.server_version, Features::WSH_EVENT_DATA_FILTERS_DATE)?;
    }

    let request_id = client.next_request_id();
    let request = encoders::encode_request_wsh_event_data(
        client.server_version,
        request_id,
        Some(contract_id),
        None,
        start_date,
        end_date,
        limit,
        auto_fill,
    )?;
    let subscription = client.send_request(request_id, request)?;

    match subscription.next() {
        Some(Ok(message)) => decode_event_data_message(message),
        Some(Err(Error::ConnectionReset)) => wsh_event_data_by_contract(client, contract_id, start_date, end_date, limit, auto_fill),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

pub fn wsh_event_data_by_filter<'a>(
    client: &'a Client,
    filter: &str,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<Subscription<'a, WshEventData>, Error> {
    check_version(client.server_version, Features::WSH_EVENT_DATA_FILTERS)?;

    if limit.is_some() {
        check_version(client.server_version, Features::WSH_EVENT_DATA_FILTERS_DATE)?;
    }

    let request_id = client.next_request_id();
    let request = encoders::encode_request_wsh_event_data(
        client.server_version,
        request_id,
        None,
        Some(filter),
        None,  // start_date  
        None,  // end_date
        limit,
        auto_fill,
    )?;
    let subscription = client.send_request(request_id, request)?;
    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}