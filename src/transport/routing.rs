//! Common message routing logic for sync and async implementations

use crate::errors::Error;
use crate::messages::{IncomingMessages, Notice, ResponseMessage, DATA_ADVISORY_CODES, WARNING_CODE_RANGE};

use super::RoutedItem;

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

/// Decode the protobuf Error envelope. Defaults match the text-path accessors:
/// missing id → `UNSPECIFIED_REQUEST_ID`, missing error_code → 0,
/// missing strings → empty, missing error_time → `None`.
pub(crate) fn decode_error_envelope(raw_bytes: &[u8]) -> Option<DecodedError> {
    let envelope: crate::proto::ErrorMessage = prost::Message::decode(raw_bytes).ok()?;
    Some(DecodedError {
        request_id: envelope.id.unwrap_or(UNSPECIFIED_REQUEST_ID),
        error_code: envelope.error_code.unwrap_or(0),
        error_message: envelope.error_msg.unwrap_or_default(),
        error_time: envelope.error_time,
        advanced_order_reject_json: envelope.advanced_order_reject_json.unwrap_or_default(),
    })
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
        let decoded = message.raw_bytes().and_then(decode_error_envelope).unwrap_or_default();
        return RoutingDecision::Error(decoded);
    }

    // ResponseMessage::{order_id, request_id} are proto-aware, so the same
    // dispatch handles text and protobuf wire formats.
    if is_order_message(message_type) {
        return RoutingDecision::ByOrderId(message.order_id().unwrap_or(-1));
    }
    if is_shared_message(message_type) {
        return RoutingDecision::SharedMessage(message_type);
    }
    if let Some(request_id) = message.request_id() {
        return RoutingDecision::ByRequestId(request_id);
    }
    RoutingDecision::ByMessageType(message_type)
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

/// Check if an error code is a warning.
///
/// Warnings ([`WARNING_CODE_RANGE`]) and data advisories
/// ([`DATA_ADVISORY_CODES`]) are informational — TWS proceeds with the
/// request — so they are routed as a `Notice` rather than terminating the
/// subscription as an `Error`.
pub(crate) fn is_warning_error(error_code: i32) -> bool {
    WARNING_CODE_RANGE.contains(&error_code) || DATA_ADVISORY_CODES.contains(&error_code)
}

/// Request ID for unspecified errors
pub(crate) const UNSPECIFIED_REQUEST_ID: i32 = -1;

/// The outcome of classifying an inbound error frame.
///
/// The *policy* (which arm applies) is centralised here; each transport
/// provides only the runtime-specific delivery in its `route_error_message`.
#[derive(Debug)]
pub(crate) enum ErrorDisposition {
    /// Log + `NoticeStream` only (request-less warning).
    NoticeOnly(Notice),
    /// `NoticeStream` + fail-fast fan-out to in-flight one-shot shared
    /// requests (request-less hard error).
    NoticeAndFailOneShots(Notice, Error),
    /// Deliver `RoutedItem` to the subscription that owns `request_id`.
    Route(i32, RoutedItem),
}

/// Classify an inbound error frame into the action each transport must take.
///
/// Extracts the common four-arm policy so that `sync::route_error_message` and
/// `async::route_error_message` are thin runtime-specific delivery shells.
pub(crate) fn classify_error(payload: DecodedError) -> ErrorDisposition {
    let request_id = payload.request_id;
    let is_warning = is_warning_error(payload.error_code);

    if request_id == UNSPECIFIED_REQUEST_ID {
        let notice = Notice::from(payload.clone());
        if is_warning {
            ErrorDisposition::NoticeOnly(notice)
        } else {
            ErrorDisposition::NoticeAndFailOneShots(notice, Error::from(payload))
        }
    } else {
        let item = if is_warning {
            RoutedItem::Notice(Notice::from(payload))
        } else {
            RoutedItem::Error(Error::from(payload))
        };
        ErrorDisposition::Route(request_id, item)
    }
}

#[cfg(test)]
#[path = "routing_tests.rs"]
mod tests;
