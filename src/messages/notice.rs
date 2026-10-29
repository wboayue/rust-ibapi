//! TWS notices: the [`Notice`] type, its [`NoticeCategory`] classification,
//! [`ConnectivityStatus`], the notice code constants and ranges, and the
//! builders for client-synthesized notices.

use std::fmt::Display;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::ResponseMessage;

#[cfg(test)]
#[path = "notice_tests.rs"]
mod tests;

/// An error message from the TWS API.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Notice {
    /// Request or order ID that originated the notice.
    /// `None` for request-less notices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<i32>,
    /// Error code reported by TWS.
    pub code: i32,
    /// Human-readable error message text.
    pub message: String,
    /// Timestamp when the error occurred.
    /// Only present for server versions >= ERROR_TIME (194).
    pub error_time: Option<OffsetDateTime>,
    /// Advanced order-reject JSON payload, present on hard order-rejection
    /// notices for server versions >= ADVANCED_ORDER_REJECT. Empty otherwise.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub advanced_order_reject_json: String,
}

/// Error code indicating an order was cancelled (confirmation, not an error).
pub const ORDER_CANCELLED_CODE: i32 = 202;

/// Generic order-message code whose text determines whether TWS reports a warning or an error.
pub const ORDER_MESSAGE_CODE: i32 = 399;

/// "Market depth data has been RESET". A [`DATA_ADVISORY_CODES`] entry; a
/// depth subscription yields it as [`MarketDepths::Reset`](crate::market_data::realtime::MarketDepths::Reset).
pub(crate) const MARKET_DEPTH_RESET_CODE: i32 = 317;

/// "Historical Market Data Service query message". Classified `Error`, but a
/// scanner subscription keeps its "no items retrieved" form nonterminal; see
/// [`classify`].
pub(crate) const HMDS_QUERY_MESSAGE_CODE: i32 = 165;

/// "No security definition has been found for the request".
pub(crate) const NO_SECURITY_DEFINITION_CODE: i32 = 200;

/// "Historical Market Data Service error message" (e.g. a disabled scanner
/// type, or no data for a historical query).
pub(crate) const HMDS_ERROR_CODE: i32 = 162;

/// "Error validating request" (e.g. an invalid account code).
pub(crate) const REQUEST_VALIDATION_CODE: i32 = 321;

/// Range of error codes that are considered warnings: the whole `21xx` band.
///
/// IB's published table stops at 2169, but the gateway keeps adding codes above
/// it (2176, the fractional-share size-rule warning; 2187, generic ticks
/// unavailable on delayed-data fallback), and a ceiling of 2169 turned each new
/// one into a hard error that failed in-flight one-shots and ended
/// subscriptions.
pub const WARNING_CODE_RANGE: std::ops::RangeInclusive<i32> = 2100..=2199;

/// Code 0 is a code-less frame: IB Gateway omits `error_code` on informational
/// notices (e.g. "Warning: Approaching max rate of 50 messages per second"),
/// and the absent field decodes to 0. An error frame that fails proto decode
/// falls back to the same default, so code 0 never carries a hard error.
pub(crate) fn is_warning_message(code: i32, message: &str) -> bool {
    code == 0
        || WARNING_CODE_RANGE.contains(&code)
        || (code == ORDER_MESSAGE_CODE && message.lines().any(|line| line.trim_start().starts_with("Warning:")))
}

/// Classify a raw error frame into a disjoint [`NoticeCategory`].
///
/// The single owner of the precedence chain documented on [`NoticeCategory`];
/// [`Notice::category`] and [`is_informational_code`] both derive from it, so
/// routing and the public partition cannot disagree.
///
/// One exception sits downstream of routing:
/// [`StreamDecoder::is_nonterminal_notice`](crate::subscriptions::common::StreamDecoder::is_nonterminal_notice)
/// lets a subscription deliver a request-bound `Error` notice as
/// `SubscriptionItem::Notice` instead of ending. The notice keeps its `Error`
/// category. Only the scanner uses it, for [`HMDS_QUERY_MESSAGE_CODE`] "no items
/// retrieved" (#886); making 165 informational here would also reach one-shot
/// requests, which would wait for an answer that never comes.
pub(crate) fn classify(code: i32, message: &str) -> NoticeCategory {
    if code == ORDER_CANCELLED_CODE {
        NoticeCategory::Cancellation
    } else if DATA_ADVISORY_CODES.contains(&code) {
        NoticeCategory::DataAdvisory
    } else if is_warning_message(code, message) {
        NoticeCategory::Warning
    } else if SYSTEM_MESSAGE_CODES.contains(&code) {
        NoticeCategory::SystemMessage
    } else if REQUEST_ERROR_CODES.contains(&code) {
        NoticeCategory::RequestError
    } else if ORDER_REJECTION_CODE_RANGE.contains(&code) {
        NoticeCategory::OrderRejection
    } else {
        NoticeCategory::Error
    }
}

