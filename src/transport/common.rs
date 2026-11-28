//! Common utilities shared between sync and async transport implementations

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak};
use std::time::Duration;

use log::{info, warn};

use crate::client::ids::{OrderId, RequestId, WireId};
use crate::errors::Error;
use crate::messages::{unknown_message_type_notice, IncomingMessages, Notice, ResponseMessage, MESSAGE_ID_LEN};
use crate::subscriptions::common::RoutedItem;

// Poison recovery. Locks taken through these helpers guard state that each
// critical section leaves whole (single map/vec operations, flag stores), so a
// panic elsewhere cannot break an invariant. Recover the guard rather than
// cascade that panic into every later route, broadcast and teardown.

/// Lock `mutex`, recovering the guard if a panicking holder poisoned it.
pub(crate) fn lock<T: ?Sized>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Read-lock `rwlock`, recovering the guard if a panicking holder poisoned it.
pub(crate) fn read_lock<T: ?Sized>(rwlock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    rwlock.read().unwrap_or_else(PoisonError::into_inner)
}

/// Write-lock `rwlock`, recovering the guard if a panicking holder poisoned it.
pub(crate) fn write_lock<T: ?Sized>(rwlock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    rwlock.write().unwrap_or_else(PoisonError::into_inner)
}

/// Poison a lock the way a panic would: hold the guard `acquire` returns
/// while a scoped thread panics.
#[cfg(test)]
pub(crate) fn poison_with<G>(acquire: impl FnOnce() -> G + Send) {
    std::thread::scope(|scope| {
        let _ = scope
            .spawn(|| {
                let _guard = acquire();
                panic!("poison the lock");
            })
            .join();
    });
}

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

#[derive(Debug, Default)]
pub(crate) struct OrderWrites {
    pending: Mutex<Option<PendingOrderWrite>>,
}

#[derive(Debug)]
struct PendingOrderWrite {
    id: OrderId,
    frames: Vec<OrderFrame>,
}

#[derive(Debug)]
pub(crate) enum OrderFrame {
    Direct(RoutedItem),
    Execution {
        item: RoutedItem,
        execution_id: Option<String>,
        request_id: Option<RequestId>,
    },
    Commission {
        item: RoutedItem,
        execution_id: String,
    },
}

impl OrderFrame {
    /// Dispatches a frame after the pending-write gate, leaving the registry
    /// and lock ordering to the runtime's delivery callbacks.
    pub(crate) fn deliver(
        self,
        order: impl FnOnce(RoutedItem, Option<&String>) -> Result<(), RoutedItem>,
        commission: impl FnOnce(&String, &RoutedItem) -> bool,
    ) -> Result<(), RoutedItem> {
        match self {
            Self::Direct(item) => order(item, None),
            Self::Execution {
                item,
                execution_id,
                request_id,
            } => {
                debug_assert!(request_id.is_none());
                order(item, execution_id.as_ref())
            }
            Self::Commission { item, execution_id } => {
                if commission(&execution_id, &item) {
                    Ok(())
                } else {
                    Err(item)
                }
            }
        }
    }

    pub(crate) fn for_replay(self, uncertain: bool) -> Self {
        match self {
            Self::Direct(item) if uncertain => Self::Direct(item.for_earlier_subscription()),
            frame => frame,
        }
    }
}

/// Whether a commission recipient is attached to the order being written.
pub(crate) fn order_has_lease(order_leases: &[LeaseRef], lease: &LeaseRef) -> bool {
    order_leases.iter().any(|candidate| candidate.is(lease))
}

/// Capacity floor for order-class request routes: one per order
/// (`place_order`, `cancel_order`, `exercise_options`) and `executions`.
/// Their traffic is per order or per request and ends, so this floor only
/// keeps a small `ClientBuilder::channel_capacity` from shrinking them; see #896.
pub(crate) const ORDER_CAPACITY: usize = 1024;

// Leave slots for the overflow and reset errors.
const ORDER_WRITE_BUFFER_LIMIT: usize = ORDER_CAPACITY - 2;

impl OrderWrites {
    pub(crate) fn begin<F>(&self, id: OrderId, connected: impl FnOnce() -> Result<(), Error>, finish: F) -> Result<OrderWrite<'_, F>, Error>
    where
        F: FnOnce(bool, OrderId, Vec<OrderFrame>),
    {
        let mut pending = lock(&self.pending);
        connected()?;
        debug_assert!(pending.is_none());
        *pending = Some(PendingOrderWrite { id, frames: Vec::new() });
        Ok(OrderWrite {
            writes: self,
            finish: Some(finish),
        })
    }

    // Never hold this lock over I/O. The dispatcher must be able to read
    // the disconnect that breaks a blocked writer.
    pub(crate) fn deliver(
        &self,
        id: OrderId,
        frame: OrderFrame,
        deliver: impl FnOnce(OrderFrame) -> Result<(), RoutedItem>,
    ) -> Result<(), RoutedItem> {
        let mut writes = lock(&self.pending);
        if let Some(pending) = writes.as_mut().filter(|pending| pending.id == id) {
            pending.push(frame);
            Ok(())
        } else {
            deliver(frame)
        }
    }

    pub(crate) fn deliver_commission(
        &self,
        execution_id: String,
        item: RoutedItem,
        deliver: impl FnOnce(Option<OrderId>, &String, &RoutedItem) -> (bool, bool),
    ) -> Result<(), RoutedItem> {
        let mut writes = lock(&self.pending);
        let (delivered, skipped) = deliver(writes.as_ref().map(|pending| pending.id), &execution_id, &item);
        if let Some(pending) = writes.as_mut() {
            if skipped || pending.accepts_commission(&execution_id) {
                pending.push(OrderFrame::Commission { item, execution_id });
                return Ok(());
            }
        }

        if delivered {
            Ok(())
        } else {
            Err(item)
        }
    }

