use std::sync::{Arc, Mutex};
use std::time::Duration;

use ibapi::messages::Notice;
use ibapi::{Client, ConnectionOptions, StartupMessage, StartupMessageCallback};
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
async fn connect_with_callback() {
    let client_id = ClientId::get();
    let count = Arc::new(Mutex::new(0_usize));
    let count_clone = count.clone();

    let callback: StartupMessageCallback = Box::new(move |msg| {
        match msg {
            StartupMessage::OpenOrder(o) => {
                assert!(o.order_id >= 0);
            }
            StartupMessage::OrderStatus(_) | StartupMessage::OpenOrderEnd | StartupMessage::AccountUpdate(_) | StartupMessage::Other(_) => {}
        }
        *count_clone.lock().unwrap() += 1;
    });

    rate_limit();
    let client = Client::connect_with_callback("127.0.0.1:4002", client_id.id(), Some(callback))
        .await
        .expect("connection failed");

    assert!(client.server_version() > 0);
    println!("startup callback fired {} times", *count.lock().unwrap());
}

#[tokio::test]
async fn connect_with_options_callback() {
    let client_id = ClientId::get();
    let count = Arc::new(Mutex::new(0_usize));
    let count_clone = count.clone();

    let options = ConnectionOptions::default()
        .tcp_no_delay(true)
        .startup_callback(move |_msg: StartupMessage| {
            *count_clone.lock().unwrap() += 1;
        });

    rate_limit();
    let client = Client::connect_with_options("127.0.0.1:4002", client_id.id(), options)
        .await
        .expect("connection failed");

    assert!(client.server_version() > 0);
    println!("startup callback fired {} times", *count.lock().unwrap());
}

/// Canonical live test: the paper gateway always emits at least one farm-status
/// notice (2104 / 2106 / 2107 / 2108 / 2158) during the handshake.
#[tokio::test]
async fn startup_notice_callback_receives_handshake_notices() {
    let client_id = ClientId::get();
    let captured: Arc<Mutex<Vec<Notice>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();

    let options = ConnectionOptions::default().startup_notice_callback(move |notice: Notice| {
        captured_clone.lock().unwrap().push(notice);
    });

    rate_limit();
    let _client = Client::connect_with_options("127.0.0.1:4002", client_id.id(), options)
        .await
        .expect("connection failed");

    let notices = captured.lock().unwrap();
    let codes: Vec<i32> = notices.iter().map(|n| n.code).collect();
    assert!(
        notices.iter().any(|n| matches!(n.code, 2104 | 2106 | 2107 | 2108 | 2158)),
        "expected at least one farm-status notice, got codes: {codes:?}",
    );
}

/// Reconnect-coverage live verification. Marked `#[ignore]` because we can't
/// reliably automate a gateway flap. Run manually: start the test, then flap
/// the gateway connection within 60s to trigger reconnect.
#[tokio::test]
#[ignore]
async fn startup_notice_callback_fires_on_reconnect() {
    let client_id = ClientId::get();
    let captured: Arc<Mutex<Vec<Notice>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();

    let options = ConnectionOptions::default().startup_notice_callback(move |notice: Notice| {
        eprintln!("[notice] code={} {}", notice.code, notice.message);
        captured_clone.lock().unwrap().push(notice);
    });

    rate_limit();
    let _client = Client::connect_with_options("127.0.0.1:4002", client_id.id(), options)
        .await
        .expect("connection failed");
    eprintln!("connected; flap the gateway within 60s to trigger reconnect");
    tokio::time::sleep(Duration::from_secs(60)).await;

    let notices = captured.lock().unwrap();
    println!("captured {} notices total", notices.len());
}
