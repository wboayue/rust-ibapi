//! Byte-level capture of the inbound TWS stream, enabled by setting
//! `IBAPI_RAW_CAPTURE_DIR` to a directory.
//!
//! This is a *tap*, not a recorder: it sits below the framing, so what lands on
//! disk is a byte-for-byte copy of what the socket handed us — **including the
//! 4-byte big-endian length prefix**. [`MessageRecorder`] cannot do this. It is
//! called with an already-parsed [`ResponseMessage`] and re-synthesises a frame,
//! so the prefix it writes is one this crate computed rather than one TWS sent,
//! and an unrecognised message id has already collapsed to `NotValid`.
//!
//! That distinction is the whole point. The failure under investigation in
//! `plans/tick-by-tick-reconnect-decode-desync.md` is a *framing* desync: a
//! length prefix that does not describe the frame that follows, after which
//! every subsequent read starts mid-message. A recorder-based capture cannot
//! contain the evidence, because the corrupted field is the one it discards.
//!
//! # Output
//!
//! One pair of files per connection (a reconnect starts a new pair, so each
//! `.bin` is one continuous TCP stream and nothing splices two of them):
//!
//! - `<stamp>-<instance>-inbound-<NNN>.bin` — the raw stream. Since the length
//!   prefixes are preserved, this file *is* replayable: feeding it to the same
//!   frame reader reproduces the same sequence of frames, and the same desync.
//! - `<stamp>-<instance>-inbound-<NNN>.idx` — one CSV line per frame,
//!   `seq,utc_timestamp,offset,declared_length`, where `offset` locates the
//!   length prefix in the `.bin`. The `.bin` carries no timestamps, and
//!   correlating a desync with a data-farm notice in the operator's log needs
//!   wall-clock. A line is written as soon as the prefix is read, so a frame
//!   whose body never arrived still appears — that is the interesting one.
//!
//! # Limits
//!
//! Bytes consumed by a `read_exact` that then *fails* never reach the tap — the
//! read has taken them off the socket and dropped them before returning. So a
//! `.bin` is byte-exact only for reads that completed. A desync caused that way
//! is still visible (the next prefix in the capture is the shifted one), but the
//! lost bytes are simply absent, and the capture will not replay against the
//! wire byte for byte. See F8 in
//! `plans/tick-by-tick-reconnect-decode-desync.md`.
//!
//! # Cost
//!
//! Nothing here is on the hot path unless the environment variable is set: a
//! disabled tap is an `Option::None` check per frame, before any formatting,
//! allocation, or locking.
//!
//! When enabled it costs three unbuffered `write(2)` per frame. **The files are
//! deliberately not `BufWriter`-wrapped.** `File::write_all` reaches the kernel
//! page cache, so a capture survives `kill -9`; an 8 KiB user-space buffer would
//! lose its tail — and the frames immediately before a hang or a desync are the
//! entire evidence. Do not "optimise" this without solving that.
//!
//! On the async side those writes are blocking calls on a runtime worker, made
//! while the reader mutex is held. That is a few microseconds against a local
//! directory and unbounded against a slow or full one, which is the deal a
//! byte-level capture makes; point `IBAPI_RAW_CAPTURE_DIR` at local disk.
//!
//! [`MessageRecorder`]: super::recorder::MessageRecorder
//! [`ResponseMessage`]: crate::messages::ResponseMessage

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use log::{info, warn};
use time::macros::format_description;
use time::OffsetDateTime;

static TAP_ID: AtomicUsize = AtomicUsize::new(0);

/// Records the inbound byte stream, or does nothing at all.
///
/// Cheap to clone; clones share one set of files. Construct with
/// [`RawFrameTap::from_env`] in production and [`RawFrameTap::disabled`]
/// wherever a stream is not the real socket.
#[derive(Clone, Debug)]
pub(crate) struct RawFrameTap {
    sink: Option<Arc<Sink>>,
}

impl RawFrameTap {
    /// A tap that captures nothing. Every record call is a no-op.
    pub(crate) fn disabled() -> Self {
        Self { sink: None }
    }

    /// Read `IBAPI_RAW_CAPTURE_DIR`. Unset or empty yields [`Self::disabled`].
    pub(crate) fn from_env() -> Self {
        match env::var("IBAPI_RAW_CAPTURE_DIR") {
            Ok(dir) if !dir.is_empty() => Self::capturing_to(dir),
            _ => Self::disabled(),
        }
    }

    /// Capture into `dir`, creating it if needed.
    ///
    /// A destination that cannot be opened downgrades to [`Self::disabled`]
    /// with a warning: a diagnostic aid must never be the reason a connection
    /// fails.
    pub(crate) fn capturing_to(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref().to_path_buf();
        if let Err(err) = fs::create_dir_all(&dir) {
            warn!("raw frame capture disabled: cannot create {}: {err}", dir.display());
            return Self::disabled();
        }

        let stamp = OffsetDateTime::now_utc()
            .format(&format_description!("[year]-[month]-[day]-[hour]-[minute]"))
            .unwrap_or_else(|_| String::from("unknown"));
        let sink = Sink {
            prefix: format!("{stamp}-{}", TAP_ID.fetch_add(1, Ordering::SeqCst)),
            dir,
            // Segment 0 is claimed below; the next rotation takes 1.
            state: Mutex::new(State {
                segment: None,
                next_number: 1,
            }),
        };

        // Open segment 0 up front, so that "has a segment" is the single
        // meaning of "still capturing" from here on — no separate disabled
        // flag, and no window where the two disagree.
        match sink.new_segment(0) {
            Some(segment) => {
                sink.lock().segment = Some(segment);
                Self { sink: Some(Arc::new(sink)) }
            }
            None => Self::disabled(),
        }
    }

