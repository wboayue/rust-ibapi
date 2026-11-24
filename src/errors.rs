//! Error types for the IBAPI library.
//!
//! This module defines all error types that can occur during API operations,
//! including I/O errors, parsing errors, and TWS-specific protocol errors.

use thiserror::Error;

use crate::accounts::types::AccountId;
use crate::builder::ValidationError;
use crate::market_data::historical::WhatToShow;
use crate::messages::{Notice, ResponseMessage};

/// The main error type for IBAPI operations.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new error variants
/// in future versions without breaking compatibility.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    // External error types
    /// I/O error from network operations.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Failed to parse time/date string.
    #[error(transparent)]
    ParseTime(#[from] time::error::Parse),

    // IBAPI-specific errors
    /// A value from TWS, or a string passed to a `FromStr`, could not be parsed.
    #[error("parse error: {}", parse_context(*field, value, reason))]
    Parse {
        /// The field that failed, by its protobuf name (`"action"`,
        /// `"contract"`), when the decoder that read it names it. `None` from a
        /// `FromStr` called directly, and where a decoder parses a value without
        /// naming its field.
        field: Option<&'static str>,
        /// The text that failed to parse. Empty when the field was absent.
        value: String,
        /// What was wrong with it.
        reason: String,
    },

    /// The connected server is older than a request or the client needs.
    #[error("server version {required} required, got {actual}: {feature}")]
    ServerVersion {
        /// The minimum server version for `feature`.
        required: i32,
        /// The connected server's version.
        actual: i32,
        /// What needs `required`.
        feature: String,
    },

    /// The call itself is wrong, a caller bug no one recovers from at runtime:
    /// a builder missing a required field (`ContractBuilder` without a symbol,
    /// `OrderBuilder::order_type(..)` without the price that type needs), a
    /// `historical_data` range the client rejects before sending (`.between()`
    /// with an end before the start, `.ending()` on a continuous future), a bad
    /// `ClientBuilder` setting, `buffer_limit(0)`. Fix the code; the message says
    /// what to change. Where a builder tracks a required field or a conflicting
    /// call in its type (`OrderBuilder`, `Contract::spread`, `historical_data`),
    /// a missing or conflicting one is a compile error instead.
    ///
    /// A bad *value* a caller may correct at runtime is [`Error::Validation`].
    #[error("InvalidArgument: {0}")]
    InvalidArgument(String),

    /// A runtime value was rejected before anything was sent: data a caller may
    /// correct, such as a quantity or price, a strike, a date parsed from user or
    /// file input, or a field on a caller-built `Order` / `OrderCancel`
    /// (`ext_operator`). Returned by the order builders' `submit`, by `place_order`,
    /// `submit_order`, `cancel_order_with` and `global_cancel_with`, by option
    /// `build()` and `ContractBuilder::build()` for a bad strike or expiry, and by
    /// `?` on a [`ValidationError`]. Match its variants instead of parsing a message.
    ///
    /// A caller bug (wrong call, missing required builder field) is
    /// [`Error::InvalidArgument`].
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),

    /// The connection to TWS/Gateway is gone for good: the automatic
    /// reconnect used up its attempts, or reading from the socket failed in a
    /// way reconnecting cannot fix. Terminal - every later request, and
    /// `Client::wait_connected`, fails with it. Streams open when the
    /// connection dropped already ended with [`Error::ConnectionReset`].
    #[error("ConnectionFailed")]
    ConnectionFailed,

    /// TWS/Gateway accepted the TCP connection but closed before completing
    /// the handshake — typically a host allow-list mismatch on the gateway.
    /// Payload carries the underlying diagnostic.
    #[error("connection rejected: {0}")]
    ConnectionRejected(String),

    /// TWS refused the session because another connection holds this client
    /// id (TWS error 326). Connect with a different client id, or close the
    /// other connection.
    ///
    /// Returned by `Client::connect`, which doesn't retry. The automatic
    /// reconnect does retry it: after a dropped connection TWS can hold the
    /// old session until it notices the socket is dead, and frees the id
    /// then. If every attempt meets it, the client fails with
    /// [`Error::ConnectionFailed`]; the error log names the cause.
    #[error("client id {client_id} is already in use by another connection")]
    ClientIdInUse {
        /// The refused client id.
        client_id: i32,
    },

    /// IB Gateway sent a timezone name that could not be mapped to an IANA zone.
    #[error("unrecognized IB Gateway timezone {0:?}; register a mapping with `ibapi::register_timezone_alias({0:?}, \"<IANA-name>\")` before connecting, or set `IBAPI_TIMEZONE_ALIASES={0}=<IANA-name>` in the environment. See https://github.com/wboayue/rust-ibapi/blob/main/docs/troubleshooting.md#gateway-time-zone. To request it as a built-in, file an issue at https://github.com/wboayue/rust-ibapi/issues")]
    UnsupportedTimeZone(String),

    /// The connection to TWS/Gateway dropped. Streams and one-shot requests that were open when
    /// it dropped end with this, as does a write it cut off, which may have reached TWS in part
    /// or whole. The client reconnects on its own; a request refused before the write while it
    /// reconnects is [`Error::Reconnecting`] instead.
    #[error("ConnectionReset")]
    ConnectionReset,

    /// A request refused, before anything was written, while the client reconnects to
    /// TWS/Gateway: it certainly did not reach TWS. Call `Client::wait_connected`, then send it
    /// again. Once the reconnect gives up, requests fail with [`Error::ConnectionFailed`] instead.
    #[error("Reconnecting")]
    Reconnecting,

    /// Operation was cancelled by user or system.
    #[error("Cancelled")]
    Cancelled,

    /// Client is shutting down.
    #[error("Shutdown")]
    Shutdown,

    /// Received unexpected message type. The string carries the `Debug` repr
    /// of the offending wire envelope for diagnostic logging; the structured
    /// payload is no longer exposed (rust-ibapi 3.x retired
    /// `ResponseMessage` from the public surface).
    #[error("UnexpectedResponse: {0}")]
    UnexpectedResponse(String),

    /// A message arrived in the wrong wire format for the reader handling it —
    /// text framing at a proto-only decoder, or proto framing at a text-field
    /// accessor. The string carries the `Debug` repr of the offending envelope.
    ///
    /// Deliberately distinct from [`Error::UnexpectedResponse`], which means
    /// "not my message type" and is *skipped* on shared channels. A framing
    /// mismatch is not skippable: the message was addressed to this reader and
    /// could not be read. At `server_versions::PROTOBUF_REST_MESSAGES_3` this is
    /// unreachable in production, so receiving it means the gateway broke
    /// protocol.
    #[error("UnexpectedWireFormat: {0}")]
    UnexpectedWireFormat(String),

    /// Stream ended unexpectedly.
    #[error("UnexpectedEndOfStream")]
    UnexpectedEndOfStream,

    /// TWS answered a historical-ticks request with ticks of another kind. It
    /// does this when the contract has no ticks of the requested kind: a
    /// `TRADES` request (`historical_ticks(..).last()`) for a forex pair, a
    /// spot metal or an index CFD is answered with midpoint ticks. Terminal;
    /// TWS sends nothing more for the request. Request `received` instead.
    ///
    /// A window with no ticks of the requested kind is not this error: TWS
    /// answers it with an empty batch of the requested kind.
    #[error("historical ticks requested as {requested}, but TWS answered with {received} ticks")]
    UnexpectedTickKind {
        /// The kind the request asked for.
        requested: WhatToShow,
        /// The kind TWS sent.
        received: WhatToShow,
    },

    /// The deadline passed before the request ended. Says nothing about the
    /// request at TWS, which may still be running; see
    /// `Subscription::cancel_and_drain`.
    ///
    /// Also returned by one-shot requests answered without a request id
    /// (`server_time`, `managed_accounts`, `request_fa`, ...) when TWS sends
    /// no answer within 30 seconds.
    #[error("timed out before the request ended")]
    Timeout,

    /// A subscription with a buffer limit had `limit` unread items when
    /// another arrived. Terminal: every item queued before it was delivered,
    /// and later frames for the request are discarded. Not a TWS error.
    #[error("subscription buffer limit exceeded ({limit} unread items)")]
    BufferLimitExceeded {
        /// The configured maximum of unread items.
        limit: usize,
    },

    /// A frame arrived whose length prefix cannot describe a TWS message:
    /// shorter than the 4-byte message id, or larger than the 16 MiB ceiling
    /// (`0x00FFFFFF`) that the official client enforces as
    /// `Constants.MaxMsgSize`.
    ///
    /// The length prefix is positional — there is no delimiter or magic value
    /// to re-anchor on — so a single bad prefix desynchronizes every subsequent
    /// read on that socket. Left unchecked, a garbage length is either a
    /// multi-gigabyte allocation or a read that silently consumes (and
    /// destroys) every real message until it is satisfied. Both end in
    /// permanently mis-framed messages that still decode without error, so this
    /// is deliberately raised as a hard framing fault rather than skipped.
    ///
    /// Treated as [`is_connection_lost`](Self::is_connection_lost): reconnecting
    /// is the only way to re-anchor the stream.
    #[error("InvalidFrame: {0}")]
    InvalidFrame(String),

    /// An IB notice frame (TWS error/warning/system message) received in
    /// response to a request. Carries the full typed [`Notice`] — code,
    /// message, optional timestamp, and advanced-order-reject JSON.
    ///
    /// Use [`Notice::category`] / [`Notice::is_order_rejection`] /
    /// [`Notice::is_warning`] to classify without string-parsing. Distinct
    /// from [`Error::ConnectionRejected`] (handshake-time refusal) and the
    /// transport variants ([`Error::Io`], [`Error::ConnectionReset`]).
    #[error("{0}")]
    Notice(Notice),

    /// Attempted to create a duplicate subscription.
    #[error("AlreadySubscribed")]
    AlreadySubscribed,

    /// `account_updates` was called for a different account while one is
    /// live. TWS keeps one account-updates subscription per connection; use
    /// `account_updates_multi` to stream several accounts at once.
    #[error("account updates already streaming {active}; cancel it before requesting {requested}")]
    AccountUpdatesInUse {
        /// The account currently streaming.
        active: AccountId,
        /// The account that was refused.
        requested: AccountId,
    },

    /// An order id at or above 1,500,000,000, the range reserved for request
    /// ids. The client routes frames by id range, so an order there would be
    /// taken for a request. Returned for an id you pass, one
    /// `next_valid_order_id` receives from TWS, or the client's next order id
    /// after a reconnect moved it there.
    #[error("order id {order_id} is at or above {floor}, which is reserved for request ids", floor = crate::client::ids::REQUEST_ID_FLOOR)]
    OrderIdInRequestRange {
        /// The refused order id.
        order_id: i32,
    },

    /// A multi-order submit (`submit_oca_orders`, a bracket's `submit_all`) stopped partway
    /// because sending one order failed with `source`. Every order passed its checks before the
    /// first was sent, so `source` is a transport error, such as [`Error::ConnectionReset`] or
    /// [`Error::Reconnecting`].
    ///
    /// The orders in `sent` may be at TWS: the ones written before the failure, and last, the one
    /// whose write failed, which may still have arrived. A send refused before writing
    /// ([`Error::Reconnecting`]) puts its order in `not_sent` instead, and a submit whose first
    /// send is refused returns the refusal itself, since nothing was sent. Cancel the `sent`
    /// orders by id after reconnecting with the same client id; a cancel for one that never
    /// arrived draws error 10147 and does nothing.
    /// When `source` is [`Error::Shutdown`] or [`Error::ConnectionFailed`], no reconnect follows
    /// on this client: cancel them from a new connection with the same client id.
    /// A bracket's parent and take-profit are held at TWS until the stop-loss transmits, and
    /// `open_orders` doesn't list held orders, so their ids are the only way to reach them. The
    /// orders in `not_sent` were never written. Both lists are in submit order.
    #[error("{} of {} orders may have been sent before the submit stopped: {source}", .sent.len(), .sent.len() + .not_sent.len())]
    OrdersPartiallySent {
        /// Ids of the orders that may be at TWS, in submit order. Never empty.
        sent: Vec<i32>,
        /// Ids of the orders never written, in submit order.
        not_sent: Vec<i32>,
        /// The send failure that stopped the submit.
        source: Box<Error>,
    },

    /// Failed to decode a protobuf message. The decode detail is in
    /// `Display`; [`source`](std::error::Error::source) is `None`.
    #[error("protobuf decode error: {0}")]
    ProtobufDecode(ProtobufDecodeError),
}

