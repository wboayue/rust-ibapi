use super::*;

/// Round-trip a frame body through the async `MemoryStream`. Either ordering is
/// correct — if the consumer reaches the queue first it parks on `Notify` and
/// the producer's `notify_one` wakes it; if the producer pushes first the
/// consumer returns immediately. EOF surfaces as `Io(UnexpectedEof)` after
/// `close`.
///
/// Pinned to `current_thread` so the producer task cannot run before the
/// consumer arms its notify; that ordering is what exercises the wait path.
#[tokio::test(flavor = "current_thread")]
async fn round_trip_frame() {
    let stream = MemoryStream::default();

    let producer = stream.clone();
    let push = tokio::spawn(async move {
        tokio::task::yield_now().await;
        producer.push_inbound(b"hello".to_vec());
    });

    let body = stream.read_message().await.unwrap();
    assert_eq!(body, b"hello");
    push.await.unwrap();

    stream.write_all(b"out").await.unwrap();
    assert_eq!(stream.captured(), b"out");

    stream.close();
    let err = stream.read_message().await.unwrap_err();
    assert!(
        matches!(err, Error::Io(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof),
        "unexpected error: {err:?}"
    );
}
