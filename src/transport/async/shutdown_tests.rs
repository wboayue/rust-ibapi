use std::sync::Arc;
use std::time::Instant;

use super::*;

#[tokio::test]
async fn sleep_returns_when_shutdown_requested() {
    let signal = Arc::new(ShutdownSignal::default());
    assert!(!signal.is_requested());

    let waker = Arc::clone(&signal);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        waker.request();
    });

    let start = Instant::now();
    signal.sleep(Duration::from_secs(30)).await;

    assert!(signal.is_requested());
    assert!(start.elapsed() < Duration::from_secs(5), "sleep did not return on request");
}

#[tokio::test]
async fn wait_returns_immediately_when_already_requested() {
    let signal = ShutdownSignal::default();
    signal.request();

    tokio::time::timeout(Duration::from_secs(5), signal.wait())
        .await
        .expect("latched request must not block");
}

#[tokio::test]
async fn sleep_expires_without_a_request() {
    let signal = ShutdownSignal::default();

    signal.sleep(Duration::from_millis(10)).await;

    assert!(!signal.is_requested());
}
