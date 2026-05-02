use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Spike: push a length-prefixed frame, decode it via length-prefix protocol,
/// then write a frame and verify it lands in the captured outbound buffer.
/// Also validates the waker path: a `read_exact` that pends gets resumed by
/// a later `push_frame` from a producer task without busy-looping.
///
/// Pinned to `current_thread` so the producer task cannot run before the
/// consumer parks — that ordering is what exercises the waker registration.
#[tokio::test(flavor = "current_thread")]
async fn round_trip_length_prefixed_frame() {
    let stream = MemoryStream::new();

    let frame: &[u8] = b"abc\x00def\x00\x01";

    // 1. Producer-side: schedule a push that happens after the consumer has parked.
    let producer = stream.clone();
    let push_task = tokio::spawn(async move {
        tokio::task::yield_now().await;
        producer.push_frame(frame);
    });

    // 2. Consumer reads the length prefix, then the payload.
    let mut consumer = stream.clone();
    let mut len_bytes = [0u8; 4];
    consumer.read_exact(&mut len_bytes).await.unwrap();
    let len = u32::from_be_bytes(len_bytes) as usize;
    assert_eq!(len, frame.len());
    let mut payload = vec![0u8; len];
    consumer.read_exact(&mut payload).await.unwrap();
    assert_eq!(payload, frame);
    push_task.await.unwrap();

    // 3. Round-trip the other direction: write a frame, capture the bytes.
    let outbound = b"hello\x00";
    let mut writer = stream.clone();
    writer.write_all(outbound).await.unwrap();
    writer.flush().await.unwrap();
    assert_eq!(stream.captured(), outbound);

    // 4. Closing surfaces EOF as `Ok(0)` from `read`.
    stream.close();
    let mut tail = [0u8; 4];
    let n = consumer.read(&mut tail).await.unwrap();
    assert_eq!(n, 0);
}
