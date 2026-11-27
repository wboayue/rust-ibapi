use super::*;
use crate::messages::encode_raw_length;
use crate::transport::raw_capture::test_support;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Async twin of `transport::sync::tests::read_message_taps_raw_bytes_including_rejected_prefixes`.
///
/// Two claims in one: the reader rejects a length prefix that cannot describe a
/// frame, and the tap sees that prefix anyway. The second is the one worth a
/// test — a capture holding only frames the reader accepted would omit the byte
/// sequence in #891 that an operator
/// opens the capture to find.
#[tokio::test]
async fn read_framed_message_taps_raw_bytes_including_rejected_prefixes() {
    let dir = tempfile::TempDir::new().unwrap();
    let tap = RawFrameTap::capturing_to(dir.path());

    let good = encode_raw_length(&[0, 0, 0, 9, 42]);
    let bad_prefix = u32::MAX.to_be_bytes();

    let mut stream = std::io::Cursor::new([good.clone(), bad_prefix.to_vec()].concat());
    assert_eq!(read_framed_message(&mut stream, &tap).await.unwrap(), [0, 0, 0, 9, 42]);
    let err = read_framed_message(&mut stream, &tap).await.expect_err("prefix must be rejected");
    assert!(matches!(err, Error::InvalidFrame(_)), "got {err:?}");

    let mut expected = good;
    expected.extend_from_slice(&bad_prefix);
    assert_eq!(test_support::frames(dir.path()), expected);
}

/// Larger than loopback send + receive buffers, so a write to a peer that is
/// not reading stalls mid-frame.
const LARGE: usize = 32 * 1024 * 1024;

/// A connected socket, its peer, and the listener, kept for reconnects.
async fn socket_pair() -> (Arc<AsyncSocket>, TcpStream, tokio::net::TcpListener) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (socket, peer) = tokio::join!(AsyncSocket::connect(Connector::tcp(address, true)), listener.accept());
    (Arc::new(socket.unwrap()), peer.unwrap().0, listener)
}

/// Write `bytes` with a caller that gives up after 100ms; the write must not
/// have completed by then.
async fn abandon_write(socket: &AsyncSocket, bytes: &[u8]) {
    let written = tokio::time::timeout(Duration::from_millis(100), socket.write_all(bytes, None)).await;
    assert!(written.is_err(), "write completed; the peer should not be reading yet");
}

/// Wait until a pending write holds the writer (stalled on the full buffer).
async fn until_writer_held(socket: &AsyncSocket) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while socket.writer.try_lock().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("large write never took the writer");
}

/// The peer sees EOF: our side released its socket.
pub(in crate::transport::r#async) async fn assert_peer_sees_eof(mut peer: TcpStream) {
    let eof = tokio::time::timeout(Duration::from_secs(1), peer.read_u8())
        .await
        .expect("TCP socket retained");
    assert_eq!(eof.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
}

/// Everything the peer receives until the connection closes.
fn read_to_end(mut peer: TcpStream) -> tokio::task::JoinHandle<Vec<u8>> {
    tokio::spawn(async move {
        let mut received = Vec::new();
        peer.read_to_end(&mut received).await.unwrap();
        received
    })
}

/// A write cut off mid-frame breaks the connection: a pending read fails, so
/// the dispatcher reconnects, and later writes are refused rather than
/// appended to the fragment.
#[tokio::test]
async fn write_cut_off_mid_frame_breaks_the_connection() {
    let (socket, peer, _listener) = socket_pair().await;
    let large = vec![0xAA; LARGE];

    let (read, ()) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(1), socket.read_message()),
        abandon_write(&socket, &large),
    );
    let read = read.expect("pending read not woken");
    assert!(matches!(read, Err(Error::ConnectionReset)), "read: {read:?}");
    let written = socket.write_all(&[0xCC; 3], None).await;
    assert!(matches!(written, Err(Error::ConnectionReset)), "write: {written:?}");

    let reader = read_to_end(peer);
    drop(socket);
    let received = reader.await.unwrap();
    assert!(!received.is_empty() && received.len() < LARGE, "no fragment: {} bytes", received.len());
    assert!(received.iter().all(|&b| b == 0xAA), "bytes followed the fragment");
}

