//! Table-driven test data for WSH module tests

use crate::common::test_utils::helpers::proto_response;
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::testdata::builders::wsh::{wsh_event_data_response, wsh_metadata_response};
use crate::testdata::builders::ResponseProtoEncoder;
use crate::wsh::AutoFill;
use time::macros::date;
use time::Date;

fn metadata_response(req_id: i32, data_json: &str) -> ResponseMessage {
    proto_response(
        IncomingMessages::WshMetaData,
        wsh_metadata_response().request_id(req_id).data_json(data_json).encode_proto(),
    )
}

fn event_data_response(req_id: i32, data_json: &str) -> ResponseMessage {
    proto_response(
        IncomingMessages::WshEventData,
        wsh_event_data_response().request_id(req_id).data_json(data_json).encode_proto(),
    )
}

/// Test case for StreamDecoder decode tests.
pub struct DecodeTestCase {
    pub name: &'static str,
    pub req_id: i32,
    pub data_json: &'static str,
}

/// Test cases for WshMetadata decode (proto-only). One happy-path case is
/// enough — the proto String round-trip has only the one failure mode.
/// Dispatch arms (`Error`, unexpected message type) are covered in
/// `stream_decoders.rs::tests` and `decoders.rs::tests::_rejects_text_framing`.
pub const WSH_METADATA_DECODE_TESTS: &[DecodeTestCase] = &[DecodeTestCase {
    name: "valid metadata",
    req_id: 9000,
    data_json: r#"{"test":"metadata"}"#,
}];

/// Test cases for WshEventData decode (proto-only). See WSH_METADATA_DECODE_TESTS.
pub const WSH_EVENT_DATA_DECODE_TESTS: &[DecodeTestCase] = &[DecodeTestCase {
    name: "valid event data",
    req_id: 9000,
    data_json: r#"{"test":"event"}"#,
}];

impl DecodeTestCase {
    pub fn metadata_message(&self) -> ResponseMessage {
        metadata_response(self.req_id, self.data_json)
    }
    pub fn event_data_message(&self) -> ResponseMessage {
        event_data_response(self.req_id, self.data_json)
    }
}

/// Test case for API function tests
pub struct ApiTestCase {
    pub name: &'static str,
    pub server_version: i32,
    pub response_messages: Vec<ResponseMessage>,
    pub expected_result: ApiExpectedResult,
}

pub enum ApiExpectedResult {
    Success { json: String },
    ServerVersionError,
}