    /// Record a length prefix the moment it comes off the socket — **before**
    /// it is validated or used to size a read.
    ///
    /// Ordering matters: an out-of-range prefix is rejected by
    /// [`validate_frame_length`], and a prefix that is merely *wrong* sends the
    /// next `read_exact` past the end of the real frame. Either way the prefix
    /// is the evidence, so it has to be on disk before anything can reject it.
    ///
    /// [`validate_frame_length`]: super::common::validate_frame_length
    pub(crate) fn record_length_prefix(&self, prefix: &[u8; 4]) {
        let Some(sink) = &self.sink else { return };
        let declared = u32::from_be_bytes(*prefix);
        sink.write(|segment| {
            let line = format!("{},{},{},{declared}\n", segment.next_seq, timestamp(), segment.offset);
            segment.frames.write_all(prefix)?;
            segment.offset += prefix.len() as u64;
            segment.next_seq += 1;
            segment.index.write_all(line.as_bytes())
        });
    }

    /// Record a frame body, after the read that filled it succeeded.
    pub(crate) fn record_body(&self, body: &[u8]) {
        let Some(sink) = &self.sink else { return };
        sink.write(|segment| {
            segment.frames.write_all(body)?;
            segment.offset += body.len() as u64;
            Ok(())
        });
    }

    /// Begin a new file pair. Called on reconnect, so that a `.bin` is never a
    /// splice of two TCP streams — replaying such a file would show a phantom
    /// desync at the seam.
    ///
    /// A sink that has already given up stays given up: a failing disk should
    /// not produce a fresh warning on every reconnect attempt.
    pub(crate) fn start_new_segment(&self) {
        let Some(sink) = &self.sink else { return };
        let mut state = sink.lock();
        if state.segment.is_none() {
            return;
        }
        let number = state.next_number;
        state.next_number += 1;
        state.segment = sink.new_segment(number);
    }
}

fn timestamp() -> String {
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");
    OffsetDateTime::now_utc().format(&format).unwrap_or_else(|_| String::from("unknown"))
}

#[derive(Debug)]
struct Sink {
    dir: PathBuf,
    prefix: String,
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    /// The open capture, or `None` once an I/O error has retired this sink for
    /// good. There is no separate disabled flag: a `Sink` only ever exists with
    /// segment 0 already open, so `None` means exactly one thing.
    segment: Option<Segment>,
    next_number: usize,
}

#[derive(Debug)]
struct Segment {
    frames: File,
    index: File,
    /// Bytes written to `frames` so far — the offset the next prefix lands at.
    offset: u64,
    next_seq: usize,
}

impl Sink {
    /// A poisoned lock means some other thread panicked mid-capture. The
    /// capture may have a torn frame in it, but refusing to record from here on
    /// is strictly worse than continuing, so recover the guard.
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Open the file pair for `number`, or `None` with a warning if either file
    /// cannot be created.
    fn new_segment(&self, number: usize) -> Option<Segment> {
        let base = self.dir.join(format!("{}-inbound-{number:03}", self.prefix));
        let frames_path = base.with_extension("bin");
        match (File::create(&frames_path), File::create(base.with_extension("idx"))) {
            (Ok(frames), Ok(index)) => {
                info!("raw frame capture: {}", frames_path.display());
                Some(Segment {
                    frames,
                    index,
                    offset: 0,
                    next_seq: 0,
                })
            }
            (Err(err), _) | (_, Err(err)) => {
                warn!("raw frame capture disabled: cannot open {}: {err}", frames_path.display());
                None
            }
        }
    }

    /// Run one record step against the open segment. An I/O failure retires the
    /// sink — one warning for a failing disk, not one per frame.
    fn write(&self, record: impl FnOnce(&mut Segment) -> std::io::Result<()>) {
        let mut state = self.lock();
        let Some(segment) = state.segment.as_mut() else { return };
        if let Err(err) = record(segment) {
            warn!("raw frame capture disabled: write failed: {err}");
            state.segment = None;
        }
    }
}

/// Read a capture back. Lives here because the file-naming scheme is this
/// module's private business, and three test files need to inspect one.
#[cfg(test)]
pub(crate) mod test_support {
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Capture files in `dir` with the given extension, in segment order.
    pub(crate) fn segments(dir: &Path, extension: &str) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = fs::read_dir(dir)
            .expect("capture directory must exist")
            .map(|entry| entry.expect("readable entry").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == extension))
            .collect();
        paths.sort();
        paths
    }

    /// Every captured byte, segments concatenated in order. For a single-segment
    /// capture this is the inbound stream verbatim.
    pub(crate) fn frames(dir: &Path) -> Vec<u8> {
        segments(dir, "bin")
            .iter()
            .flat_map(|path| fs::read(path).expect("readable capture"))
            .collect()
    }

    /// Index lines across all segments, in order, newlines stripped.
    pub(crate) fn index(dir: &Path) -> Vec<String> {
        segments(dir, "idx")
            .iter()
            .flat_map(|path| {
                fs::read_to_string(path)
                    .expect("readable index")
                    .lines()
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "raw_capture_tests.rs"]
mod tests;
