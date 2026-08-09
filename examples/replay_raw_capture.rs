//! Walk a raw inbound capture produced by `IBAPI_RAW_CAPTURE_DIR` and report
//! where — if anywhere — the framing came apart.
//!
//! ```bash
//! IBAPI_RAW_CAPTURE_DIR=/tmp/tws-raw cargo run --example tick_by_tick   # capture
//! cargo run --example replay_raw_capture -- /tmp/tws-raw/*-inbound-000.bin
//! ```
//!
//! A `.bin` is a byte-for-byte copy of the inbound stream, length prefixes
//! included, so this walks it exactly as the transport does: read a 4-byte
//! big-endian length, then that many bytes, of which the first four are the
//! message id. Deliberately *not* using the crate's internal reader — the point
//! is to see what the wire said, including the frames the reader would refuse.
//!
//! Pass `--frames` to list every frame; by default only the anomalies print.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::ExitCode;

use ibapi::IncomingMessages;

const USAGE: &str = "usage: replay_raw_capture [--frames] <capture.bin>";

/// The transport's own bounds, copied because `transport::common` is
/// `pub(crate)` and an example links as an external consumer. If the crate ever
/// moves these, this tool will disagree with the reader it is adjudicating —
/// keep them in step by hand.
const MAX_FRAME_LENGTH: usize = 0x00FF_FFFF;
const MIN_FRAME_LENGTH: usize = 4;

/// TWS adds this to the message id of a protobuf-encoded frame. Same story:
/// `messages::PROTOBUF_MSG_ID` is not public.
const PROTOBUF_MSG_ID: i32 = 200;

fn main() -> ExitCode {
    let mut path = None;
    let mut list_frames = false;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--frames" => list_frames = true,
            "-h" | "--help" => {
                eprintln!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => path = Some(other.to_string()),
        }
    }

    let Some(path) = path else {
        eprintln!("{USAGE}");
        eprintln!("       capture files are written by setting IBAPI_RAW_CAPTURE_DIR");
        return ExitCode::FAILURE;
    };

    let capture = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("cannot read {path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    println!("{path}: {} bytes", capture.len());
    let report = walk(&capture, list_frames);
    report.print(capture.len());

    if report.desync.is_some() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// A frame's identity, as two `Copy` fields rather than a rendered string —
/// captures run to millions of frames, and the label is only ever needed once
/// per distinct kind at print time.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct FrameKind {
    raw_id: i32,
    encoding: &'static str,
}

struct Report {
    frames: usize,
    by_kind: BTreeMap<FrameKind, usize>,
    /// Byte offset and description of the first frame that could not be read.
    /// Everything after it is suspect, so the walk stops there.
    desync: Option<(usize, String)>,
}

impl Report {
    fn print(&self, capture_len: usize) {
        println!("{} frames read", self.frames);
        for (kind, count) in &self.by_kind {
            println!("  {count:>7}  {}", label(kind));
        }
        match &self.desync {
            Some((offset, reason)) => {
                println!();
                println!("DESYNC at byte {offset}: {reason}");
                println!("{} bytes after this point were not walked.", capture_len - offset);
                println!("Cross-reference the `.idx` beside this file for the wall-clock time of that frame.");
            }
            None => println!("no framing anomalies; the capture reads end to end"),
        }
    }
}

fn label(kind: &FrameKind) -> String {
    let resolved = IncomingMessages::from(if kind.encoding == "proto" {
        kind.raw_id - PROTOBUF_MSG_ID
    } else {
        kind.raw_id
    });
    if resolved == IncomingMessages::NotValid {
        // An id that resolved to nothing is the fingerprint of a slip: scattered
        // ids mean the framing moved, one repeated id means TWS grew a message
        // this build does not know.
        format!("NotValid (raw id {})", kind.raw_id)
    } else {
        format!("{resolved:?} [{}]", kind.encoding)
    }
}

fn walk(capture: &[u8], list_frames: bool) -> Report {
    let mut report = Report {
        frames: 0,
        by_kind: BTreeMap::new(),
        desync: None,
    };

    let mut offset = 0usize;
    while offset < capture.len() {
        let Some(prefix) = capture.get(offset..offset + 4) else {
            report.desync = Some((offset, format!("{} trailing bytes, too few for a length prefix", capture.len() - offset)));
            break;
        };
        let declared = u32::from_be_bytes(prefix.try_into().expect("4 bytes")) as usize;

        if !(MIN_FRAME_LENGTH..=MAX_FRAME_LENGTH).contains(&declared) {
            report.desync = Some((
                offset,
                format!("length prefix {declared} is outside {MIN_FRAME_LENGTH}..={MAX_FRAME_LENGTH}"),
            ));
            break;
        }

        let body_start = offset + 4;
        let Some(body) = capture.get(body_start..body_start + declared) else {
            let available = capture.len() - body_start;
            report.desync = Some((offset, format!("length prefix {declared} but only {available} bytes remain")));
            break;
        };

        let raw_id = i32::from_be_bytes(body[..4].try_into().expect("4 bytes"));
        // Protobuf frames carry id + 200; anything else is read as-is.
        let encoding = if raw_id > PROTOBUF_MSG_ID { "proto" } else { "text" };
        let kind = FrameKind { raw_id, encoding };

        if list_frames {
            println!("  #{:<6} offset {offset:<10} len {declared:<8} {}", report.frames, label(&kind));
        }
        *report.by_kind.entry(kind).or_insert(0) += 1;
        report.frames += 1;
        offset = body_start + declared;
    }

    report
}
