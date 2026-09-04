use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use time_tz::timezones;

use super::*;
use crate::client::sync::Client;
use crate::common::test_utils::helpers::{error_frame, managed_accounts_frame, next_valid_id_frame};
use crate::messages::IncomingMessages;
use crate::server_versions;
use crate::transport::sync::{Io, MemoryStream, Reconnect, ShutdownSignal, Stream, TcpMessageBus};
use crate::transport::MessageBus;

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
fn reconnect_retries_after_transient_handshake_failure() {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), CLIENT_ID);

    push_handshake(&stream);
    connection.establish_connection().expect("initial establish_connection failed");

    let too_old = server_versions::PROTOBUF_REST_MESSAGES_3 - 1;
    stream.push_inbound(format!("{}\020240120 12:00:00 EST\0", too_old).into_bytes());
    push_handshake(&stream);

    connection.reconnect().expect("reconnect must retry a failed handshake");

    assert_eq!(connection.server_version(), SERVER_VERSION);
    let metadata = connection.connection_metadata();
    assert_eq!(metadata.next_order_id, 90);
    assert_eq!(metadata.managed_accounts, "DU1234567");
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

/// Socket for the shutdown-during-reconnect tests. Reads and writes delegate
/// to a `MemoryStream` and the backoff wait goes through the production
/// `ShutdownSignal`, so the loop spends its time exactly where the shutdown
/// has to be observed. `reconnect` either fails immediately (TWS stays down)
/// or waits for the test to open the gate and then succeeds.
///
/// Cloning yields another handle to the same state, so the test keeps one
/// while the connection owns the other.
#[derive(Clone, Debug)]
struct TestSocket {
    stream: MemoryStream,
    state: Arc<SocketState>,
}

#[derive(Debug)]
struct SocketState {
    sleep_started: AtomicBool,
    reconnect_started: AtomicBool,
    /// `Some`: `reconnect` waits on the gate, then succeeds. `None`: it fails
    /// immediately.
    gate: Option<(Mutex<bool>, Condvar)>,
}

impl TestSocket {
    fn unreachable(stream: MemoryStream) -> Self {
        Self::new(stream, None)
    }

    fn gated(stream: MemoryStream) -> Self {
        Self::new(stream, Some((Mutex::new(false), Condvar::new())))
    }

    fn new(stream: MemoryStream, gate: Option<(Mutex<bool>, Condvar)>) -> Self {
        Self {
            stream,
            state: Arc::new(SocketState {
                sleep_started: AtomicBool::new(false),
                reconnect_started: AtomicBool::new(false),
                gate,
            }),
        }
    }

    /// Let the pending `reconnect` complete.
    fn release(&self) {
        let (open, opened) = self.state.gate.as_ref().expect("socket has no gate");
        *open.lock().unwrap() = true;
        opened.notify_all();
    }

    fn sleep_started(&self) -> bool {
        self.state.sleep_started.load(Ordering::SeqCst)
    }

    fn reconnect_started(&self) -> bool {
        self.state.reconnect_started.load(Ordering::SeqCst)
    }
}

impl Io for TestSocket {
    fn read_message(&self) -> Result<Vec<u8>, Error> {
        self.stream.read_message()
    }

    fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        self.stream.write_all(buf)
    }
}

impl Reconnect for TestSocket {
    fn reconnect(&self) -> Result<(), Error> {
        self.state.reconnect_started.store(true, Ordering::SeqCst);
        let Some((open, opened)) = self.state.gate.as_ref() else {
            return Err(Error::Simple("simulated connect failure".into()));
        };
        let mut open = open.lock().unwrap();
        while !*open {
            open = opened.wait(open).unwrap();
        }
        Ok(())
    }

    fn sleep(&self, duration: Duration, shutdown: &ShutdownSignal) {
        self.state.sleep_started.store(true, Ordering::SeqCst);
        shutdown.wait_timeout(duration)
    }

    fn shutdown_read(&self) -> Result<(), Error> {
        self.stream.shutdown_read()
    }
}

impl Stream for TestSocket {}

/// Poll `condition` until it holds, failing rather than hanging.
fn wait_for(label: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        thread::sleep(Duration::from_millis(5));
    }
}