/// A protobuf message from TWS failed to decode. Payload of
/// [`Error::ProtobufDecode`].
///
/// Opaque: the `Display` text is its only content, and the `Debug` format is
/// not a stable contract. The underlying decoder is an implementation detail,
/// so its error type is not part of this crate's API.
#[derive(Debug, Clone)]
pub struct ProtobufDecodeError(prost::DecodeError);

impl std::fmt::Display for ProtobufDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for ProtobufDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<crate::transport::routing::DecodedError> for Error {
    /// Project a dispatcher-decoded error payload to [`Error::Notice`].
    /// Mirrors the [`From<ResponseMessage>`] projection but skips the
    /// wire-message re-parse since the dispatcher already extracted the
    /// fields, and moves the message string instead of cloning.
    fn from(payload: crate::transport::routing::DecodedError) -> Error {
        Error::Notice(Notice::from(payload))
    }
}

impl Error {
    /// Build an [`Error::ProtobufDecode`]. Crate-private so the prost error
    /// type stays out of the public API; decoders reach it through
    /// `ResponseMessage::proto` (via `crate::proto::decoders::DecodeProto`).
    pub(crate) fn protobuf_decode(err: prost::DecodeError) -> Error {
        Error::ProtobufDecode(ProtobufDecodeError(err))
    }

