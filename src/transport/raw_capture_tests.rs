use std::fs;

use tempfile::TempDir;

use super::test_support::{frames, index, segments};
use super::*;
use crate::messages::encode_raw_length;

/// `seq,timestamp,offset,declared_length` — the timestamp is wall-clock, so
/// tests assert on the three deterministic columns.
fn index_without_timestamps(dir: &std::path::Path) -> Vec<(usize, u64, u32)> {
    index(dir)
        .iter()
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 4, "index line must have four columns: {line:?}");
            (fields[0].parse().unwrap(), fields[2].parse().unwrap(), fields[3].parse().unwrap())
        })
        .collect()
}

/// Record `body` as one frame, prefix first, the way a frame reader would.
fn record_frame(tap: &RawFrameTap, body: &[u8]) {
    tap.record_length_prefix(&(body.len() as u32).to_be_bytes());
    tap.record_body(body);
}

#[test]
fn test_disabled_tap_is_inert() {
    let tap = RawFrameTap::disabled();
    assert!(tap.sink.is_none(), "disabled() must hold no sink to write through");

    // Nothing to assert on disk — a disabled tap was never given a destination.
    // What matters is that every entry point tolerates being driven.
    record_frame(&tap, &[1, 2, 3, 4, 5]);
    tap.start_new_segment();
}

#[test]
fn test_unset_env_var_disables_the_tap() {
    temp_env::with_var_unset("IBAPI_RAW_CAPTURE_DIR", || {
        assert!(RawFrameTap::from_env().sink.is_none());
    });
    // An empty value is a common way to switch a capture off in a shell script;
    // it must not create a directory named "".
    temp_env::with_var("IBAPI_RAW_CAPTURE_DIR", Some(""), || {
        assert!(RawFrameTap::from_env().sink.is_none());
    });
}

#[test]
fn test_env_var_opens_a_capture_under_the_named_directory() {
    let dir = TempDir::new().unwrap();
    let nested = dir.path().join("does/not/exist/yet");

    temp_env::with_var("IBAPI_RAW_CAPTURE_DIR", Some(nested.to_str().unwrap()), || {
        let tap = RawFrameTap::from_env();
        assert!(tap.sink.is_some());

        record_frame(&tap, &[0, 0, 0, 9]);

        assert_eq!(frames(&nested), encode_raw_length(&[0, 0, 0, 9]));
    });
}

/// Three frames, the middle one carrying a payload — enough that a replay which
/// mis-tracks a boundary lands somewhere visibly wrong.
const REPLAY_BODIES: [&[u8]; 3] = [&[0, 0, 0, 9], &[0, 0, 0, 5, 1, 2, 3], &[0, 0, 0, 63, 8, 208, 70]];

fn capture_replay_bodies(dir: &std::path::Path) -> Vec<u8> {
    let tap = RawFrameTap::capturing_to(dir);
    for body in REPLAY_BODIES {
        record_frame(&tap, body);
    }
    frames(dir)
}

/// The property the whole capture exists for: a `.bin` is the inbound stream
/// verbatim, so replaying it through the production frame reader reproduces the
/// frames — and would reproduce a desync just as faithfully.
#[cfg(feature = "sync")]
#[test]
fn test_capture_replays_through_the_blocking_frame_reader() {
    let dir = TempDir::new().unwrap();
    let capture = capture_replay_bodies(dir.path());

    let mut replay = capture.as_slice();
    for body in REPLAY_BODIES {
        let frame = crate::transport::sync::read_message(&mut replay, &RawFrameTap::disabled()).expect("captured frame must replay");
        assert_eq!(frame, body);
    }
    assert!(replay.is_empty(), "replay must consume the capture exactly");
}

/// Async twin of the blocking replay test. The two readers unframe
/// independently, so a capture that replays through one proves nothing about
/// the other.
#[cfg(feature = "async")]
#[tokio::test]
async fn test_capture_replays_through_the_async_frame_reader() {
    use crate::transport::r#async::read_framed_message;

    let dir = TempDir::new().unwrap();
    let capture = capture_replay_bodies(dir.path());

    let mut replay = std::io::Cursor::new(capture.clone());
    for body in REPLAY_BODIES {
        let frame = read_framed_message(&mut replay, &RawFrameTap::disabled())
            .await
            .expect("captured frame must replay");
        assert_eq!(frame, body);
    }
    assert_eq!(replay.position() as usize, capture.len(), "replay must consume the capture exactly");
}

/// A prefix with no body behind it is the shape a desync takes. It has to reach
/// the capture and the index, or the artifact is missing the only frame anyone
/// would open it to look at.
#[test]
fn test_prefix_without_a_body_is_still_captured() {
    let dir = TempDir::new().unwrap();
    let tap = RawFrameTap::capturing_to(dir.path());

    record_frame(&tap, &[0, 0, 0, 9]);
    tap.record_length_prefix(&u32::MAX.to_be_bytes());

    let mut expected = encode_raw_length(&[0, 0, 0, 9]);
    expected.extend_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(frames(dir.path()), expected);

    // Second entry's declared length is the evidence; its offset, followed by no
    // further bytes, is how a reader tells the body never arrived.
    assert_eq!(index_without_timestamps(dir.path()), vec![(0, 0, 4), (1, 8, u32::MAX)]);
}

