//! Wall Street Horizon: Earnings Calendar & Event Data.
//!
//! This module provides access to Wall Street Horizon data including
//! earnings calendars, corporate events, and other fundamental data
//! events that may impact trading decisions.

use std::str;

use serde::{Deserialize, Serialize};

mod common;

// Re-export common functionality
#[cfg(test)]
use common::decoders;
use common::encoders;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

/// Wall Street Horizon metadata containing configuration and setup information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WshMetadata {
    /// JSON string containing metadata information from Wall Street Horizon.
    pub data_json: String,
}

/// Wall Street Horizon event data containing earnings calendar and corporate events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WshEventData {
    /// JSON string containing event data from Wall Street Horizon.
    pub data_json: String,
}

/// Configuration for automatic filling of Wall Street Horizon event data.
///
/// This struct controls which types of securities should be automatically
/// included when requesting WSH event data. When enabled, the API will
/// include related securities based on the specified criteria.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AutoFill {
    /// Automatically fill in competitor values of existing positions.
    pub competitors: bool,
    /// Automatically fill in portfolio values.
    pub portfolio: bool,
    /// Automatically fill in watchlist values.
    pub watchlist: bool,
}

impl AutoFill {
    /// Returns true if any auto-fill option is enabled.
    pub fn is_specified(&self) -> bool {
        self.competitors || self.portfolio || self.watchlist
    }
}

// Re-export API functions based on active feature
#[cfg(feature = "sync")]
pub mod blocking {
    pub(crate) use super::sync::{wsh_event_data_by_contract, wsh_event_data_by_filter, wsh_metadata};
}

#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{wsh_event_data_by_contract, wsh_event_data_by_filter, wsh_metadata};

#[cfg(feature = "async")]
pub use r#async::{wsh_event_data_by_contract, wsh_event_data_by_filter, wsh_metadata};

// Tests that work with both sync and async features
#[cfg(test)]
mod common_tests {
    use super::*;
    use crate::messages::ResponseMessage;

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
    fn test_autofill_combinations() {
        // Test all possible combinations
        let combinations = vec![
            (false, false, false, false),
            (true, false, false, true),
            (false, true, false, true),
            (false, false, true, true),
            (true, true, false, true),
            (true, false, true, true),
            (false, true, true, true),
            (true, true, true, true),
        ];

        for (competitors, portfolio, watchlist, expected) in combinations {
            let autofill = AutoFill {
                competitors,
                portfolio,
                watchlist,
            };
            assert_eq!(
                autofill.is_specified(),
                expected,
                "Failed for combination: competitors={}, portfolio={}, watchlist={}",
                competitors,
                portfolio,
                watchlist
            );
        }
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
        use crate::server_versions;
        use time::macros::date;

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

    #[test]
    fn test_decode_wsh_metadata_empty_json() {
        use super::decoders::decode_wsh_metadata;

        let message = ResponseMessage::from("104\09000\0\0");
        let result = decode_wsh_metadata(message);

        assert!(result.is_ok(), "failed to decode empty wsh metadata: {}", result.err().unwrap());
        assert_eq!(result.unwrap().data_json, "");
    }

    #[test]
    fn test_decode_wsh_event_data_empty_json() {
        use super::decoders::decode_wsh_event_data;

        let message = ResponseMessage::from("105\09000\0\0");
        let result = decode_wsh_event_data(message);

        assert!(result.is_ok(), "failed to decode empty wsh event data: {}", result.err().unwrap());
        assert_eq!(result.unwrap().data_json, "");
    }

    #[test]
    fn test_decode_wsh_metadata_with_special_chars() {
        use super::decoders::decode_wsh_metadata;

        let message = ResponseMessage::from("104\09000\0{\"data\":\"test\\nwith\\tspecial\\rchars\"}\0");
        let result = decode_wsh_metadata(message);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().data_json, "{\"data\":\"test\\nwith\\tspecial\\rchars\"}");
    }

    #[test]
    fn test_encode_request_wsh_event_data_edge_cases() {
        use super::encoders::encode_request_wsh_event_data;
        use crate::server_versions;

        // Test with empty filter string
        let result = encode_request_wsh_event_data(server_versions::WSH_EVENT_DATA_FILTERS, 9000, None, Some(""), None, None, None, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().encode_simple(), "102|9000|||0|0|0|");

        // Test with special characters in filter
        let result = encode_request_wsh_event_data(
            server_versions::WSH_EVENT_DATA_FILTERS,
            9001,
            None,
            Some("filter=\"test\" AND type='earnings'"),
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().encode_simple(), "102|9001||filter=\"test\" AND type='earnings'|0|0|0|");

        // Test with negative limit (should still encode)
        let result = encode_request_wsh_event_data(
            server_versions::WSH_EVENT_DATA_FILTERS_DATE,
            9002,
            Some(12345),
            None,
            None,
            None,
            Some(-10),
            None,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().encode_simple(), "102|9002|12345||0|0|0|||-10|");
    }
}
