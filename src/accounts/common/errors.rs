//! Error handling utilities for the accounts module

use crate::Error;

/// Ensures a request ID is present, returning an error if None
#[allow(dead_code)]
pub(in crate::accounts) fn require_request_id(id: Option<i32>) -> Result<i32, Error> {
    id.ok_or_else(|| Error::Simple("Request ID required".into()))
}

/// Ensures a request ID is present with a custom error message
pub(in crate::accounts) fn require_request_id_for(id: Option<i32>, operation: &str) -> Result<i32, Error> {
    id.ok_or_else(|| Error::Simple(format!("Request ID required to {}", operation)))
}