/// Check if an error code is informational: every [`NoticeCategory`] except
/// `RequestError`, `OrderRejection` and `Error`.
///
/// For these TWS proceeds with the request, the frame confirms an outcome the
/// caller asked for, or the frame reports a connection-wide state change
/// rather than a failed request — so they are routed as a `Notice` rather
/// than terminating the subscription as an `Error`, and, when request-less,
/// they do not fail the pending one-shots.
///
/// System messages never stand in for a per-request answer: after 1100 TWS
/// still answers or rejects each request itself, and after 1300 the socket is
/// dropped, so pending one-shots see `Error::ConnectionReset` from the
/// transport reset and retry. Note 202 and the system codes are deliberately
/// *not* in [`is_warning_message`]: `Notice::is_warning()` stays false for
/// them; only the routing disposition treats them like warnings.
pub(crate) fn is_informational_code(code: i32, message: &str) -> bool {
    // Exhaustive on purpose: a new `NoticeCategory` variant must decide its
    // routing disposition here, not inherit one from a wildcard.
    match classify(code, message) {
        NoticeCategory::Cancellation | NoticeCategory::DataAdvisory | NoticeCategory::Warning | NoticeCategory::SystemMessage => true,
        NoticeCategory::RequestError | NoticeCategory::OrderRejection | NoticeCategory::Error => false,
    }
}

/// Connectivity between IB and TWS has been lost.
pub(crate) const CONNECTIVITY_LOST_CODE: i32 = 1100;
/// Connectivity restored, but market data was lost; resubscription is required.
pub(crate) const CONNECTIVITY_RESTORED_DATA_LOST_CODE: i32 = 1101;
/// Connectivity restored with market data maintained (nothing lost).
pub(crate) const CONNECTIVITY_RESTORED_DATA_MAINTAINED_CODE: i32 = 1102;
/// Socket port was reset during an active connection; the connection is dropped.
pub(crate) const SOCKET_PORT_RESET_CODE: i32 = 1300;

/// System message codes indicating connectivity status.
/// - 1100: Connectivity lost
/// - 1101: Connectivity restored, market data lost (resubscribe needed)
/// - 1102: Connectivity restored, market data maintained
/// - 1300: Socket port reset during active connection
pub const SYSTEM_MESSAGE_CODES: [i32; 4] = [
    CONNECTIVITY_LOST_CODE,
    CONNECTIVITY_RESTORED_DATA_LOST_CODE,
    CONNECTIVITY_RESTORED_DATA_MAINTAINED_CODE,
    SOCKET_PORT_RESET_CODE,
];

/// Data-advisory codes: frames that look like errors but reject nothing — the
/// request stands and data keeps flowing. Each one describes what will arrive
/// (a fallback, a partial entitlement, or a reset of what was already
/// delivered), so they are informational notices, not errors. Classifying them
/// as errors would terminate the subscription before its data arrives.
/// - 317: Market depth data has been RESET. Please empty deep book contents
///   before applying any new entries. A depth subscription yields it as
///   [`MarketDepths::Reset`](crate::market_data::realtime::MarketDepths::Reset).
/// - 2188: Up-to-the-second historical data requires additional subscription for the API.
/// - 10089: Requested market data requires additional subscription for API; delayed market data is available.
/// - 10090: Part of requested market data is not subscribed. Subscription-independent ticks are still active.
/// - 10091: Part of requested market data requires additional subscription for API.
/// - 10167: Requested market data is not subscribed. Displaying delayed market data.
///
/// A slice rather than an array so that adding a code is not a type change
/// for callers that bind the constant explicitly.
pub const DATA_ADVISORY_CODES: &[i32] = &[MARKET_DEPTH_RESET_CODE, 2188, 10089, 10090, 10091, 10167];

/// Request-error codes inside [`ORDER_REJECTION_CODE_RANGE`]: a request other
/// than an order failed (market data, depth, historical data, scanner, session
/// setup). Classified [`NoticeCategory::RequestError`], ahead of the band.
///
/// Only codes that never answer an order are listed. Codes that answer both
/// orders and other requests stay [`NoticeCategory::OrderRejection`], so
/// [`Notice::is_order_rejection`] keeps matching them on order streams: 200 (no
/// security definition) and 320-323 (server error reading, validating or
/// processing a request; 320 answers an invalid attached order, #842).
/// - 300: Can't find EId with ticker Id (cancelling unknown market data).
/// - 301, 302: Invalid ticker action; error parsing stop ticker string.
/// - 309: Max number of market depth requests has been reached.
/// - 310: Can't find the subscribed market depth.
/// - 316: Market depth data has been HALTED. Please re-subscribe.
/// - 319: Invalid log level.
/// - 326: Client id already in use.
/// - 327: Only clientId 0 can set the auto bind TWS orders property.
/// - 330, 331: Managed accounts list needs an FA or STL account with managed accounts.
/// - 354: Not subscribed to requested market data.
/// - 357: Client version out of date.
/// - 365, 366: No scanner subscription / historical data query for the ticker id.
/// - 385, 386: Duplicate ticker id for scanner subscription / historical data query.
///
/// A slice rather than an array so that adding a code is not a type change
/// for callers that bind the constant explicitly.
pub const REQUEST_ERROR_CODES: &[i32] = &[300, 301, 302, 309, 310, 316, 319, 326, 327, 330, 331, 354, 357, 365, 366, 385, 386];

