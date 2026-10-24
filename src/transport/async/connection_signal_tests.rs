use std::sync::Arc;
use std::time::Duration;

use super::*;

#[tokio::test]
async fn wait_connected_returns_immediately_while_connected() {
    let signal = ConnectionSignal::default();
    assert!(signal.is_connected());

    tokio::time::timeout(Duration::from_secs(5), signal.wait_connected())
        .await
        .expect("connected wait must not block")
        .expect("wait_connected");
}

#[tokio::test]
async fn wait_connected_returns_when_the_session_comes_back() {
    let signal = Arc::new(ConnectionSignal::default());
    signal.set_disconnected();
    assert!(!signal.is_connected());

    let waker = Arc::clone(&signal);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        waker.set_connected();
    });

    tokio::time::timeout(Duration::from_secs(5), signal.wait_connected())
        .await
        .expect("wait_connected did not return on reconnect")
        .expect("wait_connected");
    assert!(signal.is_connected());
}

#[tokio::test]
async fn wait_connected_returns_shutdown_when_requested_during_the_wait() {
    let signal = Arc::new(ConnectionSignal::default());
    signal.set_disconnected();

    let waker = Arc::clone(&signal);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        waker.shutdown();
    });

    let result = tokio::time::timeout(Duration::from_secs(5), signal.wait_connected())
        .await
        .expect("wait_connected did not return on shutdown");
    assert!(matches!(result, Err(Error::Shutdown)), "got {result:?}");
}

#[tokio::test]
async fn shutdown_is_terminal() {
    let signal = ConnectionSignal::default();
    signal.shutdown();

    signal.set_connected();

    assert!(!signal.is_connected(), "a late reconnect must not revive a shut-down session");
    let result = tokio::time::timeout(Duration::from_secs(5), signal.wait_connected())
        .await
        .expect("wait_connected blocked after shutdown");
    assert!(matches!(result, Err(Error::Shutdown)), "got {result:?}");
}

#[tokio::test]
async fn check_tells_reconnecting_from_gone() {
    let signal = ConnectionSignal::default();
    assert!(signal.check().is_ok());

    signal.set_disconnected();
    assert!(matches!(signal.check(), Err(Error::ConnectionReset)));

    signal.fail();
    assert!(matches!(signal.check(), Err(Error::ConnectionFailed)));

    let signal = ConnectionSignal::default();
    signal.shutdown();
    assert!(matches!(signal.check(), Err(Error::Shutdown)));
}

#[tokio::test]
async fn wait_connected_returns_connection_failed_when_the_reconnect_gives_up() {
    let signal = Arc::new(ConnectionSignal::default());
    signal.set_disconnected();

    let waker = Arc::clone(&signal);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        waker.fail();
    });

    let result = tokio::time::timeout(Duration::from_secs(5), signal.wait_connected())
        .await
        .expect("wait_connected did not return on failure");
    assert!(matches!(result, Err(Error::ConnectionFailed)), "got {result:?}");
}

/// The dispatcher latches `Failed` and then `retire` shuts the bus down; the
/// shutdown must not replace why the session ended.
#[tokio::test]
async fn failed_survives_the_following_shutdown_and_a_late_reconnect() {
    let signal = ConnectionSignal::default();
    signal.set_disconnected();
    signal.fail();

    signal.shutdown();
    signal.set_connected();

    assert!(!signal.is_connected());
    let result = tokio::time::timeout(Duration::from_secs(5), signal.wait_connected())
        .await
        .expect("wait_connected blocked after failure");
    assert!(matches!(result, Err(Error::ConnectionFailed)), "got {result:?}");
}
