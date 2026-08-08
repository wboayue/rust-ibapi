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
    assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "Value required"));
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
    assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "Custom error message"));
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
    assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "Request ID required"));
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
    assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "Request ID required to cancel subscription"));
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
    assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "value must be between 1 and 10, got 0"));
}

#[test]
fn test_require_range_too_high() {
    let result = require_range(15, 1, 10, "value");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "value must be between 1 and 10, got 15"));
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
    assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "name cannot be empty"));
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
    assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "numbers must contain at least one element"));
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
    assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "Failed to process: internal error"));
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
    assert!(matches!(result.unwrap_err(), Error::InvalidArgument(msg) if msg == "Could not find resource: not found"));
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
    if let Err(Error::InvalidArgument(msg)) = result {
        assert_eq!(msg, "temperature must be between 0 and 100, got -5");
    } else {
        panic!("Expected Error::InvalidArgument");
    }
}