/// Test cases for wsh_metadata function
pub fn wsh_metadata_test_cases() -> Vec<ApiTestCase> {
    vec![
        ApiTestCase {
            name: "successful metadata request",
            server_version: crate::server_versions::WSHE_CALENDAR,
            response_messages: vec![metadata_response(9000, r#"{"validated":true,"data":{"metadata":"test"}}"#)],
            expected_result: ApiExpectedResult::Success {
                json: r#"{"validated":true,"data":{"metadata":"test"}}"#.to_string(),
            },
        },
        ApiTestCase {
            name: "server version too old",
            server_version: 100,
            response_messages: vec![],
            expected_result: ApiExpectedResult::ServerVersionError,
        },
    ]
}

/// Test case for wsh_event_data_by_contract
pub struct EventDataByContractTestCase {
    pub name: &'static str,
    pub server_version: i32,
    pub contract_id: i32,
    pub start_date: Option<Date>,
    pub end_date: Option<Date>,
    pub limit: Option<i32>,
    pub auto_fill: Option<AutoFill>,
    pub response_messages: Vec<ResponseMessage>,
    pub expected_result: ApiExpectedResult,
}

pub fn event_data_by_contract_test_cases() -> Vec<EventDataByContractTestCase> {
    vec![
        EventDataByContractTestCase {
            name: "with all filters",
            server_version: crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE,
            contract_id: 12345,
            start_date: Some(date!(2024 - 01 - 01)),
            end_date: Some(date!(2024 - 12 - 31)),
            limit: Some(100),
            auto_fill: Some(AutoFill {
                competitors: true,
                portfolio: false,
                watchlist: true,
            }),
            response_messages: vec![event_data_response(9001, r#"{"validated":true,"data":{"events":[]}}"#)],
            expected_result: ApiExpectedResult::Success {
                json: r#"{"validated":true,"data":{"events":[]}}"#.to_string(),
            },
        },
        EventDataByContractTestCase {
            name: "no filters",
            server_version: crate::server_versions::WSHE_CALENDAR,
            contract_id: 12345,
            start_date: None,
            end_date: None,
            limit: None,
            auto_fill: None,
            response_messages: vec![event_data_response(9002, r#"{"events":[{"type":"earnings"}]}"#)],
            expected_result: ApiExpectedResult::Success {
                json: r#"{"events":[{"type":"earnings"}]}"#.to_string(),
            },
        },
        EventDataByContractTestCase {
            name: "date filters require newer version",
            server_version: crate::server_versions::WSH_EVENT_DATA_FILTERS,
            contract_id: 12345,
            start_date: Some(date!(2024 - 01 - 01)),
            end_date: None,
            limit: None,
            auto_fill: None,
            response_messages: vec![],
            expected_result: ApiExpectedResult::ServerVersionError,
        },
    ]
}

#[allow(dead_code)]
/// Test case for subscription-based tests
pub struct SubscriptionTestCase {
    pub name: &'static str,
    pub filter: &'static str,
    pub limit: Option<i32>,
    pub auto_fill: Option<AutoFill>,
    pub response_messages: Vec<ResponseMessage>,
    pub expected_events: Vec<String>,
}

#[allow(dead_code)]
pub fn subscription_test_cases() -> Vec<SubscriptionTestCase> {
    vec![SubscriptionTestCase {
        name: "multiple events",
        filter: "earnings",
        limit: Some(50),
        auto_fill: None,
        response_messages: vec![
            event_data_response(9003, r#"{"event":"earnings","date":"2024-01-15"}"#),
            event_data_response(9003, r#"{"event":"dividend","date":"2024-02-01"}"#),
        ],
        expected_events: vec![
            r#"{"event":"earnings","date":"2024-01-15"}"#.to_string(),
            r#"{"event":"dividend","date":"2024-02-01"}"#.to_string(),
        ],
    }]
}

/// Test case for integration tests with server version validation
pub struct IntegrationTestCase {
    pub name: &'static str,
    pub server_version: i32,
    pub response_messages: Vec<ResponseMessage>,
    pub expected_result: IntegrationExpectedResult,
}

pub enum IntegrationExpectedResult {
    Success,
    ServerVersionError,
}

/// Test cases for wsh_event_data_by_filter integration tests
pub fn event_data_by_filter_integration_test_cases() -> Vec<IntegrationTestCase> {
    vec![
        IntegrationTestCase {
            name: "successful filter request with autofill",
            server_version: crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE,
            response_messages: vec![event_data_response(9000, r#"{"validated":true,"data":{"events":[]}}"#)],
            expected_result: IntegrationExpectedResult::Success,
        },
        IntegrationTestCase {
            name: "successful filter request without autofill",
            server_version: crate::server_versions::WSH_EVENT_DATA_FILTERS,
            response_messages: vec![event_data_response(9000, r#"{"validated":true,"data":{"events":[]}}"#)],
            expected_result: IntegrationExpectedResult::Success,
        },
        IntegrationTestCase {
            name: "server version too old for filters",
            server_version: crate::server_versions::WSHE_CALENDAR,
            response_messages: vec![],
            expected_result: IntegrationExpectedResult::ServerVersionError,
        },
    ]
}

/// Test case for subscription integration tests
pub struct SubscriptionIntegrationTestCase {
    pub name: &'static str,
    pub server_version: i32,
    pub response_messages: Vec<ResponseMessage>,
    pub expected_events: Vec<String>,
}

pub fn subscription_integration_test_cases() -> Vec<SubscriptionIntegrationTestCase> {
    vec![SubscriptionIntegrationTestCase {
        name: "multiple events subscription",
        server_version: crate::server_versions::WSH_EVENT_DATA_FILTERS,
        response_messages: vec![
            event_data_response(9000, r#"{"event":1}"#),
            event_data_response(9000, r#"{"event":2}"#),
            event_data_response(9000, r#"{"event":3}"#),
        ],
        expected_events: vec![r#"{"event":1}"#.to_string(), r#"{"event":2}"#.to_string(), r#"{"event":3}"#.to_string()],
    }]
}

/// Test case for server version validation tests
pub struct ServerVersionTestCase {
    pub name: &'static str,
    pub server_version: i32,
    pub contract_id: Option<i32>,
    pub start_date: Option<time::Date>,
    pub end_date: Option<time::Date>,
    pub limit: Option<i32>,
    pub auto_fill: Option<AutoFill>,
    pub expected_error: bool,
}

pub fn server_version_test_cases() -> Vec<ServerVersionTestCase> {
    use time::macros::date;

    vec![
        ServerVersionTestCase {
            name: "filter request with old server version",
            server_version: crate::server_versions::WSHE_CALENDAR,
            contract_id: None,
            start_date: None,
            end_date: None,
            limit: None,
            auto_fill: None,
            expected_error: true,
        },
        ServerVersionTestCase {
            name: "contract request with date filters on old server version",
            server_version: crate::server_versions::WSH_EVENT_DATA_FILTERS,
            contract_id: Some(12345),
            start_date: Some(date!(2024 - 01 - 01)),
            end_date: Some(date!(2024 - 12 - 31)),
            limit: Some(100),
            auto_fill: None,
            expected_error: true,
        },
    ]
}
