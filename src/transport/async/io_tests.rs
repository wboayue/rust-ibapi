use super::*;
use crate::messages::encode_raw_length;
use crate::transport::raw_capture::test_support;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

/// Async twin of `transport::sync::tests::test_read_message_taps_raw_bytes_including_rejected_prefixes`.
///
/// Two claims in one: the reader rejects a length prefix that cannot describe a
/// frame, and the tap sees that prefix anyway. The second is the one worth a
/// test — a capture holding only frames the reader accepted would omit the byte
/// sequence in #891 that an operator
/// opens the capture to find.
#[tokio::test]
async fn test_read_framed_message_taps_raw_bytes_including_rejected_prefixes() {
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
async fn socket_pair() -> (Arc<AsyncTcpSocket>, TcpStream, tokio::net::TcpListener) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (socket, peer) = tokio::join!(AsyncTcpSocket::connect(&address, true), listener.accept());
    (Arc::new(socket.unwrap()), peer.unwrap().0, listener)
}

/// Write `bytes` with a caller that gives up after 100ms; the write must not
/// have completed by then.
async fn abandon_write(socket: &AsyncTcpSocket, bytes: &[u8]) {
    let written = tokio::time::timeout(Duration::from_millis(100), socket.write_all(bytes)).await;
    assert!(written.is_err(), "write completed; the peer should not be reading yet");
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
async fn test_write_cut_off_mid_frame_breaks_the_connection() {
    let (socket, peer, _listener) = socket_pair().await;
    let large = vec![0xAA; LARGE];

    let (read, ()) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(1), socket.read_message()),
        abandon_write(&socket, &large),
    );
    let read = read.expect("pending read not woken");
    assert!(matches!(read, Err(Error::ConnectionReset)), "read: {read:?}");
    let written = socket.write_all(&[0xCC; 3]).await;
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
async fn test_write_abandoned_before_it_starts_sends_nothing() {
    let (socket, peer, _listener) = socket_pair().await;
    let large = vec![0xAA; LARGE];

    let stalled = tokio::spawn({
        let socket = socket.clone();
        let large = large.clone();
        async move { socket.write_all(&large).await }
    });
    // Wait until the large write holds the writer (and stalls on the full buffer).
    tokio::time::timeout(Duration::from_secs(1), async {
        while socket.writer.try_lock().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("large write never took the writer");
    abandon_write(&socket, &[0xBB; 3]).await;

    let reader = read_to_end(peer);
    stalled.await.unwrap().unwrap();
    socket.write_all(&[0xCC; 3]).await.unwrap();
    drop(socket);

    let received = reader.await.unwrap();
    assert!(received == [large, vec![0xCC; 3]].concat(), "received {} bytes", received.len());
}

/// `reconnect` clears a break: the new connection reads and writes normally.
#[tokio::test]
async fn test_reconnect_clears_a_broken_connection() {
    let (socket, _peer, listener) = socket_pair().await;
    abandon_write(&socket, &vec![0xAA; LARGE]).await;
    assert!(matches!(socket.write_all(&[0xCC; 3]).await, Err(Error::ConnectionReset)));

    let (reconnected, accepted) = tokio::join!(socket.reconnect(), listener.accept());
    reconnected.unwrap();
    let mut peer = accepted.unwrap().0;

    socket.write_all(&[0xCC; 3]).await.unwrap();
    let mut received = [0u8; 3];
    peer.read_exact(&mut received).await.unwrap();
    assert_eq!(received, [0xCC; 3]);

    peer.write_all(&encode_raw_length(&[0, 0, 0, 9, 42])).await.unwrap();
    assert_eq!(socket.read_message().await.unwrap(), [0, 0, 0, 9, 42]);
}
