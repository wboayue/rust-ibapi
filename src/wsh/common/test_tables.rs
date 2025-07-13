//! Table-driven test data for WSH module tests

use super::test_data;
use crate::wsh::AutoFill;
use time::macros::date;
use time::Date;

/// Test case for StreamDecoder decode tests
pub struct DecodeTestCase {
    pub name: &'static str,
    pub message: &'static str,
    pub expected_json: &'static str,
    pub should_error: bool,
    pub error_type: Option<&'static str>,
}

/// Test cases for WshMetadata decode
pub const WSH_METADATA_DECODE_TESTS: &[DecodeTestCase] = &[
    DecodeTestCase {
        name: "valid metadata",
        message: "104\09000\0{\"test\":\"metadata\"}\0",
        expected_json: r#"{"test":"metadata"}"#,
        should_error: false,
        error_type: None,
    },
    DecodeTestCase {
        name: "empty metadata",
        message: "104\09000\0\0",
        expected_json: "",
        should_error: false,
        error_type: None,
    },
    DecodeTestCase {
        name: "metadata with special chars",
        message: "104\09000\0{\"data\":\"test\\nwith\\tspecial\\rchars\"}\0",
        expected_json: r#"{"data":"test\nwith\tspecial\rchars"}"#,
        should_error: false,
        error_type: None,
    },
    DecodeTestCase {
        name: "unexpected message type",
        message: "1\09000\0unexpected\0",
        expected_json: "",
        should_error: true,
        error_type: Some("UnexpectedResponse"),
    },
];

/// Test cases for WshEventData decode
pub const WSH_EVENT_DATA_DECODE_TESTS: &[DecodeTestCase] = &[
    DecodeTestCase {
        name: "valid event data",
        message: "105\09000\0{\"test\":\"event\"}\0",
        expected_json: r#"{"test":"event"}"#,
        should_error: false,
        error_type: None,
    },
    DecodeTestCase {
        name: "empty event data",
        message: "105\09000\0\0",
        expected_json: "",
        should_error: false,
        error_type: None,
    },
    DecodeTestCase {
        name: "error message",
        message: "4\02\09000\0321\0Test error message\0",
        expected_json: "",
        should_error: true,
        error_type: Some("Message"),
    },
    DecodeTestCase {
        name: "unexpected message type",
        message: "1\09000\0unexpected\0",
        expected_json: "",
        should_error: true,
        error_type: Some("UnexpectedResponse"),
    },
];

/// Test case for API function tests
pub struct ApiTestCase {
    pub name: &'static str,
    pub server_version: i32,
    pub response_messages: Vec<String>,
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
            response_messages: vec![test_data::build_response("104", 9000, r#"{"validated":true,"data":{"metadata":"test"}}"#)],
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
    pub response_messages: Vec<String>,
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
            response_messages: vec![test_data::build_response("105", 9001, r#"{"validated":true,"data":{"events":[]}}"#)],
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
            response_messages: vec![test_data::build_response("105", 9002, r#"{"events":[{"type":"earnings"}]}"#)],
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
    pub response_messages: Vec<String>,
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
            test_data::build_response("105", 9003, r#"{"event":"earnings","date":"2024-01-15"}"#),
            test_data::build_response("105", 9003, r#"{"event":"dividend","date":"2024-02-01"}"#),
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
    pub response_messages: Vec<String>,
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
            response_messages: vec!["105|9000|{\"validated\":true,\"data\":{\"events\":[]}}|".to_string()],
            expected_result: IntegrationExpectedResult::Success,
        },
        IntegrationTestCase {
            name: "successful filter request without autofill",
            server_version: crate::server_versions::WSH_EVENT_DATA_FILTERS,
            response_messages: vec!["105|9000|{\"validated\":true,\"data\":{\"events\":[]}}|".to_string()],
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
    pub response_messages: Vec<String>,
    pub expected_events: Vec<String>,
}

pub fn subscription_integration_test_cases() -> Vec<SubscriptionIntegrationTestCase> {
    vec![SubscriptionIntegrationTestCase {
        name: "multiple events subscription",
        server_version: crate::server_versions::WSH_EVENT_DATA_FILTERS,
        response_messages: vec![
            "105|9000|{\"event\":1}|".to_string(),
            "105|9000|{\"event\":2}|".to_string(),
            "105|9000|{\"event\":3}|".to_string(),
        ],
        expected_events: vec!["{\"event\":1}".to_string(), "{\"event\":2}".to_string(), "{\"event\":3}".to_string()],
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