/// TWS refused the session: another connection holds this client id. Sent
/// once after `start_api`, then TWS closes the socket; the handshake turns it
/// into [`Error::ClientIdInUse`](crate::Error::ClientIdInUse).
pub(crate) const CLIENT_ID_IN_USE_CODE: i32 = 326;

/// Data-farm codes reporting a healthy connection ("…connection is OK").
/// Subset of [`WARNING_CODE_RANGE`]; classified [`ConnectivityStatus::Ok`].
pub(crate) const FARM_OK_CODES: [i32; 3] = [2104, 2106, 2158];

/// Data-farm codes reporting a broken connection ("…connection is broken").
/// Subset of [`WARNING_CODE_RANGE`]; classified [`ConnectivityStatus::Broken`].
pub(crate) const FARM_BROKEN_CODES: [i32; 3] = [2103, 2105, 2157];

/// Data-farm codes reporting a dormant-but-available connection
/// ("…inactive but should be available upon demand").
/// Subset of [`WARNING_CODE_RANGE`]; classified [`ConnectivityStatus::Inactive`].
pub(crate) const FARM_INACTIVE_CODES: [i32; 2] = [2107, 2108];

/// Data-farm codes reporting a connection in progress ("…farm is connecting").
/// Subset of [`WARNING_CODE_RANGE`]; classified [`ConnectivityStatus::Connecting`].
pub(crate) const FARM_CONNECTING_CODES: [i32; 1] = [2119];

/// The order-rejection band (200-399): order parameter validation, margin and
/// risk-check rejections.
///
/// [`Notice::category`] classifies a code in this band as
/// [`NoticeCategory::OrderRejection`] unless an earlier rule claims it; see
/// [`NoticeCategory`] for the precedence chain.
pub const ORDER_REJECTION_CODE_RANGE: std::ops::RangeInclusive<i32> = 200..=399;

/// Synthesized notice code emitted when a handshake-time frame's recognized
/// [`IncomingMessages`](super::IncomingMessages) kind has no typed `StartupMessage` variant. A frame
/// whose id maps to no kind at all raises [`UNKNOWN_MESSAGE_TYPE_CODE`]
/// instead, as it does after the handshake. Negative
/// (TWS uses 0+); the other client-synthesized codes are
/// [`HANDSHAKE_DECODE_FAILURE_CODE`], [`UNKNOWN_MESSAGE_TYPE_CODE`],
/// [`SUBSCRIPTION_LAG_CODE`], [`NOTICE_STREAM_LAG_CODE`], and
/// [`TRANSPORT_RECONNECT_CODE`]. See
/// [`Notice::is_handshake_synthetic`].
pub const HANDSHAKE_UNKNOWN_FRAME_CODE: i32 = -3;

/// Synthesized notice code emitted when a typed handshake decoder fails for
/// a known [`IncomingMessages`](super::IncomingMessages) kind (`OpenOrder`, `OrderStatus`,
/// `AccountValue`/`PortfolioValue`/`AccountUpdateTime`/`AccountDownloadEnd`,
/// `ExecutionData`, `CommissionsReport`, `CompletedOrder`). Distinct from
/// [`HANDSHAKE_UNKNOWN_FRAME_CODE`] so consumers can separate TWS schema
/// drift from rust-ibapi decoder bugs. See
/// [`Notice::is_handshake_synthetic`].
pub const HANDSHAKE_DECODE_FAILURE_CODE: i32 = -4;

/// Synthesized notice code emitted when a frame arrives whose 4-byte message
/// id maps to no known [`IncomingMessages`](super::IncomingMessages) kind, so nothing can route it.
///
/// Negative, like the other client-synthesized notice codes
/// ([`HANDSHAKE_UNKNOWN_FRAME_CODE`], [`HANDSHAKE_DECODE_FAILURE_CODE`],
/// [`SUBSCRIPTION_LAG_CODE`], [`NOTICE_STREAM_LAG_CODE`],
/// [`TRANSPORT_RECONNECT_CODE`]); TWS itself only
/// uses codes 0 and up.
///
/// This is the observable form of a framing desynchronization: the length
/// prefix is positional, so once a read starts at the wrong offset every
/// subsequent message id is garbage — usually unrecognized (this notice), but
/// occasionally colliding with a real kind, in which case the payload decodes
/// without error into plausible-looking wrong values.
///
/// The notice text names the offending id, which is what separates the two
/// explanations: a slipped stream yields *scattered* ids, while a message type
/// IBKR has added repeats one. A burst of distinct ids on a previously healthy
/// connection means the framing slipped.
///
/// Raised the same way during the connection handshake, which is the window a
/// reconnect runs through; [`HANDSHAKE_UNKNOWN_FRAME_CODE`] is reserved for
/// recognized kinds with no typed startup variant.
pub const UNKNOWN_MESSAGE_TYPE_CODE: i32 = -5;

