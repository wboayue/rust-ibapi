use std::sync::{Arc, Mutex};
use std::time::Duration;

use time_tz::timezones;

use super::*;
use crate::client::r#async::Client;
use crate::messages::{IncomingMessages, Notice};
use crate::server_versions;
use crate::transport::r#async::{AsyncTcpMessageBus, MemoryStream};

const CLIENT_ID: i32 = 100;
const SERVER_VERSION: i32 = server_versions::PROTOBUF;

fn push_handshake(stream: &MemoryStream) {
    let handshake = format!("{}\020240120 12:00:00 EST\0", SERVER_VERSION);
    stream.push_inbound(handshake.into_bytes());
    stream.push_inbound(binary_text(IncomingMessages::NextValidId as i32, "1\090\0"));
    stream.push_inbound(binary_text(IncomingMessages::ManagedAccounts as i32, "1\0DU1234567\0"));
}

fn binary_text(msg_id: i32, payload: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + payload.len());
    data.extend_from_slice(&msg_id.to_be_bytes());
    data.extend_from_slice(payload.as_bytes());
    data
}

#[tokio::test]
async fn establish_connection_rejects_pre_protobuf_server() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), CLIENT_ID);

    let too_old = server_versions::PROTOBUF - 1;
    let handshake = format!("{}\020240120 12:00:00 EST\0", too_old);
    stream.push_inbound(handshake.into_bytes());

    let err = connection.establish_connection().await.expect_err("must reject old server");
    match err {
        crate::errors::Error::ServerVersion(required, got, ref msg) => {
            assert_eq!(required, server_versions::PROTOBUF);
            assert_eq!(got, too_old);
            assert!(msg.contains("protobuf"), "message should mention protobuf: {msg}");
        }
        other => panic!("expected Error::ServerVersion, got {other:?}"),
    }

    // We must not have sent the StartApi request: only the handshake bytes reach the wire.
    let captured = stream.captured();
    let expected = connection.connection_handler.format_handshake();
    assert_eq!(captured, expected, "no bytes should follow the handshake when version check fails");
}

#[tokio::test]
async fn establish_connection_populates_metadata() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), CLIENT_ID);
    push_handshake(&stream);

    connection.establish_connection().await.expect("establish_connection failed");

    assert_eq!(connection.client_id, CLIENT_ID);
    assert_eq!(connection.server_version(), SERVER_VERSION);

    let metadata = connection.connection_metadata().await;
    assert_eq!(metadata.next_order_id, 90);
    assert_eq!(metadata.managed_accounts, "DU1234567");
    assert_eq!(metadata.time_zone, Some(timezones::db::EST));
}

#[tokio::test]
async fn disconnect_completes() {
    let client = make_client().await;

    tokio::time::timeout(Duration::from_secs(2), client.disconnect())
        .await
        .expect("disconnect did not complete in time");

    assert!(!client.is_connected());
}

#[tokio::test]
async fn disconnect_is_idempotent() {
    let client = make_client().await;

    tokio::time::timeout(Duration::from_secs(2), async {
        client.disconnect().await;
        client.disconnect().await;
    })
    .await
    .expect("repeated disconnect did not complete in time");

    assert!(!client.is_connected());
}

async fn make_client() -> Client {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), CLIENT_ID);
    push_handshake(&stream);
    connection.establish_connection().await.expect("establish_connection failed");
    let server_version = connection.server_version();

    let bus = Arc::new(AsyncTcpMessageBus::new(connection).expect("AsyncTcpMessageBus::new"));
    bus.clone()
        .process_messages(server_version, Duration::from_secs(0))
        .expect("process_messages");

    Client::stubbed(bus, server_version)
}

/// Async mirror of `callbacks_fire_on_reconnect_handshake` — drive
/// `establish_connection` twice and assert both callbacks fire each time.
#[tokio::test]
async fn callbacks_fire_on_reconnect_handshake() {
    let stream = MemoryStream::default();
    let mut connection = AsyncConnection::stubbed(stream.clone(), CLIENT_ID);

    let startup_count = Arc::new(Mutex::new(0_usize));
    let startup_count_clone = startup_count.clone();
    let notice_count = Arc::new(Mutex::new(0_usize));
    let notice_count_clone = notice_count.clone();

    connection.startup_callback = Some(Arc::new(move |_msg: crate::connection::common::StartupMessage| {
        *startup_count_clone.lock().unwrap() += 1;
    }));
    connection.notice_callback = Some(Arc::new(move |_notice: Notice| {
        *notice_count_clone.lock().unwrap() += 1;
    }));

    let handshake_bytes = format!("{}\020240120 12:00:00 EST\0", SERVER_VERSION).into_bytes();
    stream.push_inbound(handshake_bytes.clone());
    stream.push_inbound(binary_text(IncomingMessages::OpenOrder as i32, "5\0123\0AAPL\0\0"));
    stream.push_inbound(binary_text(IncomingMessages::Error as i32, "2\0-1\02104\0farm OK\0"));
    stream.push_inbound(binary_text(IncomingMessages::NextValidId as i32, "1\090\0"));
    stream.push_inbound(binary_text(IncomingMessages::ManagedAccounts as i32, "1\0DU1234567\0"));

    connection.establish_connection().await.expect("first establish_connection failed");
    assert_eq!(*startup_count.lock().unwrap(), 1, "startup callback should fire on first handshake");
    assert_eq!(*notice_count.lock().unwrap(), 1, "notice callback should fire on first handshake");

    stream.push_inbound(handshake_bytes);
    stream.push_inbound(binary_text(IncomingMessages::OpenOrder as i32, "5\0456\0MSFT\0\0"));
    stream.push_inbound(binary_text(IncomingMessages::Error as i32, "2\0-1\02106\0HMDS farm OK\0"));
    stream.push_inbound(binary_text(IncomingMessages::NextValidId as i32, "1\091\0"));
    stream.push_inbound(binary_text(IncomingMessages::ManagedAccounts as i32, "1\0DU1234567\0"));

    connection.establish_connection().await.expect("second establish_connection failed");
    assert_eq!(*startup_count.lock().unwrap(), 2, "startup callback should fire on reconnect handshake");
    assert_eq!(*notice_count.lock().unwrap(), 2, "notice callback should fire on reconnect handshake");
}
