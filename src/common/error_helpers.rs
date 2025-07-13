//! Common error handling utilities for the ibapi crate

#![allow(dead_code)] // These utilities will be used by other modules

use crate::Error;

/// Ensures a value is present, returning an error with the specified message if None
pub fn require<T>(value: Option<T>, error_message: &str) -> Result<T, Error> {
    value.ok_or_else(|| Error::Simple(error_message.to_string()))
}

/// Ensures a value is present, returning an error with a formatted message if None
pub fn require_with<T, F>(value: Option<T>, error_fn: F) -> Result<T, Error>
where
    F: FnOnce() -> String,
{
    value.ok_or_else(|| Error::Simple(error_fn()))
}

/// Ensures a request ID is present, returning an error if None
pub fn require_request_id(id: Option<i32>) -> Result<i32, Error> {
    require(id, "Request ID required")
}

/// Ensures a request ID is present with a custom error message
pub fn require_request_id_for(id: Option<i32>, operation: &str) -> Result<i32, Error> {
    require_with(id, || format!("Request ID required to {}", operation))
}

/// Ensures a value is within a valid range
pub fn require_range<T>(value: T, min: T, max: T, name: &str) -> Result<T, Error>
where
    T: PartialOrd + std::fmt::Display,
{
    if value < min || value > max {
        Err(Error::Simple(format!("{} must be between {} and {}, got {}", name, min, max, value)))
    } else {
        Ok(value)
    }
}

/// Ensures a string is not empty
pub fn require_not_empty<'a>(value: &'a str, name: &str) -> Result<&'a str, Error> {
    if value.is_empty() {
        Err(Error::Simple(format!("{} cannot be empty", name)))
    } else {
        Ok(value)
    }
}

/// Ensures a collection has at least one element
pub fn require_not_empty_vec<'a, T>(value: &'a [T], name: &str) -> Result<&'a [T], Error> {
    if value.is_empty() {
        Err(Error::Simple(format!("{} must contain at least one element", name)))
    } else {
        Ok(value)
    }
}

/// Converts a Result<T, E> to Result<T, Error> with a custom error message
pub fn map_error<T, E>(result: Result<T, E>, error_message: &str) -> Result<T, Error>
where
    E: std::fmt::Display,
{
    result.map_err(|e| Error::Simple(format!("{}: {}", error_message, e)))
}

/// Converts a Result<T, E> to Result<T, Error> with a custom error function
pub fn map_error_with<T, E, F>(result: Result<T, E>, error_fn: F) -> Result<T, Error>
where
    E: std::fmt::Display,
    F: FnOnce(&E) -> String,
{
    result.map_err(|e| Error::Simple(error_fn(&e)))
}
