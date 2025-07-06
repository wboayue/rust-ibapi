//! Common types and traits for builders

use crate::messages::OutgoingMessages;

/// Context for response handling
#[derive(Debug, Clone, Default)]
pub struct ResponseContext {
    pub is_smart_depth: bool,
    pub request_type: Option<OutgoingMessages>,
}

/// Common trait for version checking
pub trait VersionChecker {
    fn check_server_version(&self, required_version: i32, feature: &str) -> Result<(), crate::Error>;
}
