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

/// Unlike the inherent `close`, the transport close survives `reopen`.
#[test]
fn trait_close_is_terminal() {
    let stream = MemoryStream::default();
    Io::close(&stream);
    stream.reopen();
    stream.push_inbound(b"late".to_vec());
    assert!(matches!(stream.read_message(), Err(Error::Shutdown)));
    assert!(matches!(stream.write_all(b"late"), Err(Error::Shutdown)));
    assert!(matches!(stream.reconnect(), Err(Error::Shutdown)));
    assert!(stream.captured().is_empty());
}

/// The transport close wakes a reader already parked on the condvar.
#[test]
fn trait_close_wakes_a_pending_reader() {
    let stream = MemoryStream::default();
    let reading = stream.clone();
    let read = std::thread::spawn(move || reading.read_message());
    std::thread::sleep(std::time::Duration::from_millis(20));
    Io::close(&stream);
    assert!(matches!(read.join().unwrap(), Err(Error::Shutdown)));
}