/// Build the [`UNKNOWN_MESSAGE_TYPE_CODE`] notice for `message`, and emit the
/// matching `warn!`. The single owner of the wording, shared by steady-state
/// routing and the handshake.
pub(crate) fn unknown_message_type_notice(message: &ResponseMessage) -> Notice {
    // The Debug dump already carries the id; the notice has no such fallback,
    // so it interpolates.
    log::warn!("unroutable frame: message id maps to no known type — the stream may be desynchronized: {message:?}");
    let id = match message.message_id() {
        Some(id) => format!("message id {id}"),
        None => "no message id".to_string(),
    };
    Notice::synthesized(
        UNKNOWN_MESSAGE_TYPE_CODE,
        format!("received a frame with {id}, which maps to no known type; the stream may be desynchronized"),
    )
}

/// Synthesized notice code emitted in-band on an async subscription whose
/// consumer fell behind its broadcast channel: the channel evicted the oldest
/// frames, and this notice — carrying the dropped count in its message — is
/// what the consumer sees in their place.
///
/// Negative, like the other client-synthesized notice codes; TWS itself only
/// uses codes 0 and up.
///
/// On receiving this notice the stream is still live, but frames are gone:
/// reconcile the same way as after a reconnect gap.
///
/// Readers that assemble a complete result fail instead of returning rows
/// with a gap: `collect_to_end_within`, `next_batch` and what is built on
/// them (`contract_details`, account-summary snapshots) return
/// `Error::Notice` carrying this code. `collect_for` and `collect_until`
/// stop at the gap and return the rows read before it.
///
/// - **Market data and other non-order streams** (ticks, bars, depth,
///   positions, account updates, PnL, news, contract details): bounded and
///   lossy by design — freshness beats completeness, and a feed
///   self-corrects with the next tick. Channels hold
///   `ClientBuilder::channel_capacity` frames; a consumer that lags
///   persistently should raise it or consume faster.
/// - **Orders** (`place_order`, `cancel_order`, `exercise_options`,
///   `executions`, `order_update_stream`, the open/completed-order streams):
///   `channel_capacity` can raise these channels but not shrink them, so
///   this notice there means order state was lost and the client logs it as
///   an error. Resynchronize with `open_orders()` / `executions()` before
///   acting on recorded state.
///
/// The sync client never drops: its queues are unbounded and log a warning
/// at every 10,000 unread messages instead. Both transports surface falling
/// behind; only async trades completeness for bounded memory.
pub const SUBSCRIPTION_LAG_CODE: i32 = -6;

/// Build the in-band notice for a subscription that fell behind its broadcast
/// channel and had `skipped` frames evicted, and emit the matching `warn!`.
/// The single owner of the non-order lag wording; see [`SUBSCRIPTION_LAG_CODE`].
#[cfg(feature = "async")]
pub(crate) fn subscription_lag_notice(skipped: u64) -> Notice {
    let message = format!("subscription fell behind; {skipped} frames dropped (consumer lagged broadcast channel)");
    log::warn!("{message}");
    Notice::synthesized(SUBSCRIPTION_LAG_CODE, message)
}

/// [`subscription_lag_notice`] for an order-class channel, where a gap means
/// lost order state: logged as an error, and the message says to resync.
/// The single owner of the order lag wording.
#[cfg(feature = "async")]
pub(crate) fn order_lag_notice(skipped: u64) -> Notice {
    let message = format!("order stream fell behind; {skipped} frames dropped — order state unknown, resync with open_orders() / executions()");
    log::error!("{message}");
    Notice::synthesized(SUBSCRIPTION_LAG_CODE, message)
}

/// Synthesized notice code delivered in-band on the notice stream when its
/// consumer fell behind the notice fan-out: the broadcast channel evicted the
/// oldest notices, and this notice — carrying the dropped count in its
/// message — is what the consumer sees in their place.
///
/// Negative, like the other client-synthesized notice codes; TWS itself only
/// uses codes 0 and up.
///
/// The notice stream is how a stateful consumer receives the unrouted
/// connection-status notices (1100 lost, 1101/1102 restored) it derives
/// durable conclusions from, so a silent skip is not survivable: losing a
/// 1101 voids every market-data request server-side yet leaves the client's
/// subscriptions looking healthy forever, and losing a restoration notice
/// after a recorded 1100 holds every pending reopen forever. On receiving
/// this notice the stream's conclusions are unknown — any of them may rest
/// on notices that were dropped — so the consumer must resynchronize rather
/// than resume: treat recorded state as describing an unknown moment between
/// the eviction and now, and re-derive it (a connection-state authority
/// re-baselines its link state and re-establishes subscriptions). This is
/// the notice-stream instance of [`SUBSCRIPTION_LAG_CODE`], closing the
/// step-1 leftover of #779. The sync
/// notice fan-out is unbounded and cannot lag; it logs a warning at every
/// 10,000 unread notices instead.
pub const NOTICE_STREAM_LAG_CODE: i32 = -7;

