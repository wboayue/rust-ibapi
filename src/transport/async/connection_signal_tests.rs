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
