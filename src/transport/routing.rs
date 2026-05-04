//! Common message routing logic for sync and async implementations

use time::OffsetDateTime;

use crate::messages::{IncomingMessages, Notice, ResponseMessage, WARNING_CODE_RANGE};

/// Represents how a message should be routed
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RoutingDecision {
    /// Route by request ID
    ByRequestId(i32),
    /// Route by order ID
    ByOrderId(i32),
    /// Route by message type to shared channel
    ByMessageType(IncomingMessages),
    /// Route to shared message channel
    SharedMessage(IncomingMessages),
    /// Special handling for error messages
    Error(DecodedError),
    /// Shutdown signal
    Shutdown,
}

/// Decoded contents of an Error wire message (type 4), populated regardless of
/// wire format. Carries both warnings (codes 2100..=2169) and hard errors.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DecodedError {
    pub request_id: i32,
    pub error_code: i32,
    pub error_message: String,
    /// Milliseconds since Unix epoch; `None` for old-format text messages without an error_time field.
    pub error_time: Option<i64>,
    pub advanced_order_reject_json: String,
}

impl Default for DecodedError {
    fn default() -> Self {
        Self {
            request_id: UNSPECIFIED_REQUEST_ID,
            error_code: 0,
            error_message: String::new(),
            error_time: None,
            advanced_order_reject_json: String::new(),
        }
    }
}

/// Build a public [`Notice`] from the dispatcher-decoded error payload,
/// preserving `advanced_order_reject_json` and converting `error_time`
/// (millis-since-epoch) to `OffsetDateTime`.
pub(crate) fn notice_from_decoded(payload: &DecodedError) -> Notice {
    let error_time = payload
        .error_time
        .and_then(|millis| OffsetDateTime::from_unix_timestamp_nanos(millis as i128 * 1_000_000).ok());
    Notice {
        code: payload.error_code,
        message: payload.error_message.clone(),
        error_time,
        advanced_order_reject_json: payload.advanced_order_reject_json.clone(),
    }
}

/// Where an error/notice should be delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Routing {
    /// No subscription owner — the message is logged only.
    Unrouted,
    /// Owned by a subscription identified by `request_id` (or `order_id`,
    /// since both share the same numeric space when delivering to the
    /// dispatcher's request/order channels).
    Owned(i32),
}

/// Severity of an error payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Severity {
    /// Non-fatal warning (codes 2100..=2169). When `Owned`, deliver as a
    /// `RoutedItem::Notice` without terminating the subscription.
    Warning,
    /// Hard error. When `Owned`, deliver as a `RoutedItem::Error` and the
    /// subscription terminates.
    HardError,
}

/// Classifies an error payload along two orthogonal axes — routing (owned vs.
/// unrouted) and severity (warning vs. hard error). Each consumer matches on
/// the axis it cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ErrorDelivery {
    pub routing: Routing,
    pub severity: Severity,
}

/// Classify an Error/Warning payload by its `request_id` and `error_code`.
pub(crate) fn classify_error_delivery(request_id: i32, error_code: i32) -> ErrorDelivery {
    ErrorDelivery {
        routing: if request_id == UNSPECIFIED_REQUEST_ID {
            Routing::Unrouted
        } else {
            Routing::Owned(request_id)
        },
        severity: if is_warning_error(error_code) {
            Severity::Warning
        } else {
            Severity::HardError
        },
    }
}

/// Minimal protobuf envelope to extract the first int32 field (tag 1).
#[derive(Clone, PartialEq, ::prost::Message)]
struct RoutingEnvelope {
    #[prost(int32, optional, tag = "1")]
    pub id: Option<i32>,
}

/// Try to extract a request/order ID from protobuf raw bytes.
/// Most protobuf messages encode `req_id` or `order_id` at tag 1 as an int32.
/// Messages where tag 1 is not the routing ID (e.g. CommissionsReport) will need
/// per-message-type handling when those messages migrate to protobuf.
fn protobuf_first_int(raw_bytes: &[u8]) -> Option<i32> {
    prost::Message::decode(raw_bytes).ok().and_then(|e: RoutingEnvelope| e.id)
}

/// Decode the protobuf Error envelope. Defaults match the text-path accessors:
/// missing id → `UNSPECIFIED_REQUEST_ID`, missing error_code → 0,
/// missing strings → empty, missing error_time → `None`.
fn decode_error_envelope(raw_bytes: &[u8]) -> Option<DecodedError> {
    let envelope: crate::proto::ErrorMessage = prost::Message::decode(raw_bytes).ok()?;
    Some(DecodedError {
        request_id: envelope.id.unwrap_or(UNSPECIFIED_REQUEST_ID),
        error_code: envelope.error_code.unwrap_or(0),
        error_message: envelope.error_msg.unwrap_or_default(),
        error_time: envelope.error_time,
        advanced_order_reject_json: envelope.advanced_order_reject_json.unwrap_or_default(),
    })
}

