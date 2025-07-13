//! Shared test data and fixtures for WSH module tests

#![allow(dead_code)] // Test utilities may not all be used immediately

use time::macros::date;
use time::Date;

use crate::wsh::AutoFill;

/// Sample JSON responses for testing
pub mod json_responses {
    pub const METADATA_SIMPLE: &str = r#"{"validated":true,"data":{"metadata":"test"}}"#;
    pub const METADATA_EMPTY: &str = "{}";
    pub const METADATA_SPECIAL_CHARS: &str = r#"{"data":"Special chars: \n\t\"quoted\""}"#;

    pub const EVENT_DATA_SIMPLE: &str = r#"{"validated":true,"data":{"events":[]}}"#;
    pub const EVENT_DATA_EMPTY: &str = "{}";
    pub const EVENT_DATA_EARNINGS: &str = r#"{"event":"earnings","date":"2024-01-15"}"#;
    pub const EVENT_DATA_DIVIDEND: &str = r#"{"event":"dividend","date":"2024-02-01"}"#;
    pub const EVENT_DATA_NO_FILTERS: &str = r#"{"events":[{"type":"earnings"}]}"#;
}

/// Sample request IDs for testing
pub const REQUEST_ID_METADATA: i32 = 9000;
pub const REQUEST_ID_EVENT_DATA: i32 = 9001;
pub const REQUEST_ID_FILTER: i32 = 9003;

/// Sample test dates
pub fn test_start_date() -> Date {
    date!(2024 - 01 - 01)
}

pub fn test_end_date() -> Date {
    date!(2024 - 12 - 31)
}

/// Sample AutoFill configurations
pub fn autofill_all_true() -> AutoFill {
    AutoFill {
        competitors: true,
        portfolio: true,
        watchlist: true,
    }
}

pub fn autofill_mixed() -> AutoFill {
    AutoFill {
        competitors: true,
        portfolio: false,
        watchlist: true,
    }
}

/// Common test contract ID
pub const TEST_CONTRACT_ID: i32 = 12345;

/// Common test filter
pub const TEST_FILTER: &str = "earnings";

/// Test server versions
pub mod server_versions {
    pub const OLD_VERSION: i32 = 100;
}

/// Helper to build test message responses
pub fn build_response(message_type: &str, request_id: i32, data: &str) -> String {
    format!("{}|{}|{}|", message_type, request_id, data)
}

/// Helper to build error response
pub fn build_error_response(request_id: i32, error_code: i32, error_msg: &str) -> String {
    format!("4|2|{}|{}|{}|", request_id, error_code, error_msg)
}
