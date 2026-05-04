use super::*;
use crate::common::test_utils::helpers::{assert_request, TEST_REQ_ID_FIRST};
use crate::stubs::MessageBusStub;
use crate::testdata::builders::wsh::{cancel_wsh_event_data_request, cancel_wsh_metadata_request, wsh_event_data_request, wsh_metadata_request};
use crate::wsh::common::test_data;
use std::sync::{Arc, RwLock};

#[tokio::test]
async fn test_wsh_metadata_table() {
    use crate::wsh::common::test_tables::{wsh_metadata_test_cases, ApiExpectedResult};

    for test_case in wsh_metadata_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: test_case.response_messages,
        });

        let client = Client::stubbed(message_bus, test_case.server_version);
        let result = client.wsh_metadata().await;

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

#[tokio::test]
async fn test_wsh_metadata_request_body() {
    use crate::wsh::common::test_data::json_responses;

    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![test_data::build_response("104", TEST_REQ_ID_FIRST, json_responses::METADATA_SIMPLE)],
    });

    let client = Client::stubbed(message_bus.clone(), crate::server_versions::WSHE_CALENDAR);
    client.wsh_metadata().await.expect("metadata request failed");

    assert_request(&message_bus, 0, &wsh_metadata_request().request_id(TEST_REQ_ID_FIRST));
}

#[tokio::test]
async fn test_wsh_event_data_by_contract_table() {
    use crate::wsh::common::test_tables::{event_data_by_contract_test_cases, ApiExpectedResult};

    for test_case in event_data_by_contract_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: test_case.response_messages,
        });

        let client = Client::stubbed(message_bus.clone(), test_case.server_version);
        let result = client
            .wsh_event_data_by_contract(
                test_case.contract_id,
                test_case.start_date,
                test_case.end_date,
                test_case.limit,
                test_case.auto_fill,
            )
            .await;

        match test_case.expected_result {
            ApiExpectedResult::Success { json } => {
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                assert_eq!(result.unwrap().data_json, json, "Test '{}' json mismatch", test_case.name);
                assert_request(
                    &message_bus,
                    0,
                    &wsh_event_data_request()
                        .request_id(TEST_REQ_ID_FIRST)
                        .contract_id(Some(test_case.contract_id))
                        .start_date(test_case.start_date)
                        .end_date(test_case.end_date)
                        .limit(test_case.limit)
                        .auto_fill(test_case.auto_fill),
                );
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

#[tokio::test]
async fn test_wsh_event_data_by_filter_subscription_table() {
    use crate::wsh::common::test_tables::subscription_test_cases;

    for test_case in subscription_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: test_case.response_messages,
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE);
        let mut subscription = client
            .wsh_event_data_by_filter(test_case.filter, test_case.limit, test_case.auto_fill)
            .await
            .unwrap_or_else(|_| panic!("Test '{}' failed to create subscription", test_case.name));

        let mut received_events = vec![];
        while let Some(result) = subscription.next_data().await {
            match result {
                Ok(event) => received_events.push(event.data_json),
                Err(e) => panic!("Test '{}' unexpected error: {e:?}", test_case.name),
            }
        }

        assert_eq!(
            received_events.len(),
            test_case.expected_events.len(),
            "Test '{}' event count mismatch",
            test_case.name
        );

        for (i, (received, expected)) in received_events.iter().zip(test_case.expected_events.iter()).enumerate() {
            assert_eq!(received, expected, "Test '{}' event {} mismatch", test_case.name, i);
        }
    }
}

