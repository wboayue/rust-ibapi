//! Shared fixtures for the historical sync and async tests.

use crate::messages::HMDS_QUERY_MESSAGE_CODE;

/// The 165 text seen live on a historical request when the HMDS farm connected (#1136).
pub(crate) const HMDS_CONNECTED_MESSAGE: &str = "Historical Market Data Service query message:HMDS server connection was successful.";

/// Notices TWS sends before the bars of a request that still succeeds.
pub(crate) const PRE_DATA_NOTICES: [(i32, &str); 2] = [
    (2188, "Up-to-the-second historical data requires additional subscription for the API."),
    (HMDS_QUERY_MESSAGE_CODE, HMDS_CONNECTED_MESSAGE),
];
