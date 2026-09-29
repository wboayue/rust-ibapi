use std::sync::{Arc, Mutex};
use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::{Error, StartupMessage};
use ibapi_test::{rate_limit, ClientId};

#[test]
fn connect_to_gateway() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).expect("connection failed");

    assert!(client.server_version() > 0);
    assert!(client.connection_time().is_some());

    rate_limit();
    let time = client.server_time().expect("failed to get server time");
    assert!(time.year() >= 2025);
}

#[test]
fn builder_startup_callback_fires_during_handshake() {
    let client_id = ClientId::get();
    let count = Arc::new(Mutex::new(0_usize));
    let count_clone = count.clone();

    rate_limit();
    let client = Client::builder()
        .address("127.0.0.1:4002")
        .client_id(client_id.id())
        .startup_callback(move |msg| {
            // Sanity-check the typed payload — should match one of the known variants.
            if let StartupMessage::OpenOrder(ref o) = msg {
                assert!(o.order_id >= 0);
            }
            *count_clone.lock().unwrap() += 1;
        })
        .connect()
        .expect("connection failed");

    assert!(client.server_version() > 0);
    println!("startup callback fired {} times", *count.lock().unwrap());
}

#[test]
fn builder_tcp_no_delay_round_trips() {
    let client_id = ClientId::get();

    rate_limit();
    let client = Client::builder()
        .address("127.0.0.1:4002")
        .client_id(client_id.id())
        .tcp_no_delay(true)
        .connect()
        .expect("connection failed");

    assert!(client.server_version() > 0);
}

/// Canonical live test: the paper gateway always emits at least one farm-status
/// notice (2104 / 2106 / 2107 / 2108 / 2158) during the handshake. The pre-bound
/// `NoticeStream` from `connect_with_notice_stream()` captures them.
#[test]
fn builder_notice_stream_receives_handshake_notices() {
    let client_id = ClientId::get();

    rate_limit();
    let (_client, notices) = Client::builder()
        .address("127.0.0.1:4002")
        .client_id(client_id.id())
        .connect_with_notice_stream()
        .expect("connection failed");

    // Collect every notice that arrives within a short window — handshake
    // notices land within milliseconds, so 250ms is generous.
    let deadline = std::time::Instant::now() + Duration::from_millis(250);
    let mut codes = Vec::new();
    while let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) {
        match notices.next_timeout(remaining) {
            Some(n) => codes.push(n.code),
            None => break,
        }
    }

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
#[test]
#[ignore]
fn builder_notice_stream_survives_reconnect() {
    let client_id = ClientId::get();
    let captured: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();

    rate_limit();
    let (_client, notices) = Client::builder()
        .address("127.0.0.1:4002")
        .client_id(client_id.id())
        .connect_with_notice_stream()
        .expect("connection failed");

    // Drain on a worker thread — broadcaster lives on Connection, so the
    // stream survives gateway flaps.
    std::thread::spawn(move || {
        for n in notices.iter() {
            eprintln!("[notice] code={} {}", n.code, n.message);
            captured_clone.lock().unwrap().push(n.code);
        }
    });

    eprintln!("connected; flap the gateway within 60s to trigger reconnect");
    std::thread::sleep(Duration::from_secs(60));

    println!(
        "captured {} notices total: {:?}",
        captured.lock().unwrap().len(),
        captured.lock().unwrap()
    );
}

/// Issue #871: `disconnect()` must end a blocked `order_update_stream` reader.
/// The reader thread reports every item it sees and then the end of the
/// stream; the test fails (rather than hangs) if the end never arrives.
#[test]
fn order_update_stream_ends_on_disconnect() {
    let client_id = ClientId::get();

    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).expect("connection failed");
    let updates = client.order_update_stream().expect("order_update_stream failed");

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        while let Some(item) = updates.next() {
            let _ = tx.send(Some(format!("{item:?}")));
        }
        let _ = tx.send(None);
    });

    client.disconnect();

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        match rx.recv_timeout(remaining) {
            Ok(Some(item)) => seen.push(item),
            Ok(None) => break,
            Err(_) => panic!("order_update_stream did not end within 5s of disconnect; items seen: {seen:?}"),
        }
    }

    assert!(
        seen.last().is_some_and(|item| item.contains("Shutdown")),
        "expected the stream to end with Err(Shutdown), items seen: {seen:?}"
    );
}

/// Issue #871: after `disconnect()`, `order_update_stream()` is refused
/// instead of returning a stream nothing will end.
#[test]
fn order_update_stream_after_disconnect_fails() {
    let client_id = ClientId::get();

    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).expect("connection failed");
    client.disconnect();

    match client.order_update_stream() {
        Err(Error::Shutdown) => {}
        other => panic!("expected Err(Shutdown), got: {:?}", other.map(|_| "stream")),
    }
}

/// Issue #871: a notice stream opened after `disconnect()` is already ended,
/// like the streams the disconnect closed.
#[test]
fn notice_stream_after_disconnect_is_ended() {
    let client_id = ClientId::get();

    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).expect("connection failed");
    client.disconnect();

    let notices = client.notice_stream().expect("notice_stream failed");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(notices.next().map(|n| n.code));
    });
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(None) => {}
        Ok(Some(code)) => panic!("unexpected notice after disconnect: {code}"),
        Err(_) => panic!("notice stream did not end within 5s"),
    }
}
