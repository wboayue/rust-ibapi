use std::sync::{Arc, Mutex};
use std::time::Duration;

use time_tz::timezones;

use super::*;
use crate::client::r#async::Client;
use crate::messages::IncomingMessages;
use crate::server_versions;
use crate::transport::common::MAX_RECONNECT_ATTEMPTS;
use crate::transport::r#async::{AsyncTcpMessageBus, MemoryStream};

const CLIENT_ID: i32 = 100;
const SERVER_VERSION: i32 = server_versions::PROTOBUF_SCAN_DATA;

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

    let too_old = server_versions::PROTOBUF_SCAN_DATA - 1;
    let handshake = format!("{}\020240120 12:00:00 EST\0", too_old);
    stream.push_inbound(handshake.into_bytes());

    let err = connection.establish_connection().await.expect_err("must reject old server");
    match err {
        crate::errors::Error::ServerVersion(required, got, ref msg) => {
            assert_eq!(required, server_versions::PROTOBUF_SCAN_DATA);
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

/// Async mirror of `handshake_callbacks_and_notice_stream_survive_reconnect`
/// (sync) — drive `establish_connection` twice and assert the startup callback
/// fires both times AND any 21xx farm-status notices reach a `broadcast::Receiver`
/// subscribed pre-handshake. The broadcaster lives on `AsyncConnection`, so
/// the same receiver survives reconnects.
#[tokio::test]
async fn handshake_callbacks_and_notice_stream_survive_reconnect() {
    let stream = MemoryStream::default();
    let mut connection = AsyncConnection::stubbed(stream.clone(), CLIENT_ID);

    let startup_count = Arc::new(Mutex::new(0_usize));
    let startup_count_clone = startup_count.clone();

    connection.startup_callback = Some(Arc::new(move |_msg: crate::connection::common::StartupMessage| {
        *startup_count_clone.lock().unwrap() += 1;
    }));

    // Subscribe to the per-connection broadcaster BEFORE the handshake — same
    // shape as ClientBuilder::connect_with_notice_stream's pre-bind.
    let mut notice_rx = connection.notice_sender.subscribe();

    let handshake_bytes = format!("{}\020240120 12:00:00 EST\0", SERVER_VERSION).into_bytes();
    stream.push_inbound(handshake_bytes.clone());
    stream.push_inbound(binary_text(IncomingMessages::OpenOrder as i32, "5\0123\0AAPL\0\0"));
    stream.push_inbound(binary_text(IncomingMessages::Error as i32, "-1\02104\0farm OK\0"));
    stream.push_inbound(binary_text(IncomingMessages::NextValidId as i32, "1\090\0"));
    stream.push_inbound(binary_text(IncomingMessages::ManagedAccounts as i32, "1\0DU1234567\0"));

    connection.establish_connection().await.expect("first establish_connection failed");
    assert_eq!(*startup_count.lock().unwrap(), 1, "startup callback should fire on first handshake");
    let n1 = notice_rx.try_recv().expect("first farm-status notice should be on the stream");
    assert_eq!(n1.code, 2104);

    stream.push_inbound(handshake_bytes);
    stream.push_inbound(binary_text(IncomingMessages::OpenOrder as i32, "5\0456\0MSFT\0\0"));
    stream.push_inbound(binary_text(IncomingMessages::Error as i32, "-1\02106\0HMDS farm OK\0"));
    stream.push_inbound(binary_text(IncomingMessages::NextValidId as i32, "1\091\0"));
    stream.push_inbound(binary_text(IncomingMessages::ManagedAccounts as i32, "1\0DU1234567\0"));

    connection.establish_connection().await.expect("second establish_connection failed");
    assert_eq!(*startup_count.lock().unwrap(), 2, "startup callback should fire on reconnect handshake");
    let n2 = notice_rx.try_recv().expect("second farm-status notice should be on the same stream");
    assert_eq!(n2.code, 2106);
}

/// Debug impl is wired up — print and check the client id is in the output.
#[test]
fn debug_impl_formats_connection() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream, CLIENT_ID);
    let rendered = format!("{connection:?}");
    assert!(rendered.contains("AsyncConnection"), "{rendered}");
    assert!(rendered.contains(&CLIENT_ID.to_string()), "{rendered}");
}