/// Build the in-band notice delivered when the notice stream's consumer fell
/// behind the notice fan-out and `skipped` notices were evicted, and emit the
/// matching `warn!`. The single owner of the notice-stream lag wording; see
/// [`NOTICE_STREAM_LAG_CODE`].
#[cfg(feature = "async")]
pub(crate) fn notice_stream_lag_notice(skipped: u64) -> Notice {
    let message = format!("notice stream fell behind; {skipped} notices dropped (consumer lagged the notice fan-out)");
    log::warn!("{message}");
    Notice::synthesized(NOTICE_STREAM_LAG_CODE, message)
}

/// Synthesized notice code published to the notice stream (sync and async)
/// after the transport finishes reconnecting its socket to TWS/Gateway: a new
/// connection whose server-side state starts empty, so every request
/// subscription made on the previous connection is gone, and unrouted-notice
/// state — notably the 1100/1101/1102 connection-status sequence — describes
/// the previous connection only.
///
/// Negative, like the other client-synthesized notice codes; TWS itself only
/// uses codes 0 and up.
///
/// TWS never frames this event itself: 1101/1102 announce the restoration
/// *transition* to clients connected at that moment and are not replayed to a
/// client that connects afterwards, so a consumer that recorded 1100 ("TWS
/// lost its IB-server link") before a socket drop cannot learn from the new
/// connection that the link is back. On receiving this notice, treat every
/// fact gathered from the previous connection as describing that connection
/// alone: re-establish what the new session must provide (resubscribe
/// requests), and reset connection-state conclusions — a recorded 1100 — to
/// the fresh-connection baseline a new client starts from, then let the new
/// connection's own notices re-derive the link state. The market data type
/// set with `switch_market_data_type` needs no action: the reconnect re-sends
/// it before this notice.
///
/// `Client::resubscribing` also delivers it in-band, as a
/// [`SubscriptionItem::Notice`](crate::subscriptions::SubscriptionItem::Notice) before the first item
/// of each subscription it reopens.
///
/// Published once the reconnected session is live: the channel reset runs
/// before the reconnect, so a consumer may resubscribe from inside its handler
/// and land on the new session without racing either. Like the other
/// client-synthesized codes this classifies as [`NoticeCategory::Error`]
/// ("everything else"); consumers match the constant itself rather than the
/// category.
pub const TRANSPORT_RECONNECT_CODE: i32 = -8;

/// Build the [`TRANSPORT_RECONNECT_CODE`] notice. Emits no log line: the
/// transport already logs the reconnect. The single owner of the wording.
pub(crate) fn transport_reconnect_notice() -> Notice {
    Notice::synthesized(
        TRANSPORT_RECONNECT_CODE,
        "transport reconnected to TWS/Gateway; this is a new connection whose server-side state starts empty, so every request subscription from the previous connection is gone".to_string(),
    )
}

/// Build the in-band [`TRANSPORT_RECONNECT_CODE`] notice `Client::resubscribing`
/// yields before a resubscribed stream's first item.
pub(crate) fn resubscribed_notice() -> Notice {
    Notice::synthesized(
        TRANSPORT_RECONNECT_CODE,
        "resubscribed after a transport reconnect; data from the outage is missing and the new subscription may repeat its initial state".to_string(),
    )
}

/// Typed classification of a [`Notice`] by TWS error-code range.
///
/// Returned by [`Notice::category`]. Forms a disjoint partition over all
/// possible notices; when classifications overlap on the wire, the classifier
/// resolves overlap by **precedence**:
///
/// 1. [`Cancellation`](Self::Cancellation) — exact code 202.
/// 2. [`DataAdvisory`](Self::DataAdvisory) — [`DATA_ADVISORY_CODES`]. Ahead of
///    the ranges: 317 is inside [`ORDER_REJECTION_CODE_RANGE`] and 2188 inside
///    [`WARNING_CODE_RANGE`].
/// 3. [`Warning`](Self::Warning) — [`WARNING_CODE_RANGE`], code 399 with a `Warning:`
///    line, or code 0 (a frame whose `error_code` field was absent on the wire).
/// 4. [`SystemMessage`](Self::SystemMessage) — 1100, 1101, 1102, 1300.
/// 5. [`RequestError`](Self::RequestError) — [`REQUEST_ERROR_CODES`]. Ahead of
///    [`ORDER_REJECTION_CODE_RANGE`], which they sit inside.
/// 6. [`OrderRejection`](Self::OrderRejection) — [`ORDER_REJECTION_CODE_RANGE`],
///    excluding the cases above.
/// 7. [`Error`](Self::Error) — everything else.
///
/// Each `Notice::is_*` category predicate ([`Notice::is_cancellation`],
/// [`Notice::is_warning`], ...) is `category() == X` for its variant `X`, so the
/// predicates are disjoint too.
///
/// Marked `#[non_exhaustive]` so IBKR can introduce new code ranges without a
/// breaking release.
///
/// # Examples
///
/// ```no_run
/// use ibapi::{Notice, NoticeCategory};
/// # let notice: Notice = unimplemented!();
/// match notice.category() {
///     NoticeCategory::OrderRejection => eprintln!("rejected: {}", notice),
///     NoticeCategory::RequestError   => eprintln!("failed: {}",   notice),
///     NoticeCategory::Warning        => eprintln!("warn: {}",     notice),
///     NoticeCategory::Error          => eprintln!("error: {}",    notice),
///     _ => {}
/// }
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeCategory {
    /// Order cancellation confirmation (exact code 202).
    Cancellation,
    /// Informational warning ([`WARNING_CODE_RANGE`], code 399 with a `Warning:`
    /// line, or code 0 — a code-less frame).
    Warning,
    /// Connectivity / system status (codes 1100, 1101, 1102, 1300).
    SystemMessage,
    /// Order rejection ([`ORDER_REJECTION_CODE_RANGE`], excluding the cases
    /// above it in the precedence chain).
    OrderRejection,
    /// A request other than an order failed ([`REQUEST_ERROR_CODES`]): market
    /// data, depth, historical data, scanner, session setup. Terminal.
    RequestError,
    /// Data advisory ([`DATA_ADVISORY_CODES`]): the request is not rejected and
    /// data follows — a fallback, a partial entitlement, or a depth-book reset.
    /// Informational.
    DataAdvisory,
    /// Any other error code.
    Error,
}

