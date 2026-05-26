use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use time_tz::timezones;

use super::*;
use crate::client::sync::Client;
use crate::common::test_utils::helpers::{error_frame, managed_accounts_frame, next_valid_id_frame};
use crate::messages::IncomingMessages;
use crate::server_versions;
use crate::transport::sync::{MemoryStream, TcpMessageBus};

const CLIENT_ID: i32 = 100;
const SERVER_VERSION: i32 = server_versions::PROTOBUF_REST_MESSAGES_3;

fn push_handshake(stream: &MemoryStream) {
    let handshake = format!("{}\020240120 12:00:00 EST\0", SERVER_VERSION);
    stream.push_inbound(handshake.into_bytes());
    stream.push_inbound(next_valid_id_frame(90));
    stream.push_inbound(managed_accounts_frame("DU1234567"));
}

fn binary_text(msg_id: i32, payload: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + payload.len());
    data.extend_from_slice(&msg_id.to_be_bytes());
    data.extend_from_slice(payload.as_bytes());
    data
}

/// `-2` is the clean-shutdown sentinel TWS sends when it wants the client
/// to stop reading. The dispatcher detects it via `is_shutdown()` and exits
/// without touching the reconnect path.
fn shutdown_frame() -> Vec<u8> {
    binary_text(IncomingMessages::Shutdown as i32, "1\0")
}

#[test]
fn establish_connection_rejects_pre_protobuf_server() {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), CLIENT_ID);

    let too_old = server_versions::PROTOBUF_REST_MESSAGES_3 - 1;
    let handshake = format!("{}\020240120 12:00:00 EST\0", too_old);
    stream.push_inbound(handshake.into_bytes());

    let err = connection.establish_connection().expect_err("must reject old server");
    match err {
        crate::errors::Error::ServerVersion(required, got, ref msg) => {
            assert_eq!(required, server_versions::PROTOBUF_REST_MESSAGES_3);
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

#[test]
fn establish_connection_populates_metadata() {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), CLIENT_ID);
    push_handshake(&stream);

    connection.establish_connection().expect("establish_connection failed");

    assert_eq!(connection.client_id, CLIENT_ID);
    assert_eq!(connection.server_version(), SERVER_VERSION);

    let metadata = connection.connection_metadata();
    assert_eq!(metadata.next_order_id, 90);
    assert_eq!(metadata.managed_accounts, "DU1234567");
    assert_eq!(metadata.time_zone, Some(timezones::db::EST));
}

#[test]
fn disconnect_completes() {
    let (client, stream) = make_client();

    stream.push_inbound(shutdown_frame());
    let start = Instant::now();
    client.disconnect();

    assert!(start.elapsed() < Duration::from_secs(2), "disconnect did not complete in time");
    assert!(!client.is_connected());
}

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

fn make_client() -> (Client, MemoryStream) {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), CLIENT_ID);
    push_handshake(&stream);
    connection.establish_connection().expect("establish_connection failed");
    let server_version = connection.server_version();

    let bus = Arc::new(TcpMessageBus::new(connection).expect("TcpMessageBus::new"));
    bus.process_messages(server_version).expect("process_messages");

    let client = Client::stubbed(bus, server_version);
    (client, stream)
}