/// A write abandoned while it waits for the writer sends nothing and leaves
/// the connection usable.
#[tokio::test]
async fn write_abandoned_before_it_starts_sends_nothing() {
    let (socket, peer, _listener) = socket_pair().await;
    let large = vec![0xAA; LARGE];

    let stalled = tokio::spawn({
        let socket = socket.clone();
        let large = large.clone();
        async move { socket.write_all(&large, None).await }
    });
    until_writer_held(&socket).await;
    abandon_write(&socket, &[0xBB; 3]).await;

    let reader = read_to_end(peer);
    stalled.await.unwrap().unwrap();
    socket.write_all(&[0xCC; 3], None).await.unwrap();
    drop(socket);

    let received = reader.await.unwrap();
    assert!(received == [large, vec![0xCC; 3]].concat(), "received {} bytes", received.len());
}

/// `reconnect` clears a break: the new connection reads and writes normally.
#[tokio::test]
async fn reconnect_clears_a_broken_connection() {
    let (socket, _peer, listener) = socket_pair().await;
    abandon_write(&socket, &vec![0xAA; LARGE]).await;
    assert!(matches!(socket.write_all(&[0xCC; 3], None).await, Err(Error::ConnectionReset)));

    let (reconnected, accepted) = tokio::join!(socket.reconnect(), listener.accept());
    reconnected.unwrap();
    let mut peer = accepted.unwrap().0;

    socket.write_all(&[0xCC; 3], None).await.unwrap();
    let mut received = [0u8; 3];
    peer.read_exact(&mut received).await.unwrap();
    assert_eq!(received, [0xCC; 3]);

    peer.write_all(&encode_raw_length(&[0, 0, 0, 9, 42])).await.unwrap();
    assert_eq!(socket.read_message().await.unwrap(), [0, 0, 0, 9, 42]);
}

/// Terminal close interrupts a blocked read, releases TCP, and cannot reconnect.
#[tokio::test]
async fn physical_close_wakes_read_and_is_terminal() {
    let (socket, peer, _listener) = socket_pair().await;
    let reading = socket.clone();
    let read = tokio::spawn(async move { reading.read_message().await });
    tokio::task::yield_now().await;
    tokio::time::timeout(Duration::from_secs(1), socket.close())
        .await
        .expect("close blocked on read");
    let read = read.await.unwrap();
    assert!(matches!(read, Err(Error::Shutdown)), "read: {read:?}");
    assert_peer_sees_eof(peer).await;
    socket.close().await;
    assert!(matches!(socket.write_all(&[1], None).await, Err(Error::Shutdown)));
    assert!(matches!(socket.read_message().await, Err(Error::Shutdown)));
    assert!(matches!(socket.reconnect().await, Err(Error::Shutdown)));
}

/// Close overrides a break: I/O fails with `Shutdown`, not `ConnectionReset`,
/// and reconnect refuses.
#[tokio::test]
async fn physical_close_overrides_a_broken_connection() {
    let (socket, _peer, _listener) = socket_pair().await;
    abandon_write(&socket, &vec![0xAA; LARGE]).await;
    assert!(matches!(socket.write_all(&[0xCC; 3], None).await, Err(Error::ConnectionReset)));

    socket.close().await;
    let read = socket.read_message().await;
    assert!(matches!(read, Err(Error::Shutdown)), "read: {read:?}");
    let written = socket.write_all(&[0xCC; 3], None).await;
    assert!(matches!(written, Err(Error::Shutdown)), "write: {written:?}");
    assert!(matches!(socket.reconnect().await, Err(Error::Shutdown)));
}