/// Connectivity sub-state of a data-farm notice within [`WARNING_CODE_RANGE`].
///
/// Returned by [`Notice::connectivity_status`] (and [`ConnectivityStatus::from_code`]
/// for a raw code) for the data-farm status codes; `None` for every other notice.
/// Lets reconnect/health logic tell "farm came back online" from "farm went
/// inactive" without re-parsing codes. Additive to [`NoticeCategory`], which still
/// classifies all of [`WARNING_CODE_RANGE`] as [`NoticeCategory::Warning`]
/// (bar the advisory 2188).
///
/// The code→state vocabulary follows IB's published Message Codes table:
///
/// | State | Codes | Wire meaning |
/// |---|---|---|
/// | [`Ok`](Self::Ok) | 2104, 2106, 2158 | …data farm connection is OK |
/// | [`Broken`](Self::Broken) | 2103, 2105, 2157 | …data farm connection is broken |
/// | [`Inactive`](Self::Inactive) | 2107, 2108 | …connection is inactive but should be available upon demand |
/// | [`Connecting`](Self::Connecting) | 2119 | …data farm is connecting |
///
/// The variant set is closed: an unrecognized code classifies as `None`
/// (via [`ConnectivityStatus::from_code`]), not a new variant — so callers can
/// match exhaustively and handle unknowns through the `Option`.
///
/// # Examples
///
/// ```no_run
/// use ibapi::{Notice, ConnectivityStatus};
/// # let notice: Notice = unimplemented!();
/// match notice.connectivity_status() {
///     Some(ConnectivityStatus::Ok)         => {/* farm healthy */}
///     Some(ConnectivityStatus::Broken)     => {/* link down */}
///     Some(ConnectivityStatus::Inactive)   => {/* dormant, available on demand */}
///     Some(ConnectivityStatus::Connecting) => {/* reconnecting */}
///     None => {/* not a data-farm notice */}
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityStatus {
    /// Data-farm connection is OK.
    Ok,
    /// Data-farm connection is broken.
    Broken,
    /// Connection inactive but available upon demand.
    Inactive,
    /// Connection is in the process of connecting.
    Connecting,
}

impl ConnectivityStatus {
    /// Classify a raw TWS error code into a data-farm connectivity sub-state.
    ///
    /// Returns `Some(..)` for the data-farm status codes inside
    /// [`WARNING_CODE_RANGE`]; `None` for every other code. Use this when you
    /// hold a raw code; use [`Notice::connectivity_status`] when you hold a
    /// [`Notice`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::ConnectivityStatus;
    /// assert_eq!(ConnectivityStatus::from_code(2104), Some(ConnectivityStatus::Ok));
    /// assert_eq!(ConnectivityStatus::from_code(2105), Some(ConnectivityStatus::Broken));
    /// assert_eq!(ConnectivityStatus::from_code(500), None);
    /// ```
    pub fn from_code(code: i32) -> Option<ConnectivityStatus> {
        if FARM_OK_CODES.contains(&code) {
            Some(ConnectivityStatus::Ok)
        } else if FARM_BROKEN_CODES.contains(&code) {
            Some(ConnectivityStatus::Broken)
        } else if FARM_INACTIVE_CODES.contains(&code) {
            Some(ConnectivityStatus::Inactive)
        } else if FARM_CONNECTING_CODES.contains(&code) {
            Some(ConnectivityStatus::Connecting)
        } else {
            None
        }
    }
}

impl From<&ResponseMessage> for Notice {
    /// Build a Notice from a protobuf Error frame; at floor 213 every Error
    /// payload arrives proto-encoded. Returns `Notice::from(DecodedError::default())`
    /// (empty / code 0) if the proto bytes are absent or undecodable.
    fn from(message: &ResponseMessage) -> Notice {
        let payload = message
            .raw_bytes()
            .and_then(crate::transport::routing::decode_error_envelope)
            .unwrap_or_default();
        Notice::from(payload)
    }
}

