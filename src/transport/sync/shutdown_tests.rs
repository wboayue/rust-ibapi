use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::*;

#[test]
fn wait_timeout_returns_when_shutdown_requested() {
    let signal = Arc::new(ShutdownSignal::default());
    assert!(!signal.is_requested());

    let waker = Arc::clone(&signal);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        waker.request();
    });

    let start = Instant::now();
    signal.wait_timeout(Duration::from_secs(30));

    assert!(signal.is_requested());
    assert!(start.elapsed() < Duration::from_secs(5), "wait_timeout did not return on request");
}

#[test]
fn wait_timeout_returns_immediately_when_already_requested() {
    let signal = ShutdownSignal::default();
    signal.request();

    let start = Instant::now();
    signal.wait_timeout(Duration::from_secs(30));

    assert!(start.elapsed() < Duration::from_secs(5), "latched request must not block");
}

#[test]
fn wait_timeout_expires_without_a_request() {
    let signal = ShutdownSignal::default();

    signal.wait_timeout(Duration::from_millis(10));

    assert!(!signal.is_requested());
}
