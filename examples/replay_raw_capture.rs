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

/// The transport's own bounds, restated so a capture can be inspected without
/// linking against internals. `MAX` matches the official client's
/// `Constants.MaxMsgSize`; `MIN` is the 4-byte message id every frame carries.
const MAX_FRAME_LENGTH: usize = 0x00FF_FFFF;
const MIN_FRAME_LENGTH: usize = 4;

/// TWS adds this to the message id of a protobuf-encoded frame.
const PROTOBUF_MSG_ID: i32 = 200;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut path = None;
    let mut list_frames = false;
    for arg in args.by_ref() {
        match arg.as_str() {
            "--frames" => list_frames = true,
            "-h" | "--help" => {
                eprintln!("usage: replay_raw_capture [--frames] <capture.bin>");
                return ExitCode::SUCCESS;
            }
            other => path = Some(other.to_string()),
        }
    }

    let Some(path) = path else {
        eprintln!("usage: replay_raw_capture [--frames] <capture.bin>");
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
    report.print();

    if report.desync.is_some() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

struct Report {
    frames: usize,
    by_kind: BTreeMap<String, usize>,
    /// Byte offset and description of the first frame that could not be read.
    /// Everything after it is suspect, so the walk stops there.
    desync: Option<(usize, String)>,
    trailing: usize,
}

impl Report {
    fn print(&self) {
        println!("{} frames read", self.frames);
        for (kind, count) in &self.by_kind {
            println!("  {count:>7}  {kind}");
        }
        match &self.desync {
            Some((offset, reason)) => {
                println!();
                println!("DESYNC at byte {offset}: {reason}");
                println!("{} bytes after this point were not walked.", self.trailing);
                println!("Cross-reference the `.idx` beside this file for the wall-clock time of that frame.");
            }
            None => println!("no framing anomalies; the capture reads end to end"),
        }
    }
}

fn walk(capture: &[u8], list_frames: bool) -> Report {
    let mut report = Report {
        frames: 0,
        by_kind: BTreeMap::new(),
        desync: None,
        trailing: 0,
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
        let (message_id, encoding) = if raw_id > PROTOBUF_MSG_ID {
            (raw_id - PROTOBUF_MSG_ID, "proto")
        } else {
            (raw_id, "text")
        };
        let kind = IncomingMessages::from(message_id);
        let label = if kind == IncomingMessages::NotValid {
            // The id that resolved to nothing is the fingerprint of a slip:
            // scattered ids mean the framing moved, one repeated id means TWS
            // grew a message this build does not know.
            format!("NotValid (raw id {raw_id})")
        } else {
            format!("{kind:?} [{encoding}]")
        };

        if list_frames {
            println!("  #{:<6} offset {offset:<10} len {declared:<8} {label}", report.frames);
        }
        *report.by_kind.entry(label).or_insert(0) += 1;
        report.frames += 1;
        offset = body_start + declared;
    }

    if let Some((desync_offset, _)) = report.desync {
        report.trailing = capture.len() - desync_offset;
    }
    report
}
