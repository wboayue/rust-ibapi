use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::*;

#[test]
fn wait_connected_returns_immediately_while_connected() {
    let signal = ConnectionSignal::default();
    assert!(signal.is_connected());

    let start = Instant::now();
    assert!(signal.wait_connected().is_ok());
    assert!(start.elapsed() < Duration::from_secs(5), "connected wait must not block");
}

#[test]
fn wait_connected_returns_when_the_session_comes_back() {
    let signal = Arc::new(ConnectionSignal::default());
    signal.set_disconnected();
    assert!(!signal.is_connected());

    let waker = Arc::clone(&signal);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        waker.set_connected();
    });

    let start = Instant::now();
    assert!(signal.wait_connected().is_ok());
    assert!(signal.is_connected());
    assert!(start.elapsed() < Duration::from_secs(5), "wait_connected did not return on reconnect");
}

#[test]
fn wait_connected_returns_shutdown_when_requested_during_the_wait() {
    let signal = Arc::new(ConnectionSignal::default());
    signal.set_disconnected();

    let waker = Arc::clone(&signal);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        waker.shutdown();
    });

    let start = Instant::now();
    assert!(matches!(signal.wait_connected(), Err(Error::Shutdown)));
    assert!(start.elapsed() < Duration::from_secs(5), "wait_connected did not return on shutdown");
}

#[test]
fn shutdown_is_terminal() {
    let signal = ConnectionSignal::default();
    signal.shutdown();

    signal.set_connected();

    assert!(!signal.is_connected(), "a late reconnect must not revive a shut-down session");
    assert!(matches!(signal.wait_connected(), Err(Error::Shutdown)));
}