impl Notice {
    /// Build a client-synthesized notice with no wire timestamp and no
    /// advanced-order-reject JSON. Used by the client-side observability
    /// codes (see [`HANDSHAKE_UNKNOWN_FRAME_CODE`],
    /// [`HANDSHAKE_DECODE_FAILURE_CODE`], [`UNKNOWN_MESSAGE_TYPE_CODE`],
    /// [`SUBSCRIPTION_LAG_CODE`], [`NOTICE_STREAM_LAG_CODE`],
    /// [`TRANSPORT_RECONNECT_CODE`]).
    pub(crate) fn synthesized(code: i32, message: String) -> Notice {
        Notice {
            request_id: None,
            code,
            message,
            error_time: None,
            advanced_order_reject_json: String::new(),
        }
    }

    /// Returns `true` if this notice indicates an order was cancelled (code 202).
    ///
    /// Code 202 is sent by TWS to confirm an order cancellation. This is an
    /// informational message, not an error.
    pub fn is_cancellation(&self) -> bool {
        self.category() == NoticeCategory::Cancellation
    }

    /// Returns `true` if this is a warning message.
    ///
    /// Warnings are [`WARNING_CODE_RANGE`], code 399 with a `Warning:` line, and
    /// code 0 — a frame whose `error_code` field was absent on the wire, which
    /// IB Gateway sends for informational notices. Same as
    /// `category() == NoticeCategory::Warning`.
    pub fn is_warning(&self) -> bool {
        self.category() == NoticeCategory::Warning
    }

    /// Returns `true` if this is a system/connectivity message (codes 1100-1102, 1300).
    ///
    /// System messages indicate connectivity status changes:
    /// - 1100: Connectivity between IB and TWS lost
    /// - 1101: Connectivity restored, market data lost (resubscribe needed)
    /// - 1102: Connectivity restored, market data maintained
    /// - 1300: Socket port reset during active connection
    pub fn is_system_message(&self) -> bool {
        self.category() == NoticeCategory::SystemMessage
    }

    /// Returns `true` if this is a data advisory ([`DATA_ADVISORY_CODES`]).
    ///
    /// Data advisories reject nothing: the request stands and data follows —
    /// a fallback, a partial entitlement, or a depth-book reset. The
    /// subscription stays open and the notice is informational, not an error.
    /// This does not guarantee that every requested field will arrive.
    pub fn is_data_advisory(&self) -> bool {
        self.category() == NoticeCategory::DataAdvisory
    }

    /// Returns `true` if this is an informational notice (not an error).
    ///
    /// Informational notices include cancellation confirmations, warnings,
    /// system/connectivity messages, and data advisories.
    ///
    /// A scanner subscription also yields TWS code 165 "no items retrieved"
    /// (an empty scan) as a `SubscriptionItem::Notice` and stays open, although
    /// that notice is not informational; don't end a scanner loop on
    /// `!is_informational()` alone.
    pub fn is_informational(&self) -> bool {
        is_informational_code(self.code, &self.message)
    }

    /// Returns `true` if this is an error requiring attention.
    ///
    /// Returns `false` for informational messages like cancellation confirmations,
    /// warnings, and system messages.
    pub fn is_error(&self) -> bool {
        !self.is_informational()
    }

    /// Returns `true` if this notice is an order rejection
    /// ([`NoticeCategory::OrderRejection`]).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Notice;
    /// # let notice: Notice = unimplemented!();
    /// if notice.is_order_rejection() {
    ///     eprintln!("rejection: {}", notice);
    /// }
    /// ```
    pub fn is_order_rejection(&self) -> bool {
        self.category() == NoticeCategory::OrderRejection
    }

    /// Returns `true` if a request other than an order failed
    /// ([`REQUEST_ERROR_CODES`], [`NoticeCategory::RequestError`]).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Notice;
    /// # let notice: Notice = unimplemented!();
    /// if notice.is_request_error() {
    ///     eprintln!("request failed: {}", notice);
    /// }
    /// ```
    pub fn is_request_error(&self) -> bool {
        self.category() == NoticeCategory::RequestError
    }

    /// Returns `true` if this notice was synthesized client-side during the
    /// connection handshake — i.e. carries either
    /// [`HANDSHAKE_UNKNOWN_FRAME_CODE`] (a recognized `IncomingMessages` kind with no
    /// typed `StartupMessage` variant) or [`HANDSHAKE_DECODE_FAILURE_CODE`]
    /// (a typed decoder failed on a known kind).
    ///
    /// Subscribers to [`Client::notice_stream`](crate::Client::notice_stream)
    /// can use this to log + investigate without conflating with TWS-emitted
    /// notices.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Notice;
    /// # let notice: Notice = unimplemented!();
    /// if notice.is_handshake_synthetic() {
    ///     eprintln!("handshake observability: code={} {}", notice.code, notice);
    /// }
    /// ```
    pub fn is_handshake_synthetic(&self) -> bool {
        self.code == HANDSHAKE_UNKNOWN_FRAME_CODE || self.code == HANDSHAKE_DECODE_FAILURE_CODE
    }

