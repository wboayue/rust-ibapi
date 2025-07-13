//! Asynchronous implementation of Wall Street Horizon functionality

use time::Date;

use crate::{
    client::ClientRequestBuilders,
    protocol::{check_version, Features},
    subscriptions::Subscription,
    Client, Error,
};

use super::{encoders, AutoFill, WshEventData, WshMetadata};

pub async fn wsh_metadata(client: &Client) -> Result<WshMetadata, Error> {
    check_version(client.server_version(), Features::WSHE_CALENDAR)?;

    let builder = client.request();
    let request = encoders::encode_request_wsh_metadata(builder.request_id())?;
    let mut subscription = builder.send::<WshMetadata>(request).await?;

    match subscription.next().await {
        Some(Ok(metadata)) => Ok(metadata),
        Some(Err(Error::ConnectionReset)) => Box::pin(wsh_metadata(client)).await,
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

pub async fn wsh_event_data_by_contract(
    client: &Client,
    contract_id: i32,
    start_date: Option<Date>,
    end_date: Option<Date>,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<WshEventData, Error> {
    check_version(client.server_version(), Features::WSHE_CALENDAR)?;

    if auto_fill.is_some() {
        check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS)?;
    }

    if start_date.is_some() || end_date.is_some() || limit.is_some() {
        check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS_DATE)?;
    }

    let builder = client.request();
    let request = encoders::encode_request_wsh_event_data(
        client.server_version(),
        builder.request_id(),
        Some(contract_id),
        None,
        start_date,
        end_date,
        limit,
        auto_fill,
    )?;
    let mut subscription = builder.send::<WshEventData>(request).await?;

    match subscription.next().await {
        Some(Ok(event_data)) => Ok(event_data),
        Some(Err(Error::ConnectionReset)) => Box::pin(wsh_event_data_by_contract(client, contract_id, start_date, end_date, limit, auto_fill)).await,
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

pub async fn wsh_event_data_by_filter(
    client: &Client,
    filter: &str,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<Subscription<WshEventData>, Error> {
    check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS)?;

    if limit.is_some() {
        check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS_DATE)?;
    }

    let builder = client.request();
    let request = encoders::encode_request_wsh_event_data(
        client.server_version(),
        builder.request_id(),
        None,
        Some(filter),
        None, // start_date
        None, // end_date
        limit,
        auto_fill,
    )?;

    builder.send::<WshEventData>(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::MessageBusStub;
    use crate::wsh::common::test_data::{self};
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
            let result = wsh_metadata(&client).await;

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
    async fn test_wsh_event_data_by_contract_table() {
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
            )
            .await;

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
    async fn test_wsh_event_data_by_filter_subscription_table() {
        use crate::wsh::common::test_tables::subscription_test_cases;

        for test_case in subscription_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages,
            });

            let client = Client::stubbed(message_bus, crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE);
            let mut subscription = wsh_event_data_by_filter(&client, test_case.filter, test_case.limit, test_case.auto_fill)
                .await
                .unwrap_or_else(|_| panic!("Test '{}' failed to create subscription", test_case.name));

            let mut received_events = vec![];
            while let Some(result) = subscription.next().await {
                match result {
                    Ok(event) => received_events.push(event.data_json),
                    Err(e) => panic!("Test '{}' unexpected error: {:?}", test_case.name, e),
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
        use crate::subscriptions::StreamDecoder;
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

    #[tokio::test]
    async fn test_wsh_event_data_decode_table() {
        use crate::messages::ResponseMessage;
        use crate::subscriptions::StreamDecoder;
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

    #[tokio::test]
    async fn test_data_stream_cancel_message() {
        use crate::subscriptions::StreamDecoder;

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
}
