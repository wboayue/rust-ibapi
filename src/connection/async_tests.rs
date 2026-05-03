use std::sync::Arc;
use std::time::Duration;

use time_tz::timezones;

use super::*;
use crate::client::r#async::Client;
use crate::messages::IncomingMessages;
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

    let err = connection.establish_connection(None).await.expect_err("must reject old server");
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

    connection.establish_connection(None).await.expect("establish_connection failed");

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
    connection.establish_connection(None).await.expect("establish_connection failed");
    let server_version = connection.server_version();

    let bus = Arc::new(AsyncTcpMessageBus::new(connection).expect("AsyncTcpMessageBus::new"));
    bus.clone()
        .process_messages(server_version, Duration::from_secs(0))
        .expect("process_messages");

    Client::stubbed(bus, server_version)
}
