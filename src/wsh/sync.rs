//! Synchronous implementation of Wall Street Horizon functionality

use time::Date;

use crate::{
    client::Subscription,
    common::request_helpers,
    messages::OutgoingMessages,
    protocol::{check_version, Features},
    Client, Error,
};

use super::{common::decoders, encoders, AutoFill, WshEventData, WshMetadata};

pub fn wsh_metadata(client: &Client) -> Result<WshMetadata, Error> {
    check_version(client.server_version, Features::WSHE_CALENDAR)?;

    request_helpers::one_shot_with_retry(
        client,
        OutgoingMessages::RequestWshMetaData,
        || {
            let request_id = client.next_request_id();
            encoders::encode_request_wsh_metadata(request_id)
        },
        |message| decoders::decode_wsh_metadata(message.clone()),
        || Err(Error::UnexpectedEndOfStream),
    )
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

    request_helpers::one_shot_with_retry(
        client,
        OutgoingMessages::RequestWshEventData,
        || {
            let request_id = client.next_request_id();
            encoders::encode_request_wsh_event_data(
                client.server_version,
                request_id,
                Some(contract_id),
                None,
                start_date,
                end_date,
                limit,
                auto_fill,
            )
        },
        |message| decoders::decode_event_data_message(message.clone()),
        || Err(Error::UnexpectedEndOfStream),
    )
}

pub fn wsh_event_data_by_filter<'a>(
    client: &'a Client,
    filter: &str,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<Subscription<'a, WshEventData>, Error> {
    if limit.is_some() {
        check_version(client.server_version, Features::WSH_EVENT_DATA_FILTERS_DATE)?;
    }

    request_helpers::request_with_id(client, Features::WSH_EVENT_DATA_FILTERS, |request_id| {
        encoders::encode_request_wsh_event_data(
            client.server_version,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::ResponseMessage;
    use crate::stubs::MessageBusStub;
    use crate::subscriptions::StreamDecoder;
    use crate::wsh::common::test_data::{self, json_responses};
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_wsh_metadata_table() {
        use crate::wsh::common::test_tables::{wsh_metadata_test_cases, ApiExpectedResult};

        for test_case in wsh_metadata_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages,
            });

            let client = Client::stubbed(message_bus, test_case.server_version);
            let result = wsh_metadata(&client);

            match test_case.expected_result {
                ApiExpectedResult::Success { json } => {
                    assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                    assert_eq!(result.unwrap().data_json, json, "Test '{}' json mismatch", test_case.name);
                }
                ApiExpectedResult::ServerVersionError => {
                    assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                    assert!(
                        matches!(result.unwrap_err(), Error::ServerVersion(_, _, _)),
                        "Test '{}' wrong error type",
                        test_case.name
                    );
                }
            }
        }
    }

    #[test]
    fn test_wsh_event_data_by_contract_table() {
        use crate::wsh::common::test_tables::{event_data_by_contract_test_cases, ApiExpectedResult};

        for test_case in event_data_by_contract_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages,
            });

            let client = Client::stubbed(message_bus, test_case.server_version);
            let result = wsh_event_data_by_contract(
                &client,
                test_case.contract_id,
                test_case.start_date,
                test_case.end_date,
                test_case.limit,
                test_case.auto_fill,
            );

            match test_case.expected_result {
                ApiExpectedResult::Success { json } => {
                    assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                    assert_eq!(result.unwrap().data_json, json, "Test '{}' json mismatch", test_case.name);
                }
                ApiExpectedResult::ServerVersionError => {
                    assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                    assert!(
                        matches!(result.unwrap_err(), Error::ServerVersion(_, _, _)),
                        "Test '{}' wrong error type",
                        test_case.name
                    );
                }
            }
        }
    }

    #[test]
    fn test_wsh_event_data_by_filter_subscription_sync() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                test_data::build_response("105", test_data::REQUEST_ID_FILTER, json_responses::EVENT_DATA_EARNINGS),
                test_data::build_response("105", test_data::REQUEST_ID_FILTER, json_responses::EVENT_DATA_DIVIDEND),
            ],
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE);
        let result = wsh_event_data_by_filter(&client, test_data::TEST_FILTER, Some(50), None);

        assert!(result.is_ok());
        let subscription = result.unwrap();

        // First event
        let first = subscription.next();
        assert!(first.is_some());
        let event = first.unwrap();
        assert_eq!(event.data_json, json_responses::EVENT_DATA_EARNINGS);

        // Second event
        let second = subscription.next();
        assert!(second.is_some());
        let event = second.unwrap();
        assert_eq!(event.data_json, json_responses::EVENT_DATA_DIVIDEND);

        // No more events
        let third = subscription.next();
        assert!(third.is_none());
    }

    #[test]
    fn test_data_stream_cancel_message() {
        let request_id = test_data::REQUEST_ID_METADATA;

        // Test WshMetadata cancel
        let cancel_msg = WshMetadata::cancel_message(0, Some(request_id), None);
        assert!(cancel_msg.is_ok());
        assert_eq!(cancel_msg.unwrap().encode_simple(), "101|9000|");

        // Test WshEventData cancel
        let cancel_msg = WshEventData::cancel_message(0, Some(request_id), None);
        assert!(cancel_msg.is_ok());
        assert_eq!(cancel_msg.unwrap().encode_simple(), "103|9000|");
    }

    #[test]
    fn test_wsh_metadata_decode_table() {
        use crate::wsh::common::test_tables::WSH_METADATA_DECODE_TESTS;

        for test_case in WSH_METADATA_DECODE_TESTS {
            let mut message = ResponseMessage::from(test_case.message);
            let result = WshMetadata::decode(0, &mut message);

            if test_case.should_error {
                assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                match test_case.error_type {
                    Some("UnexpectedResponse") => assert!(matches!(result.unwrap_err(), Error::UnexpectedResponse(_))),
                    _ => panic!("Unknown error type for test '{}'", test_case.name),
                }
            } else {
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                assert_eq!(
                    result.unwrap().data_json,
                    test_case.expected_json,
                    "Test '{}' json mismatch",
                    test_case.name
                );
            }
        }
    }

    #[test]
    fn test_wsh_event_data_decode_table() {
        use crate::wsh::common::test_tables::WSH_EVENT_DATA_DECODE_TESTS;

        for test_case in WSH_EVENT_DATA_DECODE_TESTS {
            let mut message = ResponseMessage::from(test_case.message);
            let result = WshEventData::decode(0, &mut message);

            if test_case.should_error {
                assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                match test_case.error_type {
                    Some("Message") => assert!(matches!(result.unwrap_err(), Error::Message(_, _))),
                    Some("UnexpectedResponse") => assert!(matches!(result.unwrap_err(), Error::UnexpectedResponse(_))),
                    _ => panic!("Unknown error type for test '{}'", test_case.name),
                }
            } else {
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                assert_eq!(
                    result.unwrap().data_json,
                    test_case.expected_json,
                    "Test '{}' json mismatch",
                    test_case.name
                );
            }
        }
    }
}