    pub(crate) fn drain(&self, deliver: impl FnOnce(OrderId, Vec<OrderFrame>)) -> bool {
        let mut pending = lock(&self.pending);
        let Some(write) = pending.take() else {
            return false;
        };
        // New deliveries cannot pass frames still being replayed.
        deliver(write.id, write.frames);
        true
    }
}

impl PendingOrderWrite {
    fn push(&mut self, frame: OrderFrame) {
        match self.frames.len() {
            ORDER_WRITE_BUFFER_LIMIT => self.frames.push(OrderFrame::Direct(
                Error::BufferLimitExceeded {
                    limit: ORDER_WRITE_BUFFER_LIMIT,
                }
                .into(),
            )),
            count if count < ORDER_WRITE_BUFFER_LIMIT => self.frames.push(frame),
            _ => {}
        }
    }

    fn accepts_commission(&self, id: &str) -> bool {
        self.frames
            .iter()
            .any(|frame| matches!(frame, OrderFrame::Execution { execution_id: Some(execution_id), .. } if execution_id == id))
    }
}

pub(crate) struct OrderWrite<'a, F: FnOnce(bool, OrderId, Vec<OrderFrame>)> {
    writes: &'a OrderWrites,
    finish: Option<F>,
}

impl<F: FnOnce(bool, OrderId, Vec<OrderFrame>)> OrderWrite<'_, F> {
    pub(crate) fn commit(mut self) -> bool {
        self.finish(true)
    }

    fn finish(&mut self, sent: bool) -> bool {
        let Some(finish) = self.finish.take() else {
            return false;
        };
        self.writes.drain(|id, frames| finish(sent, id, frames))
    }
}

impl<F: FnOnce(bool, OrderId, Vec<OrderFrame>)> Drop for OrderWrite<'_, F> {
    fn drop(&mut self) {
        self.finish(false);
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

/// Session state behind both buses' connection signals.
///
/// `Disconnected` while a reconnect may still bring the session back;
/// `ShutDown` and `Failed` (the reconnect gave up, or the dispatcher died)
/// are terminal and latch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionState {
    Connected,
    Disconnected,
    ShutDown,
    Failed,
}

impl SessionState {
    /// Neither a reconnect nor a later shutdown moves the session out of a
    /// terminal state.
    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, SessionState::ShutDown | SessionState::Failed)
    }

    /// The send gate's verdict: live, reconnecting, or gone for good.
    pub(crate) fn check(self) -> Result<(), Error> {
        match self {
            SessionState::Connected => Ok(()),
            SessionState::Disconnected => Err(Error::Reconnecting),
            SessionState::ShutDown => Err(Error::Shutdown),
            SessionState::Failed => Err(Error::ConnectionFailed),
        }
    }

    /// Like [`check`](Self::check), but `Ok` while reconnecting: for streams
    /// that write nothing and may be opened mid-reconnect.
    pub(crate) fn check_ended(self) -> Result<(), Error> {
        if self.is_terminal() {
            self.check()
        } else {
            Ok(())
        }
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

pub(crate) fn commission_version(message: &ResponseMessage) -> Option<(String, Vec<u8>)> {
    let mut report = message.proto::<crate::proto::CommissionAndFeesReport>().ok()?;
    let id = report.exec_id.take().filter(|id| !id.is_empty())?;
    Some((id, prost::Message::encode_to_vec(&report)))
}

pub(crate) fn record_commission(previous: &mut Option<Vec<u8>>, payload: &[u8]) -> bool {
    if previous.as_deref() == Some(payload) {
        return false;
    }
    *previous = Some(payload.to_vec());
    true
}

/// How many live executions can retain commission routing. Past it the
/// oldest execution is dropped.
pub(crate) const LIVE_EXECUTIONS_CAPACITY: usize = 10_000;

/// Commission routing for execution ids observed on this client's live stream.
///
/// A report carries an execution id but no request or client id. An overlapping
/// snapshot can repeat or update a commission for a known live fill.
#[derive(Debug, Default)]
pub(crate) struct LiveExecutions {
    state: Mutex<LiveExecutionsState>,
}

#[derive(Debug, Default)]
struct LiveExecutionsState {
    ids: HashMap<String, Option<Vec<u8>>>,
    /// `ids` in arrival order, oldest first, for eviction.
    order: VecDeque<String>,
    warned_unmapped: bool,
}

impl LiveExecutions {
    /// Record an execution that can receive live commission updates.
    pub(crate) fn insert(&self, execution_id: &str) {
        let mut state = lock(&self.state);
        if state.ids.contains_key(execution_id) {
            return;
        }
        state.ids.insert(execution_id.to_owned(), None);
        state.order.push_back(execution_id.to_owned());
        if state.order.len() > LIVE_EXECUTIONS_CAPACITY {
            let oldest = state.order.pop_front().expect("oldest execution");
            state.ids.remove(&oldest);
        }
    }

    pub(crate) fn record_commission(&self, execution_id: &str, payload: &[u8]) -> Option<bool> {
        lock(&self.state)
            .ids
            .get_mut(execution_id)
            .map(|previous| record_commission(previous, payload))
    }

    pub(crate) fn warn_unmapped(&self) {
        let mut state = lock(&self.state);
        if !state.warned_unmapped {
            state.warned_unmapped = true;
            warn!("commission report could not be matched to an execution; reconcile with executions()");
        }
    }

    pub(crate) fn clear(&self) {
        *lock(&self.state) = LiveExecutionsState::default();
    }

    #[cfg(test)]
    pub(crate) fn warned_unmapped(&self) -> bool {
        lock(&self.state).warned_unmapped
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        lock(&self.state).ids.len()
    }
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
