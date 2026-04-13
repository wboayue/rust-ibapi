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
            let result = client.wsh_metadata();

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
            let result = client.wsh_event_data_by_contract(
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
        let result = client.wsh_event_data_by_filter(test_data::TEST_FILTER, Some(50), None);

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
    fn test_wsh_event_data_by_filter_integration_table() {
        use crate::wsh::common::test_tables::{event_data_by_filter_integration_test_cases, IntegrationExpectedResult};

        for test_case in event_data_by_filter_integration_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages,
            });

            let client = Client::stubbed(message_bus, test_case.server_version);

            let result = match test_case.name {
                "successful filter request with autofill" => {
                    let filter = "filter=value";
                    let result = client.wsh_event_data_by_filter(
                        filter,
                        Some(100),
                        Some(AutoFill {
                            competitors: true,
                            portfolio: false,
                            watchlist: true,
                        }),
                    );

                    if result.is_ok() {
                        use crate::common::test_utils::helpers::assert_proto_msg_id;
                        use crate::messages::OutgoingMessages;
                        let request_messages = client.message_bus.request_messages();
                        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestWshEventData);
                    }
                    result
                }
                "successful filter request without autofill" => {
                    let filter = "filter=value";
                    let result = client.wsh_event_data_by_filter(filter, None, None);

                    if result.is_ok() {
                        use crate::common::test_utils::helpers::assert_proto_msg_id;
                        use crate::messages::OutgoingMessages;
                        let request_messages = client.message_bus.request_messages();
                        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestWshEventData);
                    }
                    result
                }
                _ => client.wsh_event_data_by_filter("filter", None, None),
            };

            match test_case.expected_result {
                IntegrationExpectedResult::Success => {
                    assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                }
                IntegrationExpectedResult::ServerVersionError => {
                    assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                    assert!(
                        matches!(result.as_ref().err(), Some(Error::ServerVersion(_, _, _))),
                        "Test '{}' wrong error type",
                        test_case.name
                    );
                }
            }
        }
    }

    #[test]
    fn test_server_version_validation_table() {
        use crate::wsh::common::test_tables::server_version_test_cases;

        for test_case in server_version_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: vec![],
            });

            let client = Client::stubbed(message_bus, test_case.server_version);

            let result = if let Some(contract_id) = test_case.contract_id {
                client.wsh_event_data_by_contract(
                    contract_id,
                    test_case.start_date,
                    test_case.end_date,
                    test_case.limit,
                    test_case.auto_fill,
                )
            } else {
                client
                    .wsh_event_data_by_filter("filter", test_case.limit, test_case.auto_fill)
                    .map(|_| WshEventData { data_json: "".to_string() })
            };

            if test_case.expected_error {
                assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                assert!(
                    matches!(result.unwrap_err(), Error::ServerVersion(_, _, _)),
                    "Test '{}' wrong error type",
                    test_case.name
                );
            } else {
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
            }
        }
    }

    #[test]
    fn test_subscription_integration_table() {
        use crate::wsh::common::test_tables::subscription_integration_test_cases;

        for test_case in subscription_integration_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages,
            });

            let client = Client::stubbed(message_bus, test_case.server_version);
            let result = client.wsh_event_data_by_filter("test_filter", None, None);

            assert!(result.is_ok(), "Test '{}' failed to create subscription", test_case.name);
            let subscription = result.unwrap();

            // Collect all events
            let mut events = vec![];
            while let Some(event) = subscription.next() {
                events.push(event.data_json);
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

    #[test]
    fn test_data_stream_cancel_message() {
        use crate::common::test_utils::helpers::assert_proto_msg_id;
        use crate::messages::OutgoingMessages;

        let request_id = test_data::REQUEST_ID_METADATA;

        // Test WshMetadata cancel
        let cancel_msg = WshMetadata::cancel_message(0, Some(request_id), None);
        assert!(cancel_msg.is_ok());
        assert_proto_msg_id(&cancel_msg.unwrap(), OutgoingMessages::CancelWshMetaData);

        // Test WshEventData cancel
        let cancel_msg = WshEventData::cancel_message(0, Some(request_id), None);
        assert!(cancel_msg.is_ok());
        assert_proto_msg_id(&cancel_msg.unwrap(), OutgoingMessages::CancelWshEventData);
    }

    #[test]
    fn test_wsh_metadata_decode_table() {
        use crate::subscriptions::DecoderContext;
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

    #[test]
    fn test_wsh_event_data_decode_table() {
        use crate::subscriptions::DecoderContext;
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
}