    /// Build an [`Error::UnexpectedResponse`] from an internal `ResponseMessage`.
    /// Captures the `Debug` repr in the variant's `String` payload — the
    /// structured envelope is no longer exposed publicly. Crate-private; the
    /// variant's pattern `Error::UnexpectedResponse(_)` remains matchable by
    /// downstream code.
    pub(crate) fn unexpected_response(message: &ResponseMessage) -> Error {
        Error::UnexpectedResponse(format!("{message:?}"))
    }

    /// Build an [`Error::UnexpectedWireFormat`] from an internal `ResponseMessage`.
    /// Same capture as [`Error::unexpected_response`], different variant.
    pub(crate) fn unexpected_wire_format(message: &ResponseMessage) -> Error {
        Error::UnexpectedWireFormat(format!("{message:?}"))
    }

    /// Build an [`Error::Parse`] for text that failed to parse, where no field
    /// name is at hand (a `FromStr`, a token inside a field).
    pub(crate) fn invalid_value(value: impl Into<String>, reason: impl Into<String>) -> Error {
        Error::Parse {
            field: None,
            value: value.into(),
            reason: reason.into(),
        }
    }

    /// The error for a multi-order submit whose send of the order at index `failed` in `ids`
    /// returned `source`. A write that failed may still have reached TWS, so that order counts
    /// as `sent`; a send refused before writing ([`source.is_unsent()`](Error::is_unsent), such as
    /// [`Error::Reconnecting`]) did not, so it counts as `not_sent`. When that leaves nothing
    /// sent, the refusal comes back as is: [`Error::OrdersPartiallySent`] always has a `sent` id.
    pub(crate) fn orders_partially_sent(ids: impl IntoIterator<Item = i32>, failed: usize, source: Error) -> Error {
        let mut sent: Vec<i32> = ids.into_iter().collect();
        let first_unsent = if source.is_unsent() { failed } else { failed + 1 };
        if first_unsent == 0 {
            return source;
        }
        let not_sent = sent.split_off(first_unsent);
        Error::OrdersPartiallySent {
            sent,
            not_sent,
            source: Box::new(source),
        }
    }

