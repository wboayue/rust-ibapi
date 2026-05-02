//! Sync connection tests: handshake / connect / disconnect.
//!
//! Migrated from `client/sync/tests.rs` (PR 4 of `eliminate-mock-gateway.md`).
//! Drives the real handshake against a `MemoryStream` rather than a TCP gateway,
//! so there's no socket / port allocation / sleep involved.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;
use crate::client::sync::Client;
use crate::messages::IncomingMessages;
use crate::server_versions;
use crate::transport::sync::{MemoryStream, TcpMessageBus};

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

/// `IncomingMessages::Shutdown` (`-2`) is the clean-shutdown sentinel TWS
/// sends when it wants the client to stop reading. The dispatcher detects
/// it via `is_shutdown()` and exits without touching the reconnect path.
fn shutdown_frame() -> Vec<u8> {
    binary_text(IncomingMessages::Shutdown as i32, "1\0")
}

/// Handshake smoke: with scripted version + account-info responses,
/// `establish_connection` populates `server_version`, `time_zone`, and the
/// next-order-id / managed-accounts fields on `ConnectionMetadata`.
#[test]
fn establish_connection_populates_metadata() {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), CLIENT_ID);
    push_handshake(&stream);

    connection.establish_connection(None).expect("establish_connection failed");

    assert_eq!(connection.client_id, CLIENT_ID);
    assert_eq!(connection.server_version(), SERVER_VERSION);

    let metadata = connection.connection_metadata();
    assert_eq!(metadata.next_order_id, 90);
    assert_eq!(metadata.managed_accounts, "DU1234567");
    assert!(metadata.time_zone.is_some(), "time_zone should be set after handshake");
}

/// `client.disconnect()` joins the dispatcher thread and flips
/// `is_connected()` to false. Pushing the `-2` shutdown sentinel mirrors
/// what TWS sends on a clean disconnect — the dispatcher reads it,
/// calls `request_shutdown()`, and exits cleanly.
#[test]
fn disconnect_completes() {
    let (client, stream) = make_client();

    stream.push_inbound(shutdown_frame());
    let start = Instant::now();
    client.disconnect();

    assert!(start.elapsed() < Duration::from_secs(2), "disconnect did not complete in time");
    assert!(!client.is_connected());
}

/// Repeated `disconnect()` calls are safe: the first joins worker threads,
/// subsequent calls are no-ops because `request_shutdown` is idempotent.
#[test]
fn disconnect_is_idempotent() {
    let (client, stream) = make_client();

    stream.push_inbound(shutdown_frame());
    let start = Instant::now();
    client.disconnect();
    client.disconnect();

    assert!(start.elapsed() < Duration::from_secs(2), "repeated disconnect did not complete in time");
    assert!(!client.is_connected());
}

/// Build a `Client` over `MemoryStream`: handshake responses are pre-pushed,
/// the dispatcher / cleanup threads are running, and the stream handle is
/// returned so callers can push further frames (e.g. shutdown).
fn make_client() -> (Client, MemoryStream) {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), CLIENT_ID);
    push_handshake(&stream);
    connection.establish_connection(None).expect("establish_connection failed");
    let server_version = connection.server_version();

    let bus = Arc::new(TcpMessageBus::new(connection).expect("TcpMessageBus::new"));
    bus.process_messages(server_version, Duration::from_secs(0)).expect("process_messages");

    let client = Client::stubbed(bus, server_version);
    (client, stream)
}