/// Extract Error fields from a text-format `ResponseMessage`. Field layout:
/// `..., error_message, advanced_order_reject_json?, error_time?`. Missing trailing
/// fields default to empty/None (old format / pre-`ADVANCED_ORDER_REJECT` servers).
fn extract_text_error(message: &ResponseMessage) -> DecodedError {
    let error_msg_idx = message.error_message_index();
    let advanced_idx = error_msg_idx + 1;
    let advanced_order_reject_json = if advanced_idx < message.fields.len() {
        message.peek_string(advanced_idx)
    } else {
        String::new()
    };
    let error_time = message.peek_long(error_msg_idx + 2).ok();
    DecodedError {
        request_id: message.error_request_id(),
        error_code: message.error_code(),
        error_message: message.error_message(),
        error_time,
        advanced_order_reject_json,
    }
}

fn is_order_message(message_type: IncomingMessages) -> bool {
    matches!(
        message_type,
        IncomingMessages::OrderStatus
            | IncomingMessages::OpenOrder
            | IncomingMessages::OpenOrderEnd
            | IncomingMessages::CompletedOrder
            | IncomingMessages::CompletedOrdersEnd
            | IncomingMessages::ExecutionData
            | IncomingMessages::ExecutionDataEnd
            | IncomingMessages::CommissionsReport
    )
}

fn is_shared_message(message_type: IncomingMessages) -> bool {
    matches!(
        message_type,
        IncomingMessages::ManagedAccounts | IncomingMessages::NextValidId | IncomingMessages::CurrentTime
    )
}

/// Determine how to route an incoming message
pub(crate) fn determine_routing(message: &ResponseMessage) -> RoutingDecision {
    let message_type = message.message_type();

    if message_type == IncomingMessages::Shutdown {
        return RoutingDecision::Shutdown;
    }

    if message_type == IncomingMessages::Error {
        let decoded = if message.is_protobuf {
            message.raw_bytes().and_then(decode_error_envelope).unwrap_or_default()
        } else {
            extract_text_error(message)
        };
        return RoutingDecision::Error(decoded);
    }

    // Protobuf messages: extract routing ID from raw bytes
    if message.is_protobuf {
        if is_order_message(message_type) {
            let id = message.raw_bytes().and_then(protobuf_first_int).unwrap_or(-1);
            return RoutingDecision::ByOrderId(id);
        }
        if is_shared_message(message_type) {
            return RoutingDecision::SharedMessage(message_type);
        }
        let id = message.raw_bytes().and_then(protobuf_first_int).unwrap_or(-1);
        if id >= 0 {
            return RoutingDecision::ByRequestId(id);
        }
        return RoutingDecision::ByMessageType(message_type);
    }

    // Text messages: order routing
    if is_order_message(message_type) {
        let order_id = message.order_id().unwrap_or(-1);
        return RoutingDecision::ByOrderId(order_id);
    }

    // Check if message has a request ID
    if let Some(request_id) = message.request_id() {
        return RoutingDecision::ByRequestId(request_id);
    }

    if is_shared_message(message_type) {
        RoutingDecision::SharedMessage(message_type)
    } else {
        RoutingDecision::ByMessageType(message_type)
    }
}

/// Routing strategy for order-related messages.
/// Describes which channel keys to try and in what order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OrderRoutingStrategy {
    /// Try order_id channel, then request_id channel. Store execution_id mapping.
    ExecutionData,
    /// Try order_id channel, then request_id channel.
    ExecutionDataEnd,
    /// Try order_id channel, then shared channel.
    OrderOrShared,
    /// Route via execution_id only.
    ByExecutionId,
    /// Route to shared channel only.
    SharedOnly,
    /// Route by order_id only.
    ByOrderId,
}

/// Determine the routing strategy for an order-related message type.
pub(crate) fn order_routing_strategy(message_type: IncomingMessages) -> OrderRoutingStrategy {
    match message_type {
        IncomingMessages::ExecutionData => OrderRoutingStrategy::ExecutionData,
        IncomingMessages::ExecutionDataEnd => OrderRoutingStrategy::ExecutionDataEnd,
        IncomingMessages::OpenOrder | IncomingMessages::OrderStatus => OrderRoutingStrategy::OrderOrShared,
        IncomingMessages::CommissionsReport => OrderRoutingStrategy::ByExecutionId,
        IncomingMessages::CompletedOrder | IncomingMessages::OpenOrderEnd | IncomingMessages::CompletedOrdersEnd => OrderRoutingStrategy::SharedOnly,
        _ => OrderRoutingStrategy::ByOrderId,
    }
}

/// Check if an error code is a warning
pub(crate) fn is_warning_error(error_code: i32) -> bool {
    WARNING_CODE_RANGE.contains(&error_code)
}

/// Request ID for unspecified errors
pub(crate) const UNSPECIFIED_REQUEST_ID: i32 = -1;

#[cfg(test)]
#[path = "routing_tests.rs"]
mod tests;
