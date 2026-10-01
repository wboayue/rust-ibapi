use super::*;
use crate::messages::encode_raw_length;
use crate::transport::raw_capture::test_support;

/// Async twin of `transport::sync::tests::test_read_message_taps_raw_bytes_including_rejected_prefixes`.
///
/// Two claims in one: the reader rejects a length prefix that cannot describe a
/// frame, and the tap sees that prefix anyway. The second is the one worth a
/// test — a capture holding only frames the reader accepted would omit the byte
/// sequence in `plans/tick-by-tick-reconnect-decode-desync.md` that an operator
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
/// not reading stays pending mid-frame.
const LARGE: usize = 32 * 1024 * 1024;

async fn socket_pair() -> (AsyncTcpSocket, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (socket, peer) = tokio::join!(AsyncTcpSocket::connect(&address, true), listener.accept());
    (socket.unwrap(), peer.unwrap().0)
}

/// Write `bytes` with a caller that gives up after 100ms; the write must not
/// have completed by then.
async fn abandon_write(socket: &AsyncTcpSocket, bytes: Vec<u8>) {
    let written = tokio::time::timeout(Duration::from_millis(100), socket.write_all(bytes)).await;
    assert!(written.is_err(), "write completed; the peer should not be reading yet");
}

/// Everything the peer receives until the socket closes.
fn read_to_end(mut peer: TcpStream) -> tokio::task::JoinHandle<Vec<u8>> {
    tokio::spawn(async move {
        let mut received = Vec::new();
        peer.read_to_end(&mut received).await.unwrap();
        received
    })
}

/// The `AsyncIo::write_all` cancellation contract on a real socket, with the
/// peer not reading so the first write stalls mid-frame:
/// - a write abandoned mid-frame still finishes the frame, so the next one
///   follows it whole instead of a fragment TWS would misread;
/// - a write abandoned while it waits for the writer never reaches the wire.
#[tokio::test]
async fn test_abandoned_writes_keep_the_stream_framed() {
    let large = vec![0xAA; LARGE];
    let cases: [(&str, Vec<Vec<u8>>); 2] = [
        ("abandoned mid-frame", vec![large.clone()]),
        ("abandoned before it starts", vec![large.clone(), vec![0xBB; 3]]),
    ];

    for (name, abandoned) in cases {
        let (socket, peer) = socket_pair().await;
        for bytes in abandoned {
            abandon_write(&socket, bytes).await;
        }
        let reader = read_to_end(peer);
        socket.write_all(vec![0xCC; 3]).await.unwrap();
        drop(socket);

        let received = reader.await.unwrap();
        let expected = [large.clone(), vec![0xCC; 3]].concat();
        assert!(
            received == expected,
            "{name}: received {} bytes, expected {}; first difference at byte {:?}",
            received.len(),
            expected.len(),
            received.iter().zip(&expected).position(|(a, b)| a != b)
        );
    }
}
