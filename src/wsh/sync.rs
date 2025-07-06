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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::ResponseMessage;
    use crate::stubs::MessageBusStub;
    use std::sync::{Arc, RwLock};
    use time::macros::date;

    #[test]
    fn test_wsh_metadata_sync() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["104|9000|{\"validated\":true,\"data\":{\"metadata\":\"test\"}}|".to_owned()],
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSHE_CALENDAR);
        let result = wsh_metadata(&client);

        assert!(result.is_ok(), "failed to request wsh metadata: {}", result.err().unwrap());
        assert_eq!(
            result.unwrap(),
            WshMetadata {
                data_json: "{\"validated\":true,\"data\":{\"metadata\":\"test\"}}".to_owned()
            }
        );
    }

    #[test]
    fn test_wsh_event_data_by_contract_sync() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["105|9001|{\"validated\":true,\"data\":{\"events\":[]}}|".to_owned()],
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE);
        let result = wsh_event_data_by_contract(
            &client,
            12345,
            Some(date!(2024 - 01 - 01)),
            Some(date!(2024 - 12 - 31)),
            Some(100),
            Some(AutoFill {
                competitors: true,
                portfolio: false,
                watchlist: true,
            }),
        );

        assert!(result.is_ok(), "failed to request wsh event data: {}", result.err().unwrap());
        assert_eq!(
            result.unwrap(),
            WshEventData {
                data_json: "{\"validated\":true,\"data\":{\"events\":[]}}".to_owned()
            }
        );
    }

    #[test]
    fn test_wsh_event_data_by_filter_subscription_sync() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "105|9003|{\"event\":\"earnings\",\"date\":\"2024-01-15\"}|".to_owned(),
                "105|9003|{\"event\":\"dividend\",\"date\":\"2024-02-01\"}|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE);
        let filter = "earnings";
        let result = wsh_event_data_by_filter(&client, filter, Some(50), None);
        
        assert!(result.is_ok());
        let mut subscription = result.unwrap();

        // First event
        let first = subscription.next();
        assert!(first.is_some());
        let event = first.unwrap();
        assert_eq!(event.data_json, "{\"event\":\"earnings\",\"date\":\"2024-01-15\"}");

        // Second event
        let second = subscription.next();
        assert!(second.is_some());
        let event = second.unwrap();
        assert_eq!(event.data_json, "{\"event\":\"dividend\",\"date\":\"2024-02-01\"}");

        // No more events
        let third = subscription.next();
        assert!(third.is_none());
    }

    #[test]
    fn test_data_stream_cancel_message() {
        let request_id = 9000;
        
        // Test WshMetadata cancel
        let cancel_msg = WshMetadata::cancel_message(0, Some(request_id), &ResponseContext::default());
        assert!(cancel_msg.is_ok());
        assert_eq!(cancel_msg.unwrap().encode_simple(), "101|9000|");

        // Test WshEventData cancel
        let cancel_msg = WshEventData::cancel_message(0, Some(request_id), &ResponseContext::default());
        assert!(cancel_msg.is_ok());
        assert_eq!(cancel_msg.unwrap().encode_simple(), "103|9000|");
    }

    #[test]
    fn test_data_stream_decode() {
        // Test WshMetadata decode
        let mut message = ResponseMessage::from("104\09000\0{\"test\":\"metadata\"}\0");
        let result = WshMetadata::decode(&Client::stubbed(Arc::new(MessageBusStub::default()), 0), &mut message);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().data_json, "{\"test\":\"metadata\"}");

        // Test WshEventData decode - success case
        let mut message = ResponseMessage::from("105\09000\0{\"test\":\"event\"}\0");
        let result = WshEventData::decode(&Client::stubbed(Arc::new(MessageBusStub::default()), 0), &mut message);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().data_json, "{\"test\":\"event\"}");

        // Test WshEventData decode - error case
        let mut error_message = ResponseMessage::from("4\02\09000\0321\0Test error message\0");
        let result = WshEventData::decode(&Client::stubbed(Arc::new(MessageBusStub::default()), 0), &mut error_message);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Message(321, _)));
    }

    #[test]
    fn test_decode_unexpected_message_type() {
        // Test unexpected message type for WshMetadata
        let mut message = ResponseMessage::from("1\09000\0unexpected\0");
        let result = WshMetadata::decode(&Client::stubbed(Arc::new(MessageBusStub::default()), 0), &mut message);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::UnexpectedResponse(_)));

        // Test unexpected message type for WshEventData
        let mut message = ResponseMessage::from("1\09000\0unexpected\0");
        let result = WshEventData::decode(&Client::stubbed(Arc::new(MessageBusStub::default()), 0), &mut message);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::UnexpectedResponse(_)));
    }
}