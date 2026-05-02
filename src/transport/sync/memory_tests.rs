use super::*;

/// Round-trip a frame through the sync `MemoryStream`. Either ordering is
/// correct — if the consumer reaches `pop_front` first it parks on the
/// `Condvar` and the producer's `notify_one` wakes it; if the producer pushes
/// first the consumer returns immediately. EOF surfaces as `Io(UnexpectedEof)`
/// after `close`.
#[test]
fn round_trip_frame() {
    let stream = MemoryStream::default();

    let producer = stream.clone();
    let push = std::thread::spawn(move || {
        producer.push_inbound(b"hello".to_vec());
    });

    let body = stream.read_message().unwrap();
    assert_eq!(body, b"hello");
    push.join().unwrap();

    stream.write_all(b"out").unwrap();
    assert_eq!(stream.captured(), b"out");

    stream.close();
    let err = stream.read_message().unwrap_err();
    assert!(
        matches!(err, Error::Io(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof),
        "unexpected error: {err:?}"
    );
}