#[test]
fn test_index_offsets_locate_each_prefix_in_the_capture() {
    let dir = TempDir::new().unwrap();
    let capture = capture_replay_bodies(dir.path());
    for (seq, offset, declared) in index_without_timestamps(dir.path()) {
        let offset = offset as usize;
        assert_eq!(
            u32::from_be_bytes(capture[offset..offset + 4].try_into().unwrap()),
            declared,
            "index row {seq} must point at its own length prefix"
        );
    }
}

#[test]
fn test_index_timestamps_are_utc_and_ordered() {
    let dir = TempDir::new().unwrap();
    let tap = RawFrameTap::capturing_to(dir.path());

    tap.record_length_prefix(&[0, 0, 0, 4]);
    tap.record_length_prefix(&[0, 0, 0, 4]);

    let stamps: Vec<String> = index(dir.path()).iter().map(|line| line.split(',').nth(1).unwrap().to_string()).collect();
    assert_eq!(stamps.len(), 2);
    for stamp in &stamps {
        assert!(stamp.ends_with('Z'), "timestamp must be marked UTC: {stamp}");
        assert!(stamp.contains('T'), "timestamp must be ISO-8601: {stamp}");
    }
    assert!(stamps[0] <= stamps[1], "timestamps must not go backwards: {stamps:?}");
}

/// A reconnect starts a fresh TCP stream. Appending it to the previous capture
/// would read back as a framing desync at the seam — the exact artifact these
/// files exist to prove or disprove.
#[test]
fn test_reconnect_starts_a_new_segment() {
    let dir = TempDir::new().unwrap();
    let tap = RawFrameTap::capturing_to(dir.path());

    record_frame(&tap, &[0, 0, 0, 9]);
    tap.start_new_segment();
    record_frame(&tap, &[0, 0, 0, 5]);

    let bins = segments(dir.path(), "bin");
    assert_eq!(bins.len(), 2, "each connection gets its own file: {bins:?}");
    assert_eq!(fs::read(&bins[0]).unwrap(), encode_raw_length(&[0, 0, 0, 9]));
    assert_eq!(fs::read(&bins[1]).unwrap(), encode_raw_length(&[0, 0, 0, 5]));

    // Sequence and offset restart with the segment, so an index row is read
    // against the `.bin` beside it and never against a neighbour.
    let per_segment: Vec<Vec<String>> = segments(dir.path(), "idx")
        .iter()
        .map(|path| fs::read_to_string(path).unwrap().lines().map(String::from).collect())
        .collect();
    for lines in &per_segment {
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("0,"), "sequence restarts per segment: {lines:?}");
        assert!(lines[0].ends_with(",0,4"), "offset restarts per segment: {lines:?}");
    }
}

#[test]
fn test_clones_share_one_capture() {
    let dir = TempDir::new().unwrap();
    let tap = RawFrameTap::capturing_to(dir.path());
    let clone = tap.clone();

    record_frame(&tap, &[0, 0, 0, 9]);
    record_frame(&clone, &[0, 0, 0, 5]);

    assert_eq!(segments(dir.path(), "bin").len(), 1, "a clone must not open its own file");
    let mut expected = encode_raw_length(&[0, 0, 0, 9]);
    expected.extend_from_slice(&encode_raw_length(&[0, 0, 0, 5]));
    assert_eq!(frames(dir.path()), expected);
}

/// A capture is a diagnostic aid. An unusable destination downgrades it to a
/// no-op rather than failing the connection that was being diagnosed.
#[test]
fn test_unusable_directory_downgrades_to_disabled() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("not-a-directory");
    fs::write(&file, b"occupied").unwrap();

    let tap = RawFrameTap::capturing_to(file.join("capture"));
    assert!(tap.sink.is_none(), "a destination that cannot be opened yields no sink at all");

    // Still safe to drive.
    record_frame(&tap, &[0, 0, 0, 9]);
    tap.start_new_segment();
}

/// A destination that goes away mid-run — an unmounted volume, a cleaned tmpdir
/// — must stop the capture, not the connection. And it must stop it *once*: a
/// reconnecting client calls `start_new_segment` on every attempt, and a warning
/// per attempt on a failing disk is its own problem.
#[test]
fn test_a_failed_segment_open_gives_up_permanently() {
    let dir = TempDir::new().unwrap();
    let capture_dir = dir.path().join("capture");
    let tap = RawFrameTap::capturing_to(&capture_dir);
    record_frame(&tap, &[0, 0, 0, 9]);

    fs::remove_dir_all(&capture_dir).unwrap();
    tap.start_new_segment();

    // Losing the segment *is* the retired state — there is no second flag that
    // could disagree with it.
    let sink = tap.sink.as_ref().expect("the tap outlives its destination");
    assert!(sink.lock().segment.is_none(), "a failed open must retire the sink");

    // Still safe to drive, and silent from here on: nothing reopens.
    record_frame(&tap, &[0, 0, 0, 9]);
    tap.start_new_segment();
    assert!(sink.lock().segment.is_none(), "a retired sink must not reopen");
}

#[test]
fn test_tap_is_send_and_sync() {
    crate::tests::assert_send_and_sync::<RawFrameTap>();
}
