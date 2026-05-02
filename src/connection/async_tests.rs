//! Async connection tests: handshake / connect / disconnect.
//!
//! Migrated from `client/async/tests.rs` (PR 4 of `eliminate-mock-gateway.md`).
//! Drives the real handshake against a `MemoryStream` rather than a TCP gateway.

use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::client::r#async::Client;
use crate::server_versions;
use crate::transport::r#async::{AsyncTcpMessageBus, MemoryStream};

const CLIENT_ID: i32 = 100;
const SERVER_VERSION: i32 = server_versions::PROTOBUF;

/// Push the three response frames that satisfy `establish_connection`:
/// handshake ack (raw text), then NextValidId and ManagedAccounts in
/// binary-text form (4-byte BE msg_id + text payload — what TWS sends once
/// `server_version >= PROTOBUF`).
fn push_handshake(stream: &MemoryStream) {
    let handshake = format!("{}\020240120 12:00:00 EST\0", SERVER_VERSION);
    stream.push_inbound(handshake.into_bytes());
    stream.push_inbound(binary_text(9, "1\090\0")); // NextValidId
    stream.push_inbound(binary_text(15, "1\0DU1234567\0")); // ManagedAccounts
}

fn binary_text(msg_id: i32, payload: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + payload.len());
    data.extend_from_slice(&msg_id.to_be_bytes());
    data.extend_from_slice(payload.as_bytes());
    data
}

/// Handshake smoke: with scripted version + account-info responses,
/// `establish_connection` populates `server_version`, `time_zone`, and the
/// next-order-id / managed-accounts fields on `ConnectionMetadata`.
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
    assert!(metadata.time_zone.is_some(), "time_zone should be set after handshake");
}

/// `client.disconnect().await` joins the dispatch task and flips
/// `is_connected()` to false. Unlike the sync case, the async dispatcher
/// uses `tokio::select!` against a shutdown notification, so no
/// `stream.close()` is required to unblock the reader.
#[tokio::test]
async fn disconnect_completes() {
    let client = make_client().await;

    tokio::time::timeout(Duration::from_secs(2), client.disconnect())
        .await
        .expect("disconnect did not complete in time");

    assert!(!client.is_connected());
}

/// Repeated `disconnect().await` calls are safe: the first joins the
/// processing task, subsequent calls are no-ops because `request_shutdown`
/// is idempotent.
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

/// Build a `Client` over `MemoryStream`: handshake responses are pre-pushed
/// and the dispatcher task is running.
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