    /// The connection is down but the client may bring it back: a reset
    /// ([`Error::ConnectionReset`]) or a send refused while it reconnects
    /// ([`Error::Reconnecting`]). The internal "session interrupted" set the one-shot retry,
    /// `cancel_and_drain` and the cancel log share, so a new such variant joins them all at once.
    pub(crate) fn is_session_interrupted(&self) -> bool {
        matches!(self, Error::ConnectionReset | Error::Reconnecting)
    }

    /// Build an [`Error::Parse`] naming the field, for a field that is absent
    /// or holds something other than text (a timestamp out of range).
    pub(crate) fn invalid_field(field: &'static str, reason: impl Into<String>) -> Error {
        Error::Parse {
            field: Some(field),
            value: String::new(),
            reason: reason.into(),
        }
    }

    /// Build an [`Error::Parse`] for a text-cursor read: field `i` of the
    /// message held `value` and failed with `reason`. The index goes into the
    /// reason, since only the handshake still reads text fields by position.
    /// `reason` is `impl Display` rather than `impl Into<String>` so
    /// [`Error::eof_at`] can pass `format_args!` without allocating twice.
    pub(crate) fn invalid_text_field(i: usize, value: impl Into<String>, reason: impl std::fmt::Display) -> Error {
        Error::invalid_value(value, format!("{reason} (field {i})"))
    }

