use crate::transport::TcpSocket;
use time::macros::datetime;
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt};

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