    /// Returns `true` if rust-ibapi synthesized this notice rather than
    /// receiving it from TWS: any negative code. TWS uses codes 0 and up.
    ///
    /// Covers [`HANDSHAKE_UNKNOWN_FRAME_CODE`], [`HANDSHAKE_DECODE_FAILURE_CODE`],
    /// [`UNKNOWN_MESSAGE_TYPE_CODE`], [`SUBSCRIPTION_LAG_CODE`],
    /// [`NOTICE_STREAM_LAG_CODE`] and [`TRANSPORT_RECONNECT_CODE`], and any
    /// code added later. These all classify as [`NoticeCategory::Error`], so
    /// use this predicate or the constants, not [`Notice::category`], to tell
    /// them apart from TWS errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Notice;
    /// # let notice: Notice = unimplemented!();
    /// if notice.is_client_synthesized() {
    ///     eprintln!("client-side condition: code={} {}", notice.code, notice);
    /// }
    /// ```
    pub fn is_client_synthesized(&self) -> bool {
        self.code < 0
    }

    /// Classify this notice into a disjoint [`NoticeCategory`].
    ///
    /// See [`NoticeCategory`] for the precedence chain.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::{Notice, NoticeCategory};
    /// # let notice: Notice = unimplemented!();
    /// let kind = match notice.category() {
    ///     NoticeCategory::Cancellation
    ///     | NoticeCategory::Warning
    ///     | NoticeCategory::DataAdvisory
    ///     | NoticeCategory::SystemMessage => "informational",
    ///     NoticeCategory::RequestError | NoticeCategory::OrderRejection | NoticeCategory::Error => "error",
    ///     _ => "unknown",
    /// };
    /// # let _ = kind;
    /// ```
    pub fn category(&self) -> NoticeCategory {
        classify(self.code, &self.message)
    }

    /// Classify the data-farm connectivity sub-state of this notice.
    ///
    /// Returns `Some(..)` for the data-farm status codes inside
    /// [`WARNING_CODE_RANGE`]; `None` for every other notice. Additive to
    /// [`Notice::category`], which still classifies the farm codes as
    /// [`NoticeCategory::Warning`]. Thin wrapper over
    /// [`ConnectivityStatus::from_code`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::{Notice, ConnectivityStatus};
    /// # let notice: Notice = unimplemented!();
    /// if notice.connectivity_status() == Some(ConnectivityStatus::Broken) {
    ///     eprintln!("data farm down: {notice}");
    /// }
    /// ```
    pub fn connectivity_status(&self) -> Option<ConnectivityStatus> {
        ConnectivityStatus::from_code(self.code)
    }

    /// Log severity for this notice, derived from [`Notice::category`] so that
    /// every code in a category logs alike regardless of which numeric band it
    /// sits in.
    ///
    /// Informational categories log at `warn` (the caller may want to act:
    /// a fallback engaged, a book must be cleared), with two exceptions at `info`:
    /// the cancellation confirmation, and data-farm notices that need no action.
    /// A farm that is OK, `Inactive` ("…available upon demand") or `Connecting` is
    /// routine on nearly every connect; only `Broken` warns.
    /// System connectivity codes are graded by how much they matter: 1102
    /// (restored, data maintained) → info; 1101 (restored, data lost —
    /// resubscribe required) → warn; 1100 (connectivity lost) and 1300 (socket
    /// reset) → error. Request errors, order rejections and errors log at `error`.
    fn log_level(&self) -> log::Level {
        use log::Level;
        match self.category() {
            NoticeCategory::Cancellation => Level::Info,
            NoticeCategory::Warning
                if matches!(
                    self.connectivity_status(),
                    Some(ConnectivityStatus::Ok | ConnectivityStatus::Inactive | ConnectivityStatus::Connecting)
                ) =>
            {
                Level::Info
            }
            NoticeCategory::Warning | NoticeCategory::DataAdvisory => Level::Warn,
            NoticeCategory::SystemMessage => match self.code {
                CONNECTIVITY_RESTORED_DATA_MAINTAINED_CODE => Level::Info,
                CONNECTIVITY_RESTORED_DATA_LOST_CODE => Level::Warn,
                _ => Level::Error,
            },
            NoticeCategory::RequestError | NoticeCategory::OrderRejection | NoticeCategory::Error => Level::Error,
        }
    }

    /// Log this notice at [`Notice::log_level`].
    pub(crate) fn log(&self) {
        log::log!(self.log_level(), "{self}");
    }
}

impl Display for Notice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl From<crate::transport::routing::DecodedError> for Notice {
    /// Build a Notice from a dispatcher-decoded error payload, moving the
    /// `error_message` and `advanced_order_reject_json` strings and converting
    /// `error_time` (millis-since-epoch) to `OffsetDateTime`.
    fn from(payload: crate::transport::routing::DecodedError) -> Notice {
        let error_time = payload
            .error_time
            .and_then(|millis| OffsetDateTime::from_unix_timestamp_nanos(millis as i128 * 1_000_000).ok());
        Notice {
            request_id: (payload.request_id != crate::transport::routing::UNSPECIFIED_REQUEST_ID).then_some(payload.request_id),
            code: payload.error_code,
            message: payload.error_message,
            error_time,
            advanced_order_reject_json: payload.advanced_order_reject_json,
        }
    }
}