    /// Name the field of an [`Error::Parse`] that has none, such as one a
    /// `FromStr` returned to a decoder that knows which field it was reading.
    /// Any other error, or a `Parse` already naming a field, is unchanged.
    pub(crate) fn in_field(self, name: &'static str) -> Error {
        match self {
            Error::Parse { field: None, value, reason } => Error::Parse {
                field: Some(name),
                value,
                reason,
            },
            other => other,
        }
    }

    /// Build an [`Error::Parse`] for cursor EOF: the message ran out of fields
    /// while the caller was trying to read field `i`. `label` names the
    /// expected type ("int", "string", ...).
    pub(crate) fn eof_at(i: usize, label: &str) -> Error {
        Error::invalid_text_field(i, "", format_args!("expected {label} and found end of message"))
    }

    /// Returns `true` if this error means the TWS/Gateway stream is unusable in
    /// place and the client should reconnect, rather than retry the in-flight
    /// request.
    ///
    /// Matches [`Error::ConnectionReset`], [`Error::Reconnecting`] and connection-kind [`Error::Io`] errors
    /// (broken pipe, unexpected EOF, connection reset/abort) — recoverable losses
    /// where re-establishing the connection is the right response — plus
    /// [`Error::InvalidFrame`], where the socket is still open but the framing has
    /// desynchronized and only a fresh connection can re-anchor it.
    ///
    /// Returns `false` for failures reconnecting cannot recover, so a read loop can
    /// branch on them separately to stop retrying: intentional teardown
    /// ([`Error::Shutdown`]), handshake refusal ([`Error::ConnectionRejected`],
    /// [`Error::ClientIdInUse`]), and
    /// a session the client gave up on ([`Error::ConnectionFailed`]: reconnection
    /// exhausted, or a read error reconnecting cannot fix).
    ///
    /// The client reconnects on its own, and streams open when the connection
    /// dropped end with [`Error::ConnectionReset`] as soon as it does, before the
    /// reconnect starts. A request sent while it is still reconnecting is refused
    /// with [`Error::Reconnecting`]; once it gives up, requests fail with
    /// `ConnectionFailed` instead. So on `true`, call `Client::wait_connected`
    /// before resubscribing: it returns once the session is back, or with
    /// `ConnectionFailed` / `Shutdown` if it never will be.
    ///
    /// [`Error::OrdersPartiallySent`] returns `true` when its `source` does.
    ///
    /// # Examples
    ///
    /// In a subscription read loop, branch on this predicate to decide whether to
    /// wait for the reconnect and resubscribe or surface a request-level failure:
    ///
    /// ```
    /// use ibapi::Error;
    ///
    /// fn on_stream_error(err: Error) -> Result<(), Error> {
    ///     if err.is_connection_lost() {
    ///         // wait_connected, resubscribe, then keep going
    ///         Ok(())
    ///     } else {
    ///         // a request-level failure (or terminal disconnect) — surface it
    ///         Err(err)
    ///     }
    /// }
    ///
    /// assert!(on_stream_error(Error::ConnectionReset).is_ok());
    /// assert!(on_stream_error(Error::ConnectionFailed).is_err()); // reconnect gave up
    /// assert!(on_stream_error(Error::Shutdown).is_err());
    /// ```
    pub fn is_connection_lost(&self) -> bool {
        use std::io::ErrorKind;
        match self {
            Error::Io(io_err) => matches!(
                io_err.kind(),
                ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::UnexpectedEof | ErrorKind::BrokenPipe
            ),
            Error::ConnectionReset | Error::Reconnecting | Error::InvalidFrame(_) => true,
            Error::OrdersPartiallySent { source, .. } => source.is_connection_lost(),
            _ => false,
        }
    }