#[tokio::test]
async fn test_wsh_metadata_decode_table() {
    use crate::messages::ResponseMessage;
    use crate::subscriptions::{DecoderContext, StreamDecoder};
    use crate::wsh::common::test_tables::WSH_METADATA_DECODE_TESTS;

    for test_case in WSH_METADATA_DECODE_TESTS {
        let mut message = ResponseMessage::from(test_case.message);
        let result = WshMetadata::decode(&DecoderContext::default(), &mut message);

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

#[tokio::test]
async fn test_wsh_event_data_decode_table() {
    use crate::messages::ResponseMessage;
    use crate::subscriptions::{DecoderContext, StreamDecoder};
    use crate::wsh::common::test_tables::WSH_EVENT_DATA_DECODE_TESTS;

    for test_case in WSH_EVENT_DATA_DECODE_TESTS {
        let mut message = ResponseMessage::from(test_case.message);
        let result = WshEventData::decode(&DecoderContext::default(), &mut message);

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

#[tokio::test]
async fn test_wsh_event_data_by_filter_integration_table() {
    use crate::wsh::common::test_tables::{event_data_by_filter_integration_test_cases, IntegrationExpectedResult};

    for test_case in event_data_by_filter_integration_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: test_case.response_messages,
        });

        let client = Client::stubbed(message_bus.clone(), test_case.server_version);

        let result = match test_case.name {
            "successful filter request with autofill" => {
                let filter = "filter=value";
                let auto_fill = AutoFill {
                    competitors: true,
                    portfolio: false,
                    watchlist: true,
                };
                let result = client.wsh_event_data_by_filter(filter, Some(100), Some(auto_fill)).await;
                if result.is_ok() {
                    assert_request(
                        &message_bus,
                        0,
                        &wsh_event_data_request()
                            .request_id(TEST_REQ_ID_FIRST)
                            .filter(Some(filter))
                            .limit(Some(100))
                            .auto_fill(Some(auto_fill)),
                    );
                }
                result
            }
            "successful filter request without autofill" => {
                let filter = "filter=value";
                let result = client.wsh_event_data_by_filter(filter, None, None).await;
                if result.is_ok() {
                    assert_request(
                        &message_bus,
                        0,
                        &wsh_event_data_request().request_id(TEST_REQ_ID_FIRST).filter(Some(filter)),
                    );
                }
                result
            }
            _ => client.wsh_event_data_by_filter("filter", None, None).await,
        };

        match test_case.expected_result {
            IntegrationExpectedResult::Success => {
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
            }
            IntegrationExpectedResult::ServerVersionError => {
                assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                if let Err(error) = result {
                    assert!(
                        matches!(error, Error::ServerVersion(_, _, _)),
                        "Test '{}' wrong error type",
                        test_case.name
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_server_version_validation_table() {
    use crate::wsh::common::test_tables::server_version_test_cases;

    for test_case in server_version_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, test_case.server_version);

        let result = if let Some(contract_id) = test_case.contract_id {
            client
                .wsh_event_data_by_contract(
                    contract_id,
                    test_case.start_date,
                    test_case.end_date,
                    test_case.limit,
                    test_case.auto_fill,
                )
                .await
        } else {
            client
                .wsh_event_data_by_filter("filter", test_case.limit, test_case.auto_fill)
                .await
                .map(|_| WshEventData { data_json: "".to_string() })
        };

        if test_case.expected_error {
            assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
            if let Err(error) = result {
                assert!(
                    matches!(error, Error::ServerVersion(_, _, _)),
                    "Test '{}' wrong error type",
                    test_case.name
                );
            }
        } else {
            assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
        }
    }
}

#[tokio::test]
async fn test_subscription_integration_table() {
    use crate::wsh::common::test_tables::subscription_integration_test_cases;

    for test_case in subscription_integration_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: test_case.response_messages,
        });

        let client = Client::stubbed(message_bus, test_case.server_version);
        let result = client.wsh_event_data_by_filter("test_filter", None, None).await;

        assert!(result.is_ok(), "Test '{}' failed to create subscription", test_case.name);
        let mut subscription = result.unwrap();

        let mut events = vec![];
        while let Some(event_result) = subscription.next_data().await {
            match event_result {
                Ok(event) => events.push(event.data_json),
                Err(e) => panic!("Test '{}' unexpected error: {e:?}", test_case.name),
            }
        }

        assert_eq!(
            events.len(),
            test_case.expected_events.len(),
            "Test '{}' event count mismatch",
            test_case.name
        );

        for (i, (received, expected)) in events.iter().zip(test_case.expected_events.iter()).enumerate() {
            assert_eq!(received, expected, "Test '{}' event {} mismatch", test_case.name, i);
        }
    }
}

#[tokio::test]
async fn test_data_stream_cancel_message() {
    use crate::subscriptions::StreamDecoder;
    use crate::testdata::builders::RequestEncoder;

    let request_id = test_data::REQUEST_ID_METADATA;

    let metadata_cancel = WshMetadata::cancel_message(0, Some(request_id), None).unwrap();
    let metadata_expected = cancel_wsh_metadata_request().request_id(request_id);
    assert_eq!(metadata_cancel, metadata_expected.encode_request());

    let event_cancel = WshEventData::cancel_message(0, Some(request_id), None).unwrap();
    let event_expected = cancel_wsh_event_data_request().request_id(request_id);
    assert_eq!(event_cancel, event_expected.encode_request());
}