/// A sender queued before close cannot write when its lock becomes available.
#[tokio::test]
async fn physical_close_prevents_a_queued_write() {
    let (socket, peer, _listener) = socket_pair().await;
    let writer = socket.writer.lock().await;
    let sending = socket.clone();
    let write = tokio::spawn(async move { sending.write_all(&[0xAA; 4], None).await });
    tokio::task::yield_now().await;
    let closing = socket.clone();
    let close = tokio::spawn(async move { closing.close().await });
    tokio::time::timeout(Duration::from_secs(1), async {
        while !socket.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("close never latched");
    drop(writer);
    tokio::time::timeout(Duration::from_secs(1), close).await.unwrap().unwrap();
    assert!(matches!(write.await.unwrap(), Err(Error::Shutdown)));
    assert_peer_sees_eof(peer).await;
}

/// Close wakes a write stalled on a full send buffer, rather than waiting for
/// the peer to drain it.
#[tokio::test]
async fn physical_close_wakes_a_backpressured_write() {
    let (socket, _peer, _listener) = socket_pair().await;
    let write = tokio::spawn({
        let socket = socket.clone();
        async move { socket.write_all(&vec![0xAA; LARGE], None).await }
    });
    until_writer_held(&socket).await;

    tokio::time::timeout(Duration::from_secs(1), socket.close())
        .await
        .expect("close blocked on write");
    let written = write.await.unwrap();
    assert!(matches!(written, Err(Error::Shutdown)), "write: {written:?}");
    // The woken write was cut off mid-frame; that must not undo the close.
    assert!(matches!(socket.reconnect().await, Err(Error::Shutdown)));
}

/// `sever` fails a write stuck on a peer that stopped reading; later writes
/// fail too, until a reconnect replaces the connection.
#[tokio::test]
async fn sever_fails_a_stuck_write_until_reconnect() {
    use tokio::io::AsyncReadExt;

    let (socket, _peer, listener) = socket_pair().await;
    let stuck = tokio::spawn({
        let socket = socket.clone();
        async move { socket.write_all(&vec![0xAA; LARGE], None).await }
    });
    until_writer_held(&socket).await;

    socket.sever();
    let written = tokio::time::timeout(Duration::from_secs(1), stuck)
        .await
        .expect("sever left the write stuck")
        .unwrap();
    assert!(matches!(written, Err(Error::ConnectionReset)), "write: {written:?}");
    assert!(matches!(socket.write_all(&[1], None).await, Err(Error::ConnectionReset)));

    let (reconnected, accepted) = tokio::join!(socket.reconnect(), listener.accept());
    reconnected.expect("reconnect after sever");
    let (mut new_peer, _) = accepted.unwrap();
    socket.write_all(b"live", None).await.expect("the new connection must take writes");
    let mut received = [0u8; 4];
    new_peer.read_exact(&mut received).await.unwrap();
    assert_eq!(&received, b"live");
}

/// A caller's connector, over in-memory duplex pipes: each call opens a fresh
/// pipe and hands its far end to the test.
fn duplex_connector(capacity: usize) -> (Connector, tokio::sync::mpsc::UnboundedReceiver<tokio::io::DuplexStream>) {
    let (peers, opened) = tokio::sync::mpsc::unbounded_channel();
    let connector = Connector::from_stream(move || {
        let (near, far) = tokio::io::duplex(capacity);
        let opened = peers.send(far);
        async move {
            opened.map_err(|_| std::io::Error::other("test dropped the peer receiver"))?;
            Ok(near)
        }
    });
    (connector, opened)
}

/// A caller's stream gets the same framing as TCP, and `reconnect` calls the
/// connector again rather than reusing the stream it replaced.
#[tokio::test]
async fn caller_stream_is_framed_and_reopened_on_reconnect() {
    let (connector, mut opened) = duplex_connector(1024);
    let socket = AsyncSocket::connect(connector).await.unwrap();
    let mut first = opened.try_recv().expect("connect did not open a stream");

    socket.write_all(&encode_raw_length(&[0, 0, 0, 1, 7]), None).await.unwrap();
    let mut sent = [0u8; 9];
    first.read_exact(&mut sent).await.unwrap();
    assert_eq!(sent, [0, 0, 0, 5, 0, 0, 0, 1, 7]);
    first.write_all(&encode_raw_length(&[0, 0, 0, 2, 8])).await.unwrap();
    assert_eq!(socket.read_message().await.unwrap(), [0, 0, 0, 2, 8]);

    socket.reconnect().await.unwrap();
    let mut second = opened.try_recv().expect("reconnect did not open a new stream");
    second.write_all(&encode_raw_length(&[0, 0, 0, 3, 9])).await.unwrap();
    assert_eq!(socket.read_message().await.unwrap(), [0, 0, 0, 3, 9]);
    // The replaced stream was released.
    assert_eq!(first.read_u8().await.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
}

/// Frame validation covers a caller's stream: an impossible length prefix
/// fails the read instead of being trusted.
#[tokio::test]
async fn caller_stream_rejects_an_invalid_frame_length() {
    let (connector, mut opened) = duplex_connector(1024);
    let socket = AsyncSocket::connect(connector).await.unwrap();
    let mut peer = opened.try_recv().unwrap();

    peer.write_all(&u32::MAX.to_be_bytes()).await.unwrap();
    let read = socket.read_message().await;
    assert!(matches!(read, Err(Error::InvalidFrame(_))), "got {read:?}");
}

#[tokio::test(start_paused = true)]
async fn pending_first_write_rechecks_its_deadline_before_accepting_bytes() {
    let (connector, mut opened) = duplex_connector(1);
    let socket = AsyncSocket::connect(connector).await.unwrap();
    let mut peer = opened.try_recv().unwrap();
    socket.write_all(b"A", None).await.unwrap();
    let deadline = Instant::now() + Duration::from_micros(400);
    let (admission, _cancel) = WriteAdmission::new(deadline);
    let write = socket.write_all(b"B", Some(&admission));
    tokio::pin!(write);
    assert!(futures::poll!(write.as_mut()).is_pending());

    tokio::time::advance(Duration::from_micros(400)).await;
    assert_eq!(peer.read_u8().await.unwrap(), b'A');
    let result = futures::poll!(write.as_mut());

    assert!(matches!(result, std::task::Poll::Ready(Err(Error::Timeout))), "got {result:?}");
    assert!(futures::poll!(Box::pin(peer.read_u8()).as_mut()).is_pending());
    socket.write_all(b"C", None).await.unwrap();
    assert_eq!(peer.read_u8().await.unwrap(), b'C');
}

/// A caller's connector that never resolves on reconnect cannot hold up
/// `close`: the pending reconnect fails with `Shutdown`, so `disconnect`'s
/// join of the dispatcher returns.
#[tokio::test]
async fn close_ends_a_reconnect_stuck_in_the_connector() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let connector = Connector::from_stream(move || {
        let first = counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0;
        async move {
            if !first {
                std::future::pending::<()>().await;
            }
            Ok(tokio::io::duplex(1024).0)
        }
    });
    let socket = Arc::new(AsyncSocket::connect(connector).await.unwrap());

    let reconnecting = tokio::spawn({
        let socket = Arc::clone(&socket);
        async move { socket.reconnect().await }
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        while calls.load(std::sync::atomic::Ordering::SeqCst) < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("reconnect never called the connector");
    socket.close().await;

    let reconnected = tokio::time::timeout(Duration::from_secs(1), reconnecting)
        .await
        .expect("reconnect still stuck after close")
        .unwrap();
    assert!(matches!(reconnected, Err(Error::Shutdown)), "got {reconnected:?}");
}