/// A shutdown requested while `reconnect` is waiting out its backoff must end
/// the wait and return `Error::Shutdown`, not run the whole Fibonacci
/// schedule. With `max_reconnect_attempts = None` the pre-fix loop never
/// returned at all, so the receive timeout is what fails a regression.
#[test]
fn reconnect_returns_shutdown_while_waiting_out_backoff() {
    let stream = MemoryStream::default();
    let socket = TestSocket::unreachable(stream);
    let mut connection = Connection::stubbed(socket.clone(), CLIENT_ID);
    connection.max_reconnect_attempts = None;

    let connection = Arc::new(connection);
    let shutdown = connection.shutdown_signal();

    let (sender, receiver) = std::sync::mpsc::channel();
    let conn_for_thread = Arc::clone(&connection);
    thread::spawn(move || {
        let _ = sender.send(conn_for_thread.reconnect());
    });

    // The first backoff delay is a second; request shutdown well inside it.
    thread::sleep(Duration::from_millis(50));
    let requested_at = Instant::now();
    shutdown.request();

    let result = receiver.recv_timeout(Duration::from_secs(5)).expect("reconnect did not return");
    assert!(matches!(result, Err(Error::Shutdown)), "expected Error::Shutdown, got {result:?}");
    // The wait is against a 1 s backoff, so a few hundred milliseconds is
    // slack enough under load while still failing a non-interruptible wait.
    assert!(
        requested_at.elapsed() < Duration::from_millis(250),
        "reconnect waited out the backoff: {:?}",
        requested_at.elapsed()
    );
    assert!(!socket.reconnect_started(), "reconnect must not attempt a connect after shutdown");
}

/// The dispatcher must stop promptly when shutdown is requested while it is
/// inside `reconnect`, instead of blocking `ensure_shutdown` (and so
/// `Client::drop`) until every backoff attempt is exhausted.
#[test]
fn dispatcher_exits_when_shutdown_requested_during_reconnect() {
    let stream = MemoryStream::default();
    let socket = TestSocket::unreachable(stream.clone());
    let connection = Connection::stubbed(socket.clone(), CLIENT_ID);

    push_handshake(&stream);
    connection.establish_connection().expect("establish_connection failed");
    let server_version = connection.server_version();

    let bus = Arc::new(TcpMessageBus::new(connection).expect("TcpMessageBus::new"));
    bus.process_messages(server_version).expect("process_messages");

    // Break the read: the dispatcher enters reconnect and waits out its
    // backoff between failing connects.
    stream.close();
    wait_for("reconnect backoff to start", || socket.sleep_started());

    let (sender, receiver) = std::sync::mpsc::channel();
    let bus_for_thread = Arc::clone(&bus);
    let requested_at = Instant::now();
    thread::spawn(move || {
        MessageBus::ensure_shutdown(&*bus_for_thread);
        let _ = sender.send(());
    });

    receiver.recv_timeout(Duration::from_secs(5)).expect("ensure_shutdown did not return");
    // The wait is against a 1 s backoff, so a few hundred milliseconds is
    // slack enough under load while still failing a non-interruptible wait.
    assert!(
        requested_at.elapsed() < Duration::from_millis(250),
        "ensure_shutdown waited out the backoff: {:?}",
        requested_at.elapsed()
    );
    assert!(!socket.reconnect_started(), "reconnect must not attempt a connect after shutdown");
    assert!(!MessageBus::is_connected(&*bus));
}

/// Sync mirror of the async dispatcher test: a shutdown requested while a
/// connect is in flight must still stop the dispatcher, even though that
/// connect - and the handshake replay behind it - then succeeds.
///
/// Covers the path rather than the early-exit check itself: without the check
/// the dispatcher still exits, one read later, when the next read finds the
/// stream closed and the shutdown flag set.
#[test]
fn dispatcher_exits_when_shutdown_requested_during_a_successful_reconnect() {
    let stream = MemoryStream::default();
    let socket = TestSocket::gated(stream.clone());
    let connection = Connection::stubbed(socket.clone(), CLIENT_ID);

    push_handshake(&stream);
    connection.establish_connection().expect("establish_connection failed");
    let server_version = connection.server_version();
    let shutdown = connection.shutdown_signal();

    let bus = Arc::new(TcpMessageBus::new(connection).expect("TcpMessageBus::new"));
    bus.process_messages(server_version).expect("process_messages");

    // Break the read: the dispatcher enters reconnect, waits out its backoff
    // and blocks on the gated connect.
    stream.close();
    wait_for("reconnect to start", || socket.reconnect_started());

    // Queue the handshake the post-reconnect session replays, then request
    // shutdown (ensure_shutdown joins the dispatcher, so it runs on its own
    // thread) and only then let the connect succeed.
    push_handshake(&stream);
    let (sender, receiver) = std::sync::mpsc::channel();
    let bus_for_thread = Arc::clone(&bus);
    thread::spawn(move || {
        MessageBus::ensure_shutdown(&*bus_for_thread);
        let _ = sender.send(());
    });

    wait_for("shutdown to be requested", || shutdown.is_requested());
    socket.release();

    receiver.recv_timeout(Duration::from_secs(5)).expect("ensure_shutdown did not return");
    assert!(!MessageBus::is_connected(&*bus));
}
