//! Common utilities shared between sync and async transport implementations

use std::time::Duration;

use log::{error, info, warn};

use crate::connection::common::NoticeSink;
use crate::errors::Error;
use crate::messages::{
    ConnectivityStatus, IncomingMessages, Notice, ResponseMessage, CONNECTIVITY_RESTORED_DATA_LOST_CODE, CONNECTIVITY_RESTORED_DATA_MAINTAINED_CODE,
    UNKNOWN_MESSAGE_TYPE_CODE,
};
use crate::subscriptions::common::RoutedItem;

/// A notice reports *healthy* data-farm connectivity ("…connection is OK")
/// rather than a problem. IB's message-codes reference classifies these as
/// System Notifications, not warnings, so they're logged at info instead of
/// warn. Only [`ConnectivityStatus::Ok`] is benign — `Broken`/`Inactive`/
/// `Connecting` stay at warn via [`Notice::is_warning`].
///
/// Code 1102 ("connectivity restored — data maintained") is a system message,
/// not a data-farm notice, so it has no [`ConnectivityStatus`]; it is treated
/// as benign here because nothing was lost on the reconnect.
fn is_benign_connectivity_notice(notice: &Notice) -> bool {
    notice.connectivity_status() == Some(ConnectivityStatus::Ok) || notice.code == CONNECTIVITY_RESTORED_DATA_MAINTAINED_CODE
}

/// Log an unrouted notice (no subscription owner) at the appropriate severity.
///
/// System connectivity codes are graded by how much they matter: 1102
/// (restored, data maintained) is benign → info; 1101 (restored, data lost —
/// resubscribe required) is a warning; 1100 (connectivity lost) and everything
/// else fall through to error.
pub(crate) fn log_unrouted_notice(notice: &Notice) {
    if is_benign_connectivity_notice(notice) {
        info!("connectivity: {notice}");
    } else if notice.code == CONNECTIVITY_RESTORED_DATA_LOST_CODE || notice.is_warning() {
        warn!("warning: {notice}");
    } else {
        error!("error: {notice}");
    }
}

/// Log a routed notice/error that arrived bound to an id with no matching
/// request or order channel. The dispatcher only constructs `Notice` and
/// `Error` variants for this path; `Response` is unreachable here.
pub(crate) fn log_orphan(request_id: i32, item: &RoutedItem) {
    match item {
        RoutedItem::Notice(n) => info!("no recipient for notice (id={request_id}): {n}"),
        RoutedItem::Error(e) => info!("no recipient for error (id={request_id}): {e}"),
        RoutedItem::Response(_) => {}
    }
}

/// Report a frame that reached the end of routing with no recipient.
///
/// Two very different situations end up here, and conflating them is what made
/// a desynchronized stream indistinguishable from an idle one:
///
/// - **Unknown message kind** ([`IncomingMessages::NotValid`]) — nothing can
///   ever route this, and it is the shape a framing slip takes. Published to
///   the notice stream as [`UNKNOWN_MESSAGE_TYPE_CODE`] so a consumer can react
///   programmatically rather than by reading logs.
/// - **Known kind, nobody listening** — an ordinary steady-state condition, so
///   it stays at `info` and raises no notice.
///
/// The blocking transport logged both at `info` and the async transport logged
/// neither, which is how the incident in
/// `plans/tick-by-tick-reconnect-decode-desync.md` produced farm notices and no
/// decode error.
pub(crate) fn report_unroutable_frame(message: &ResponseMessage, notice_sink: &dyn NoticeSink) {
    if message.message_type() == IncomingMessages::NotValid {
        // The Debug dump already carries the id; the notice has no such
        // fallback, so it interpolates.
        warn!("unroutable frame: message id maps to no known type — the stream may be desynchronized: {message:?}");
        notice_sink.deliver(Notice::synthesized(
            UNKNOWN_MESSAGE_TYPE_CODE,
            format!(
                "received a frame with message id {}, which maps to no known type; the stream may be desynchronized",
                message.message_id()
            ),
        ));
    } else {
        info!("no recipient found for: {message:?}");
    }
}

/// Default maximum number of reconnection attempts.
///
/// Overridable per client via `ClientBuilder::max_reconnect_attempts` /
/// `ClientBuilder::reconnect_forever`.
pub(crate) const MAX_RECONNECT_ATTEMPTS: u32 = 20;

/// Largest frame body rust-ibapi will accept from a length prefix, matching the
/// official client's `Constants.MaxMsgSize` (`0x00FFFFFF`, ~16 MiB), which
/// `EReader.readSingleMessage` enforces with `BAD_LENGTH`. Nothing in the wire
/// format bounds the 4-byte prefix on its own.
pub(crate) const MAX_FRAME_LENGTH: usize = 0x00FF_FFFF;

/// Smallest valid frame body: every TWS frame is `[4-byte BE msg_id][payload]`,
/// so a body that cannot hold the message id is malformed by definition. An
/// empty payload after the id is legal.
pub(crate) const MIN_FRAME_LENGTH: usize = 4;

/// Reject a length prefix that cannot describe a TWS frame, before it is used
/// to size an allocation or drive a `read_exact`.
///
/// Returns a hard [`Error::InvalidFrame`] rather than skipping the frame,
/// because either direction desynchronizes the stream permanently instead of
/// corrupting one message — see that variant's docs for why.
pub(crate) fn validate_frame_length(length: usize) -> Result<usize, Error> {
    if length > MAX_FRAME_LENGTH {
        return Err(Error::InvalidFrame(format!(
            "frame length {length} exceeds maximum {MAX_FRAME_LENGTH}; the stream is desynchronized"
        )));
    }
    if length < MIN_FRAME_LENGTH {
        return Err(Error::InvalidFrame(format!(
            "frame length {length} is shorter than the {MIN_FRAME_LENGTH}-byte message id; the stream is desynchronized"
        )));
    }
    Ok(length)
}

/// Fibonacci backoff for reconnection attempts
pub(crate) struct FibonacciBackoff {
    previous: u64,
    current: u64,
    max: u64,
}

impl FibonacciBackoff {
    pub(crate) fn new(max: u64) -> Self {
        FibonacciBackoff {
            previous: 0,
            current: 1,
            max,
        }
    }

    pub(crate) fn next_delay(&mut self) -> Duration {
        // Note: `max` must clamp `previous` and `current` (not just the return value)
        // because u64 will overflow at approx fib(94).
        if self.current < self.max {
            let next = (self.previous + self.current).min(self.max);
            self.previous = self.current;
            self.current = next;
        }
        Duration::from_secs(self.current)
    }
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