/// Drive `establish_connection` twice through the same `Connection<S>` with a
/// startup callback attached and a `NoticeStream` subscribed pre-handshake,
/// simulating an initial connect followed by the post-flap reconnect handshake.
/// Both handshakes should re-fire the startup callback AND deliver any 21xx
/// farm-status notices to the same stream (the broadcaster is reused across
/// reconnects because it lives on `Connection`, not on the bus).
#[test]
fn handshake_callbacks_and_notice_stream_survive_reconnect() {
    let stream = MemoryStream::default();
    let mut connection = Connection::stubbed(stream.clone(), CLIENT_ID);

    let startup_count = Arc::new(Mutex::new(0_usize));
    let startup_count_clone = startup_count.clone();

    connection.startup_callback = Some(Arc::new(move |_msg: crate::connection::common::StartupMessage| {
        *startup_count_clone.lock().unwrap() += 1;
    }));

    // Subscribe to the connection's broadcaster BEFORE the handshake — same
    // shape as ClientBuilder::connect_with_notice_stream's pre-bind.
    let notice_rx = connection.notice_broadcaster.subscribe();

    // First handshake: handshake bytes + OpenOrderEnd marker + farm-status notice + NextValidId + ManagedAccounts.
    // OpenOrderEnd is a unit marker (no payload to decode), so the typed
    // callback fires regardless of wire framing.
    let handshake_bytes = format!("{}\020240120 12:00:00 EST\0", SERVER_VERSION).into_bytes();
    stream.push_inbound(handshake_bytes.clone());
    stream.push_inbound(binary_text(IncomingMessages::OpenOrderEnd as i32, "1\0"));
    stream.push_inbound(error_frame(-1, 2104, "farm OK"));
    stream.push_inbound(next_valid_id_frame(90));
    stream.push_inbound(managed_accounts_frame("DU1234567"));

    connection.establish_connection().expect("first establish_connection failed");
    assert_eq!(*startup_count.lock().unwrap(), 1, "startup callback should fire on first handshake");
    let n1 = notice_rx.try_recv().expect("first farm-status notice should be on the stream");
    assert_eq!(n1.code, 2104);

    // Second handshake (simulating post-reconnect): same shape.
    stream.push_inbound(handshake_bytes);
    stream.push_inbound(binary_text(IncomingMessages::OpenOrderEnd as i32, "1\0"));
    stream.push_inbound(error_frame(-1, 2106, "HMDS farm OK"));
    stream.push_inbound(next_valid_id_frame(91));
    stream.push_inbound(managed_accounts_frame("DU1234567"));

    connection.establish_connection().expect("second establish_connection failed");
    assert_eq!(*startup_count.lock().unwrap(), 2, "startup callback should fire on reconnect handshake");
    let n2 = notice_rx.try_recv().expect("second farm-status notice should be on the same stream");
    assert_eq!(n2.code, 2106);
}

/// During a reconnect, any caller of `connection_metadata()` must see cleared
/// state rather than the prior session's `server_version` / `next_order_id` /
/// `managed_accounts`. Sync mirror of the async test.
#[test]
fn reconnect_clears_metadata_while_waiting_for_handshake() {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), CLIENT_ID);

    push_handshake(&stream);
    connection.establish_connection().expect("initial establish_connection failed");

    let metadata = connection.connection_metadata();
    assert_eq!(metadata.server_version, SERVER_VERSION);
    assert_eq!(metadata.next_order_id, 90);
    assert_eq!(metadata.managed_accounts, "DU1234567");

    let initial_capture_len = stream.captured().len();

    // Spawn reconnect on a thread with no handshake responses queued: it will
    // write the new handshake magic and block on the first read.
    let connection = Arc::new(connection);
    let conn_for_thread = Arc::clone(&connection);
    let reconnect_thread = thread::spawn(move || conn_for_thread.reconnect());

    let deadline = Instant::now() + Duration::from_secs(2);
    while stream.captured().len() == initial_capture_len {
        assert!(Instant::now() < deadline, "reconnect must reach handshake-write phase");
        thread::sleep(Duration::from_millis(5));
    }

    let metadata = connection.connection_metadata();
    assert_eq!(metadata.client_id, CLIENT_ID);
    assert_eq!(metadata.server_version, 0);
    assert_eq!(metadata.next_order_id, 0);
    assert_eq!(metadata.managed_accounts, "");
    assert!(metadata.connection_time.is_none());
    assert!(metadata.time_zone.is_none());

    push_handshake(&stream);

    reconnect_thread.join().expect("reconnect thread panicked").expect("reconnect failed");

    let metadata = connection.connection_metadata();
    assert_eq!(metadata.server_version, SERVER_VERSION);
    assert_eq!(metadata.next_order_id, 90);
    assert_eq!(metadata.managed_accounts, "DU1234567");
    assert_eq!(metadata.time_zone, Some(timezones::db::EST));
}

/// A closed stream surfaces `Io(UnexpectedEof)` from `read_message`, which
/// `handshake` must translate to `Error::ConnectionRejected` — the
/// user-visible signal for a host allow-list mismatch.
#[test]
fn handshake_unexpected_eof_returns_connection_rejected() {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), CLIENT_ID);

    // EOF before any handshake response: read_message → UnexpectedEof.
    stream.close();

    let err = connection.handshake().expect_err("must surface rejection error");
    match err {
        crate::errors::Error::ConnectionRejected(ref msg) => {
            assert!(msg.contains("server may be rejecting"), "unexpected message: {msg}");
        }
        other => panic!("expected Error::ConnectionRejected, got {other:?}"),
    }
}
