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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_some_value() {
        let result = require(Some(42), "Value required");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_require_none_value() {
        let result: Result<i32, Error> = require(None, "Value required");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "Value required"));
    }

    #[test]
    fn test_require_with_some_value() {
        let result = require_with(Some("hello"), || "Value is missing".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_require_with_none_value() {
        let result: Result<&str, Error> = require_with(None, || "Custom error message".to_string());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "Custom error message"));
    }

    #[test]
    fn test_require_request_id_some() {
        let result = require_request_id(Some(123));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    }

    #[test]
    fn test_require_request_id_none() {
        let result = require_request_id(None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "Request ID required"));
    }

    #[test]
    fn test_require_request_id_for_some() {
        let result = require_request_id_for(Some(456), "process order");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 456);
    }

    #[test]
    fn test_require_request_id_for_none() {
        let result = require_request_id_for(None, "cancel subscription");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "Request ID required to cancel subscription"));
    }

    #[test]
    fn test_require_range_valid() {
        let result = require_range(5, 1, 10, "value");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }

    #[test]
    fn test_require_range_too_low() {
        let result = require_range(0, 1, 10, "value");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "value must be between 1 and 10, got 0"));
    }

    #[test]
    fn test_require_range_too_high() {
        let result = require_range(15, 1, 10, "value");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "value must be between 1 and 10, got 15"));
    }

    #[test]
    fn test_require_range_boundary_values() {
        assert!(require_range(1, 1, 10, "value").is_ok());
        assert!(require_range(10, 1, 10, "value").is_ok());
    }

    #[test]
    fn test_require_not_empty_valid() {
        let result = require_not_empty("hello", "name");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_require_not_empty_invalid() {
        let result = require_not_empty("", "name");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "name cannot be empty"));
    }

    #[test]
    fn test_require_not_empty_vec_valid() {
        let vec = vec![1, 2, 3];
        let result = require_not_empty_vec(&vec, "numbers");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), &vec[..]);
    }

    #[test]
    fn test_require_not_empty_vec_invalid() {
        let vec: Vec<i32> = vec![];
        let result = require_not_empty_vec(&vec, "numbers");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "numbers must contain at least one element"));
    }

    #[test]
    fn test_map_error_ok() {
        let ok_result: Result<i32, &str> = Ok(42);
        let result = map_error(ok_result, "Failed to process");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_map_error_err() {
        let err_result: Result<i32, &str> = Err("internal error");
        let result = map_error(err_result, "Failed to process");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "Failed to process: internal error"));
    }

    #[test]
    fn test_map_error_with_ok() {
        let ok_result: Result<&str, &str> = Ok("success");
        let result = map_error_with(ok_result, |e| format!("Custom error: {}", e));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[test]
    fn test_map_error_with_err() {
        let err_result: Result<&str, &str> = Err("not found");
        let result = map_error_with(err_result, |e| format!("Could not find resource: {}", e));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Simple(msg) if msg == "Could not find resource: not found"));
    }

    #[test]
    fn test_require_range_with_floats() {
        let result = require_range(5.5, 0.0, 10.0, "percentage");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5.5);

        let result = require_range(10.1, 0.0, 10.0, "percentage");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_message_formatting() {
        // Test that error messages are properly formatted
        let result = require_range(-5, 0, 100, "temperature");
        assert!(result.is_err());
        if let Err(Error::Simple(msg)) = result {
            assert_eq!(msg, "temperature must be between 0 and 100, got -5");
        } else {
            panic!("Expected Error::Simple");
        }
    }
}
