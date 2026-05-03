//! Wall Street Horizon: Earnings Calendar & Event Data.
//!
//! This module provides access to Wall Street Horizon data including
//! earnings calendars, corporate events, and other fundamental data
//! events that may impact trading decisions.

use std::str;

use serde::{Deserialize, Serialize};

mod common;

// Re-export common functionality
use common::encoders;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

/// Wall Street Horizon metadata containing configuration and setup information.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WshMetadata {
    /// JSON string containing metadata information from Wall Street Horizon.
    pub data_json: String,
}

/// Wall Street Horizon event data containing earnings calendar and corporate events.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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

// Async API methods are now on Client directly via wsh/async.rs

#[cfg(test)]
#[path = "common_tests.rs"]
mod common_tests;
