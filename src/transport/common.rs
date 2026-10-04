//! Common utilities shared between sync and async transport implementations

use std::sync::{Arc, Weak};
use std::time::Duration;

use log::info;

use crate::client::ids::WireId;
use crate::errors::Error;
use crate::messages::{unknown_message_type_notice, IncomingMessages, Notice, ResponseMessage, MESSAGE_ID_LEN};
use crate::subscriptions::common::RoutedItem;

/// Sink for unrouted notices observed during the handshake. Production impls
/// forward to the per-feature notice broadcaster owned by `Connection`, so
/// handshake-time notices reach any pre-bound `NoticeStream` the user obtained
/// from `ClientBuilder::connect_with_notice_stream`.
pub(crate) trait NoticeSink: Send + Sync {
    fn deliver(&self, notice: Notice);
}

/// Log a routed notice/error that arrived bound to an id with no matching
/// request or order channel. The dispatcher only constructs `Notice` and
/// `Error` variants for this path; `Response` is unreachable here.
pub(crate) fn log_orphan(id: WireId, item: &RoutedItem) {
    match item {
        RoutedItem::Notice(n) => info!("no recipient for notice (id={id}): {n}"),
        RoutedItem::Error(e) => info!("no recipient for error (id={id}): {e}"),
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
///   the notice stream as [`UNKNOWN_MESSAGE_TYPE_CODE`](crate::messages::UNKNOWN_MESSAGE_TYPE_CODE) so a consumer can react
///   programmatically rather than by reading logs.
/// - **Known kind, nobody listening** — an ordinary steady-state condition, so
///   it stays at `info` and raises no notice.
///
/// The blocking transport logged both at `info` and the async transport logged
/// neither, which is how the incident in
/// #891 produced farm notices and no
/// decode error.
pub(crate) fn report_unroutable_frame(message: &ResponseMessage, notice_sink: &dyn NoticeSink) {
    if message.message_type() == IncomingMessages::NotValid {
        notice_sink.deliver(unknown_message_type_notice(message));
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
pub(crate) const MIN_FRAME_LENGTH: usize = MESSAGE_ID_LEN;

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

/// A subscription's claim on its registration: the registration is live while
/// any holder is. Async clones share one lease, so it covers them all.
///
/// A dropping subscription releases its lease *before* sending its cleanup
/// signal, so the cleanup that signal triggers sees the registration dead.
#[derive(Clone, Debug, Default)]
pub(crate) struct Lease(Arc<()>);

impl Lease {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The registration's side of this lease.
    pub(crate) fn downgrade(&self) -> LeaseRef {
        LeaseRef(Arc::downgrade(&self.0))
    }
}

/// Held beside a registration: tells whether its subscription is still live,
/// and identifies which subscription it belongs to, so a late cleanup signal
/// never removes a newer registration under the same key.
#[derive(Clone, Debug)]
pub(crate) struct LeaseRef(Weak<()>);

impl LeaseRef {
    /// Whether any holder of the lease remains.
    pub(crate) fn is_live(&self) -> bool {
        self.0.strong_count() > 0
    }

    /// Whether `other` refers to the same lease.
    pub(crate) fn is(&self, other: &LeaseRef) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
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
            // Clamped so `current <= max` holds from construction on; keeps
            // `max: 0` meaning "no delay" rather than a fixed 1s.
            current: 1.min(max),
            max,
        }
    }

    pub(crate) fn next_delay(&mut self) -> Duration {
        // Note: `max` must clamp `previous` and `current` (not just the return value)
        // because u64 overflows at fib(94). The saturating_add covers `max` values
        // large enough that the sum overflows before the clamp can engage.
        if self.current < self.max {
            let next = self.previous.saturating_add(self.current).min(self.max);
            self.previous = self.current;
            self.current = next;
        }
        Duration::from_secs(self.current)
    }
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
