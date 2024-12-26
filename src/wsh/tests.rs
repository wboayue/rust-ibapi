use std::sync::{Arc, RwLock};
use time::macros::date;

use crate::{messages::ResponseMessage, server_versions, stubs::MessageBusStub, Client, Error};

use super::*;

#[test]
fn test_wsh_metadata() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["104|9000|{\"validated\":true,\"data\":{\"metadata\":\"test\"}}|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::WSHE_CALENDAR);
    let result = wsh_metadata(&client);

    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages[0].encode_simple(), "100|9000|");

    assert!(result.is_ok(), "failed to request wsh metadata: {}", result.err().unwrap());
    assert_eq!(
        result.unwrap(),
        WshMetadata {
            data_json: "{\"validated\":true,\"data\":{\"metadata\":\"test\"}}".to_owned()
        }
    );
}

#[test]
fn test_wsh_event_data_by_contract() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["105|9000|{\"validated\":true,\"data\":{\"events\":[]}}|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::WSH_EVENT_DATA_FILTERS_DATE);
    let result = wsh_event_data_by_contract(
        &client,
        12345,
        Some(date!(2024 - 01 - 01)),
        Some(date!(2024 - 12 - 31)),
        Some(100),
        Some(AutoFill {
            competitors: true,
            portfolio: true,
            watchlist: true,
        }),
    );

    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages[0].encode_simple(), "102|9000|12345||1|1|1|20240101|20241231|100|");

    assert!(result.is_ok(), "failed to request wsh event data: {}", result.err().unwrap());
    assert_eq!(
        result.unwrap(),
        WshEventData {
            data_json: "{\"validated\":true,\"data\":{\"events\":[]}}".to_owned()
        }
    );
}

#[test]
fn test_wsh_event_data_by_contract_no_filters() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["105|9000|{\"validated\":true,\"data\":{\"events\":[]}}|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::WSHE_CALENDAR);
    let result = wsh_event_data_by_contract(&client, 12345, None, None, None, None);

    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages[0].encode_simple(), "102|9000|12345|");

    assert!(result.is_ok(), "failed to request wsh event data: {}", result.err().unwrap());
    assert_eq!(
        result.unwrap(),
        WshEventData {
            data_json: "{\"validated\":true,\"data\":{\"events\":[]}}".to_owned()
        }
    );
}

#[test]
fn test_wsh_event_data_by_filter() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["105|9000|{\"validated\":true,\"data\":{\"events\":[]}}|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::WSH_EVENT_DATA_FILTERS_DATE);
    let filter = "filter=value";
    let result = wsh_event_data_by_filter(
        &client,
        filter,
        Some(100),
        Some(AutoFill {
            competitors: true,
            portfolio: false,
            watchlist: true,
        }),
    );

    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages[0].encode_simple(), "102|9000||filter=value|1|0|1|||100|");

    assert!(result.is_ok(), "failed to request wsh event data by filter: {}", result.err().unwrap());
}

#[test]
fn test_wsh_event_data_by_filter_no_autofill() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["105|9000|{\"validated\":true,\"data\":{\"events\":[]}}|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::WSH_EVENT_DATA_FILTERS);
    let filter = "filter=value";
    let result = wsh_event_data_by_filter(&client, filter, None, None);

    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages[0].encode_simple(), "102|9000||filter=value|0|0|0|");

    assert!(result.is_ok(), "failed to request wsh event data by filter: {}", result.err().unwrap());
}

#[test]
fn test_invalid_server_version_wsh_metadata() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::SCALE_ORDERS);
    let result = wsh_metadata(&client);

    assert!(matches!(result, Err(Error::ServerVersion(_, _, _))));
}

#[test]
fn test_invalid_server_version_wsh_event_data_filters() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::WSHE_CALENDAR);
    let result = wsh_event_data_by_filter(&client, "filter", None, None);

    assert!(matches!(result, Err(Error::ServerVersion(_, _, _))));
}

#[test]
fn test_invalid_server_version_wsh_event_data_date_filters() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::WSH_EVENT_DATA_FILTERS);
    let result = wsh_event_data_by_contract(&client, 12345, Some(date!(2024 - 01 - 01)), Some(date!(2024 - 12 - 31)), Some(100), None);

    assert!(matches!(result, Err(Error::ServerVersion(_, _, _))));
}

#[test]
fn test_autofill_is_specified() {
    assert!(!AutoFill::default().is_specified());

    assert!(AutoFill {
        competitors: true,
        portfolio: false,
        watchlist: false,
    }
    .is_specified());

    assert!(AutoFill {
        competitors: false,
        portfolio: true,
        watchlist: false,
    }
    .is_specified());

    assert!(AutoFill {
        competitors: false,
        portfolio: false,
        watchlist: true,
    }
    .is_specified());
}

#[test]
fn test_decode_wsh_metadata() {
    use super::decoders::decode_wsh_metadata;

    let message = ResponseMessage::from("104\09000\0{\"test\":\"data\"}\0");
    let result = decode_wsh_metadata(message);

    assert!(result.is_ok(), "failed to decode wsh metadata: {}", result.err().unwrap());
    assert_eq!(result.unwrap().data_json, "{\"test\":\"data\"}");
}

#[test]
fn test_decode_wsh_event_data() {
    use super::decoders::decode_wsh_event_data;

    let message = ResponseMessage::from("105\09000\0{\"test\":\"data\"}\0");
    let result = decode_wsh_event_data(message);

    assert!(result.is_ok(), "failed to decode wsh event data: {}", result.err().unwrap());
    assert_eq!(result.unwrap().data_json, "{\"test\":\"data\"}");
}

#[test]
fn test_encode_request_wsh_metadata() {
    use super::encoders::encode_request_wsh_metadata;

    let result = encode_request_wsh_metadata(9000);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().encode_simple(), "100|9000|");
}

#[test]
fn test_encode_cancel_wsh_metadata() {
    use super::encoders::encode_cancel_wsh_metadata;

    let result = encode_cancel_wsh_metadata(9000);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().encode_simple(), "101|9000|");
}

#[test]
fn test_encode_request_wsh_event_data() {
    use super::encoders::encode_request_wsh_event_data;

    // Test with minimal params
    let result = encode_request_wsh_event_data(server_versions::WSHE_CALENDAR, 9000, Some(12345), None, None, None, None, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().encode_simple(), "102|9000|12345|");

    // Test with all params
    let result = encode_request_wsh_event_data(
        server_versions::WSH_EVENT_DATA_FILTERS_DATE,
        9000,
        Some(12345),
        Some("filter"),
        Some(date!(2024 - 01 - 01)),
        Some(date!(2024 - 12 - 31)),
        Some(100),
        Some(AutoFill {
            competitors: true,
            portfolio: false,
            watchlist: true,
        }),
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().encode_simple(), "102|9000|12345|filter|1|0|1|20240101|20241231|100|");
}

#[test]
fn test_encode_cancel_wsh_event_data() {
    use super::encoders::encode_cancel_wsh_event_data;

    let result = encode_cancel_wsh_event_data(9000);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().encode_simple(), "103|9000|");
}
