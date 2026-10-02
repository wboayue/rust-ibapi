use std::sync::{Arc, Mutex};
use std::time::Duration;

use ibapi::{Client, Error, StartupMessage};
use ibapi_test::{rate_limit, ClientId};

#[tokio::test]
async fn connect_to_gateway() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).await.expect("connection failed");

    assert!(client.server_version() > 0);
    assert!(client.connection_time().is_some());

    rate_limit();
    let time = client.server_time().await.expect("failed to get server time");
    assert!(time.year() >= 2025);
}

#[tokio::test]
async fn builder_startup_callback_fires_during_handshake() {
    let client_id = ClientId::get();
    let count = Arc::new(Mutex::new(0_usize));
    let count_clone = count.clone();

    rate_limit();
    let client = Client::builder()
        .address("127.0.0.1:4002")
        .client_id(client_id.id())
        .startup_callback(move |msg| {
            if let StartupMessage::OpenOrder(ref o) = msg {
                assert!(o.order_id >= 0);
            }
            *count_clone.lock().unwrap() += 1;
        })
        .connect()
        .await
        .expect("connection failed");

    assert!(client.server_version() > 0);
    println!("startup callback fired {} times", *count.lock().unwrap());
}

#[tokio::test]
async fn builder_tcp_no_delay_round_trips() {
    let client_id = ClientId::get();

    rate_limit();
    let client = Client::builder()
        .address("127.0.0.1:4002")
        .client_id(client_id.id())
        .tcp_no_delay(true)
        .connect()
        .await
        .expect("connection failed");

    assert!(client.server_version() > 0);
}

/// Canonical live test: the paper gateway always emits at least one farm-status
/// notice (2104 / 2106 / 2107 / 2108 / 2158) during the handshake. The pre-bound
/// `NoticeStream` from `connect_with_notice_stream()` captures them.
#[tokio::test]
async fn builder_notice_stream_receives_handshake_notices() {
    let client_id = ClientId::get();

    rate_limit();
    let (_client, mut notices) = Client::builder()
        .address("127.0.0.1:4002")
        .client_id(client_id.id())
        .connect_with_notice_stream()
        .await
        .expect("connection failed");

    let mut codes: Vec<i32> = Vec::new();
    let collect = async {
        while let Some(n) = notices.next().await {
            codes.push(n.code);
        }
    };
    let _ = tokio::time::timeout(Duration::from_millis(250), collect).await;

    assert!(
        codes.iter().any(|c| matches!(*c, 2104 | 2106 | 2107 | 2108 | 2158)),
        "expected at least one farm-status notice, got codes: {codes:?}",
    );
}

/// Reconnect-coverage live verification. Marked `#[ignore]` because we can't
/// reliably automate a gateway flap from inside the test suite. To run:
///
/// 1. Start the test (it connects + waits).
/// 2. While it's waiting, restart the gateway (or briefly close the API socket
///    via Gateway → Configure → Settings → API → reset connections).
/// 3. The test should print captured notices from both the initial and
///    post-reconnect handshakes, then exit.
#[tokio::test]
#[ignore]
async fn builder_notice_stream_survives_reconnect() {
    let client_id = ClientId::get();
    let captured: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();

    rate_limit();
    let (_client, mut notices) = Client::builder()
        .address("127.0.0.1:4002")
        .client_id(client_id.id())
        .connect_with_notice_stream()
        .await
        .expect("connection failed");

    // Drain on a background task — broadcaster lives on Connection, so the
    // stream survives gateway flaps.
    tokio::spawn(async move {
        while let Some(n) = notices.next().await {
            eprintln!("[notice] code={} {}", n.code, n.message);
            captured_clone.lock().unwrap().push(n.code);
        }
    });

    eprintln!("connected; flap the gateway within 60s to trigger reconnect");
    tokio::time::sleep(Duration::from_secs(60)).await;

    println!(
        "captured {} notices total: {:?}",
        captured.lock().unwrap().len(),
        captured.lock().unwrap()
    );
}

/// Issue #871 (async side): `disconnect()` ends a live `order_update_stream`.
#[tokio::test]
async fn order_update_stream_ends_on_disconnect() {
    use futures::StreamExt;

    let client_id = ClientId::get();

    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).await.expect("connection failed");
    let mut updates = client.order_update_stream().await.expect("order_update_stream failed");

    client.disconnect().await;

    let drained = tokio::time::timeout(Duration::from_secs(5), async { while updates.next().await.is_some() {} }).await;
    assert!(drained.is_ok(), "order_update_stream did not end within 5s of disconnect");
}

/// Issue #871: after `disconnect()`, `order_update_stream()` is refused
/// instead of returning a stream nothing will end.
#[tokio::test]
async fn order_update_stream_after_disconnect_fails() {
    let client_id = ClientId::get();

    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).await.expect("connection failed");
    client.disconnect().await;

    match client.order_update_stream().await {
        Err(Error::Shutdown) => {}
        other => panic!("expected Err(Shutdown), got: {:?}", other.map(|_| "stream")),
    }
}

/// A live `notice_stream` ends on `disconnect()`, as it does on the blocking
/// client. A reader must not wait forever on a client that is gone.
#[tokio::test]
async fn notice_stream_ends_on_disconnect() {
    let client_id = ClientId::get();

    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).await.expect("connection failed");
    let mut notices = client.notice_stream().expect("notice_stream failed");

    client.disconnect().await;

    let drained = tokio::time::timeout(Duration::from_secs(5), async { while notices.next().await.is_some() {} }).await;
    assert!(drained.is_ok(), "notice_stream did not end within 5s of disconnect");
}

/// Issue #871: a notice stream opened after `disconnect()` is already ended,
/// like the streams the disconnect closed.
#[tokio::test]
async fn notice_stream_after_disconnect_is_ended() {
    let client_id = ClientId::get();

    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).await.expect("connection failed");
    client.disconnect().await;

    let mut notices = client.notice_stream().expect("notice_stream failed");
    match tokio::time::timeout(Duration::from_secs(5), notices.next()).await {
        Ok(None) => {}
        Ok(Some(n)) => panic!("unexpected notice after disconnect: {n:?}"),
        Err(_) => panic!("notice stream did not end within 5s"),
    }
}

/// Issue #871: dropping the client ends a live `order_update_stream`. Drop
/// only flags shutdown; the dispatcher must finish it on exit.
#[tokio::test]
async fn order_update_stream_ends_on_client_drop() {
    use futures::StreamExt;

    let client_id = ClientId::get();

    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).await.expect("connection failed");
    let mut updates = client.order_update_stream().await.expect("order_update_stream failed");

    drop(client);

    let drained = tokio::time::timeout(Duration::from_secs(5), async { while updates.next().await.is_some() {} }).await;
    assert!(drained.is_ok(), "order_update_stream did not end within 5s of client drop");
}
