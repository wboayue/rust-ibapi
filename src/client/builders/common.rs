//! Common types and traits for builders

use crate::messages::OutgoingMessages;

/// Context for response handling
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ResponseContext {
    pub is_smart_depth: bool,
    pub request_type: Option<OutgoingMessages>,
}
