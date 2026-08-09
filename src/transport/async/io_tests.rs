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
