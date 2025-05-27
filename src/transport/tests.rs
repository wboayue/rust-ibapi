use crate::transport::TcpSocket;
use time::macros::datetime;
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt};

use crate::messages::ResponseMessage;
use crate::tests::assert_send_and_sync;

use super::*;

#[test]
fn test_thread_safe() {
    assert_send_and_sync::<Connection<TcpSocket>>();
    assert_send_and_sync::<TcpMessageBus<TcpSocket>>();
}

#[test]
fn test_parse_connection_time() {
    let example = "20230405 22:20:39 PST";
    let (connection_time, _) = parse_connection_time(example);

    let la = timezones::db::america::LOS_ANGELES;
    if let OffsetResult::Some(other) = datetime!(2023-04-05 22:20:39).assume_timezone(la) {
        assert_eq!(connection_time, Some(other));
    }
}

#[test]
fn test_fibonacci_backoff() {
    let mut backoff = FibonacciBackoff::new(10);

    assert_eq!(backoff.next_delay(), Duration::from_secs(1));
    assert_eq!(backoff.next_delay(), Duration::from_secs(2));
    assert_eq!(backoff.next_delay(), Duration::from_secs(3));
    assert_eq!(backoff.next_delay(), Duration::from_secs(5));
    assert_eq!(backoff.next_delay(), Duration::from_secs(8));
    assert_eq!(backoff.next_delay(), Duration::from_secs(10));
    assert_eq!(backoff.next_delay(), Duration::from_secs(10));
}

#[test]
fn test_error_event_warning_handling() {
    // Test that warning error codes (2100-2169) are handled correctly
    let server_version = 100;

    // Create a warning message (error code 2104 is a common warning)
    // Format: "4|2|123|2104|Market data farm connection is OK:usfarm.nj"
    let warning_message = ResponseMessage::from_simple("4|2|123|2104|Market data farm connection is OK:usfarm.nj");

    // This should not panic and should handle as a warning
    let result = error_event(server_version, warning_message);
    assert!(result.is_ok());

    // Test actual error (non-warning code)
    // Format: "4|2|456|200|No security definition has been found"
    let error_message = ResponseMessage::from_simple("4|2|456|200|No security definition has been found");

    // This should also not panic and should handle as an error
    let result = error_event(server_version, error_message);
    assert!(result.is_ok());
}