/// A closed stream surfaces `Io(UnexpectedEof)` from `read_message`, which
/// `handshake` must translate to `Error::Simple` with the "server may be
/// rejecting connections" hint — the user-visible signal for a host
/// allow-list mismatch.
#[tokio::test]
async fn handshake_unexpected_eof_returns_rejection_simple_error() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), CLIENT_ID);

    // EOF before any handshake response: read_message → UnexpectedEof.
    stream.close();

    let err = connection.handshake().await.expect_err("must surface rejection error");
    match err {
        crate::errors::Error::Simple(ref msg) => {
            assert!(msg.contains("server may be rejecting"), "unexpected message: {msg}");
        }
        other => panic!("expected Error::Simple, got {other:?}"),
    }
}

/// Reconnect succeeds once the socket stops failing. The Fibonacci backoff
/// loop counts down `reconnect_failures` (3 here), then `establish_connection`
/// replays the handshake against the pre-queued inbound frames.
#[tokio::test]
async fn reconnect_succeeds_after_transient_failures() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), CLIENT_ID);

    // Initial connection.
    push_handshake(&stream);
    connection.establish_connection().await.expect("initial establish_connection failed");
    assert_eq!(connection.server_version(), SERVER_VERSION);

    // Fail 3 reconnect attempts, then succeed; queue a fresh handshake for the
    // post-reconnect establish_connection replay.
    stream.set_reconnect_failures(3);
    push_handshake(&stream);

    connection.reconnect().await.expect("reconnect must succeed after transient failures");
    // Handshake replay updates the server-version cache.
    assert_eq!(connection.server_version(), SERVER_VERSION);
}

/// When the socket refuses reconnects through every Fibonacci attempt, the
/// loop exits with `Error::ConnectionFailed`. Pre-arming with exactly
/// `MAX_RECONNECT_ATTEMPTS` failures binds the test to the loop's exit
/// condition (rather than a hardcoded count).
#[tokio::test]
async fn reconnect_returns_connection_failed_after_exhausting_attempts() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), CLIENT_ID);

    push_handshake(&stream);
    connection.establish_connection().await.expect("initial establish_connection failed");

    stream.set_reconnect_failures(MAX_RECONNECT_ATTEMPTS as usize);

    let err = connection.reconnect().await.expect_err("must give up after MAX_RECONNECT_ATTEMPTS");
    assert!(matches!(err, crate::errors::Error::ConnectionFailed), "got {err:?}");
}

/// During a reconnect, any caller of `connection_metadata()` must see cleared
/// state rather than the prior session's `server_version` / `next_order_id` /
/// `managed_accounts`. Without `reset_connection_metadata()` in the reconnect
/// path, stale values are observable until the new handshake completes.
#[tokio::test]
async fn reconnect_clears_metadata_while_waiting_for_handshake() {
    let stream = MemoryStream::default();
    let connection = AsyncConnection::stubbed(stream.clone(), CLIENT_ID);

    push_handshake(&stream);
    connection.establish_connection().await.expect("initial establish_connection failed");

    let metadata = connection.connection_metadata().await;
    assert_eq!(metadata.server_version, SERVER_VERSION);
    assert_eq!(metadata.next_order_id, 90);
    assert_eq!(metadata.managed_accounts, "DU1234567");

    let initial_capture_len = stream.captured().len();

    // Spawn reconnect with no handshake responses queued: the task will write
    // the new handshake magic and block on the first read.
    let connection = Arc::new(connection);
    let conn_for_task = Arc::clone(&connection);
    let reconnect_task = tokio::spawn(async move { conn_for_task.reconnect().await });

    // Wait until the reconnect's handshake bytes appear on the wire. By that
    // point `reset_connection_metadata()` has already run.
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if stream.captured().len() > initial_capture_len {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("reconnect must reach handshake-write phase");

    let metadata = connection.connection_metadata().await;
    assert_eq!(metadata.client_id, CLIENT_ID);
    assert_eq!(metadata.server_version, 0);
    assert_eq!(metadata.next_order_id, 0);
    assert_eq!(metadata.managed_accounts, "");
    assert!(metadata.connection_time.is_none());
    assert!(metadata.time_zone.is_none());

    // Release: feed the reconnect handshake responses.
    push_handshake(&stream);

    reconnect_task.await.expect("reconnect task panicked").expect("reconnect failed");

    let metadata = connection.connection_metadata().await;
    assert_eq!(metadata.server_version, SERVER_VERSION);
    assert_eq!(metadata.next_order_id, 90);
    assert_eq!(metadata.managed_accounts, "DU1234567");
    assert_eq!(metadata.time_zone, Some(timezones::db::EST));
}
