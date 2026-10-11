//! Shared test data and fixtures for WSH module tests

/// Sample JSON responses for testing
pub mod json_responses {
    pub const METADATA_SIMPLE: &str = r#"{"validated":true,"data":{"metadata":"test"}}"#;
    #[cfg(feature = "sync")]
    pub const EVENT_DATA_EARNINGS: &str = r#"{"event":"earnings","date":"2024-01-15"}"#;
    #[cfg(feature = "sync")]
    pub const EVENT_DATA_DIVIDEND: &str = r#"{"event":"dividend","date":"2024-02-01"}"#;
}

/// Sample request IDs for testing
pub const REQUEST_ID_METADATA: i32 = 9000;
#[cfg(feature = "sync")]
pub const REQUEST_ID_FILTER: i32 = 9003;

/// Common test filter
#[cfg(feature = "sync")]
pub const TEST_FILTER: &str = "earnings";
