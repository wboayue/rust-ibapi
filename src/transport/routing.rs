//! Common message routing logic for sync and async implementations

use crate::messages::{IncomingMessages, ResponseMessage, WARNING_CODE_RANGE};

/// Represents how a message should be routed
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingDecision {
    /// Route by request ID
    ByRequestId(i32),
    /// Route by order ID
    ByOrderId(i32),
    /// Route by message type to shared channel
    ByMessageType(IncomingMessages),
    /// Route to shared message channel
    SharedMessage(IncomingMessages),
    /// Special handling for error messages
    Error { request_id: i32, error_code: i32 },
    /// Shutdown signal
    Shutdown,
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

/// Decode the protobuf Error envelope to extract the request/order id and error code.
/// Defaults: missing id → `UNSPECIFIED_REQUEST_ID`, missing error_code → 0
/// (matching the text-path defaults in `ResponseMessage::error_request_id`/`error_code`).
fn decode_error_envelope(raw_bytes: &[u8]) -> Option<(i32, i32)> {
    let envelope: crate::proto::ErrorMessage = prost::Message::decode(raw_bytes).ok()?;
    Some((envelope.id.unwrap_or(UNSPECIFIED_REQUEST_ID), envelope.error_code.unwrap_or(0)))
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
pub fn determine_routing(message: &ResponseMessage) -> RoutingDecision {
    let message_type = message.message_type();

    if message_type == IncomingMessages::Shutdown {
        return RoutingDecision::Shutdown;
    }

    // Special handling for error messages
    if message_type == IncomingMessages::Error {
        if message.is_protobuf {
            let (request_id, error_code) = message.raw_bytes().and_then(decode_error_envelope).unwrap_or((UNSPECIFIED_REQUEST_ID, 0));
            return RoutingDecision::Error { request_id, error_code };
        }
        let request_id = message.error_request_id();
        let error_code = message.error_code();
        return RoutingDecision::Error { request_id, error_code };
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
pub enum OrderRoutingStrategy {
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
pub fn order_routing_strategy(message_type: IncomingMessages) -> OrderRoutingStrategy {
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
pub fn is_warning_error(error_code: i32) -> bool {
    WARNING_CODE_RANGE.contains(&error_code)
}

/// Request ID for unspecified errors
pub const UNSPECIFIED_REQUEST_ID: i32 = -1;

#[cfg(test)]
#[path = "routing_tests.rs"]
mod tests;