    /// Returns `true` only for errors raised by a check before anything was written, so the
    /// request certainly did not reach TWS. `false` means it may have: an order call must then ask
    /// TWS (open orders, executions, the order update stream) what became of it. Some `false`
    /// errors also come before any write (see [`ConnectionFailed`](Error::ConnectionFailed)
    /// below), so `false` is "unknown", not "sent".
    ///
    /// Written for the calls that place, modify, cancel or exercise orders: `place_order`,
    /// `submit_order`, `cancel_order`, `cancel_order_with`, `global_cancel`, `global_cancel_with`,
    /// the order builder's `submit` and `analyze`, and `exercise_options`. Each runs its checks,
    /// then encodes, then writes the request once; only the write can fail after bytes left. The
    /// classification holds for any other call too: no `true` variant is raised after a write.
    ///
    /// `true` for the errors only pre-write checks raise:
    ///
    /// | Variant | Raised by |
    /// |---|---|
    /// | [`Validation`](Error::Validation) | a value check: an order builder's inputs, an `ext_operator` TWS would reject |
    /// | [`InvalidArgument`](Error::InvalidArgument) | a caller-bug check before the write |
    /// | [`OrderIdInRequestRange`](Error::OrderIdInRequestRange) | an order id at or above the request-id range |
    /// | [`ServerVersion`](Error::ServerVersion) | an order field the connected server does not support |
    /// | [`AlreadySubscribed`](Error::AlreadySubscribed) | a second order update stream (not an order call), which writes nothing |
    /// | [`AccountUpdatesInUse`](Error::AccountUpdatesInUse) | the account-updates slot check (not an order call), before the request is written |
    /// | [`Reconnecting`](Error::Reconnecting) | the send gate, while the client reconnects |
    ///
    /// `false` for every other variant, since none of them guarantees that nothing was written.
    /// The ones an order call, or a retried one-shot request, can get:
    ///
    /// - [`ConnectionReset`](Error::ConnectionReset) means the connection dropped. A write it cut
    ///   off may have sent part of the frame. (The refusal while reconnecting is
    ///   [`Reconnecting`](Error::Reconnecting), above.) A one-shot request retried after a reset
    ///   also returns it when its last attempt was refused while reconnecting, since an earlier
    ///   attempt may have been written.
    /// - [`ConnectionFailed`](Error::ConnectionFailed), [`Shutdown`](Error::Shutdown) and
    ///   [`Timeout`](Error::Timeout) refuse a request up front, but a closed socket fails a
    ///   pending write with `Shutdown`, and a one-shot request retried after a reset returns
    ///   them once its first attempt was written.
    /// - [`Io`](Error::Io) is a failed socket write.
    /// - [`Notice`](Error::Notice) and the decode, parse and stream variants report what TWS
    ///   sent back, so the request was sent.
    /// - [`OrdersPartiallySent`](Error::OrdersPartiallySent) reports a multi-order submit that
    ///   stopped partway; its `sent` orders may be at TWS. It is `false` whatever its `source`
    ///   is, since `sent` is never empty: a submit whose first send is refused before writing
    ///   returns that refusal, not this.
    ///
    /// The bracket builder's `submit_all` and `submit_oca_orders` check every order before
    /// sending any, so a `true` error from them means none of their orders was sent; a send
    /// failing partway returns `OrdersPartiallySent`. `next_valid_order_id` returns
    /// `OrderIdInRequestRange` for an id TWS sent back, after its id request was written; that
    /// request places nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::Error;
    ///
    /// fn report(err: &Error) -> &'static str {
    ///     if err.is_unsent() {
    ///         "not sent"
    ///     } else {
    ///         "unknown: query the order"
    ///     }
    /// }
    ///
    /// assert_eq!(report(&Error::InvalidArgument("buffer_limit must be above zero".into())), "not sent");
    /// assert_eq!(report(&Error::Reconnecting), "not sent");
    /// assert_eq!(report(&Error::ConnectionReset), "unknown: query the order");
    /// ```
    pub fn is_unsent(&self) -> bool {
        match self {
            Error::Validation(_)
            | Error::InvalidArgument(_)
            | Error::OrderIdInRequestRange { .. }
            | Error::ServerVersion { .. }
            | Error::AlreadySubscribed
            | Error::AccountUpdatesInUse { .. }
            | Error::Reconnecting => true,
            Error::Io(_)
            | Error::ParseTime(_)
            | Error::Parse { .. }
            | Error::ConnectionFailed
            | Error::ConnectionRejected(_)
            | Error::ClientIdInUse { .. }
            | Error::UnsupportedTimeZone(_)
            | Error::ConnectionReset
            | Error::Cancelled
            | Error::Shutdown
            | Error::UnexpectedResponse(_)
            | Error::UnexpectedWireFormat(_)
            | Error::UnexpectedEndOfStream
            | Error::UnexpectedTickKind { .. }
            | Error::Timeout
            | Error::BufferLimitExceeded { .. }
            | Error::InvalidFrame(_)
            | Error::Notice(_)
            | Error::ProtobufDecode(_)
            | Error::OrdersPartiallySent { .. } => false,
        }
    }

    /// Returns `true` if a read timed out before any byte of the next frame
    /// arrived, which both dispatchers treat as "nothing yet" rather than a
    /// failure — the socket is fine and the next poll continues. A timeout
    /// mid-frame never surfaces: the frame reader waits it out. Distinct from
    /// [`Self::is_connection_lost`], which means the socket is gone.
    pub(crate) fn is_read_timeout(&self) -> bool {
        use std::io::ErrorKind;
        matches!(self, Error::Io(io_err) if matches!(io_err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut))
    }
}

/// `Display` body of [`Error::Parse`]: the field and value when present, then
/// the reason (`action "XYZ": unknown Action`).
fn parse_context(field: Option<&str>, value: &str, reason: &str) -> String {
    match (field, value.is_empty()) {
        (Some(field), false) => format!("{field} {value:?}: {reason}"),
        (Some(field), true) => format!("{field}: {reason}"),
        (None, false) => format!("{value:?}: {reason}"),
        (None, true) => reason.to_string(),
    }
}

// Manual Clone because `std::io::Error` doesn't implement it.
impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::Io(e) => Error::Io(std::io::Error::new(e.kind(), e.to_string())),
            Error::ParseTime(e) => Error::ParseTime(*e),
            Error::Parse { field, value, reason } => Error::Parse {
                field: *field,
                value: value.clone(),
                reason: reason.clone(),
            },
            Error::ServerVersion { required, actual, feature } => Error::ServerVersion {
                required: *required,
                actual: *actual,
                feature: feature.clone(),
            },
            Error::InvalidArgument(s) => Error::InvalidArgument(s.clone()),
            Error::Validation(e) => Error::Validation(e.clone()),
            Error::ConnectionFailed => Error::ConnectionFailed,
            Error::ConnectionRejected(s) => Error::ConnectionRejected(s.clone()),
            Error::ClientIdInUse { client_id } => Error::ClientIdInUse { client_id: *client_id },
            Error::UnsupportedTimeZone(s) => Error::UnsupportedTimeZone(s.clone()),
            Error::ConnectionReset => Error::ConnectionReset,
            Error::Reconnecting => Error::Reconnecting,
            Error::Cancelled => Error::Cancelled,
            Error::Shutdown => Error::Shutdown,
            Error::UnexpectedResponse(m) => Error::UnexpectedResponse(m.clone()),
            Error::UnexpectedWireFormat(m) => Error::UnexpectedWireFormat(m.clone()),
            Error::UnexpectedEndOfStream => Error::UnexpectedEndOfStream,
            Error::UnexpectedTickKind { requested, received } => Error::UnexpectedTickKind {
                requested: *requested,
                received: *received,
            },
            Error::Timeout => Error::Timeout,
            Error::BufferLimitExceeded { limit } => Error::BufferLimitExceeded { limit: *limit },
            Error::InvalidFrame(m) => Error::InvalidFrame(m.clone()),
            Error::Notice(n) => Error::Notice(n.clone()),
            Error::AlreadySubscribed => Error::AlreadySubscribed,
            Error::AccountUpdatesInUse { active, requested } => Error::AccountUpdatesInUse {
                active: active.clone(),
                requested: requested.clone(),
            },
            Error::OrderIdInRequestRange { order_id } => Error::OrderIdInRequestRange { order_id: *order_id },
            Error::ProtobufDecode(e) => Error::ProtobufDecode(e.clone()),
            Error::OrdersPartiallySent { sent, not_sent, source } => Error::OrdersPartiallySent {
                sent: sent.clone(),
                not_sent: not_sent.clone(),
                source: source.clone(),
            },
        }
    }
}

#[cfg(test)]
#[path = "errors_tests.rs"]
mod tests;
