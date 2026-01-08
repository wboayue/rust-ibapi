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

/// Determine how to route an incoming message
pub fn determine_routing(message: &ResponseMessage) -> RoutingDecision {
    let message_type = message.message_type();

    if message_type == IncomingMessages::Shutdown {
        return RoutingDecision::Shutdown;
    }

    // Special handling for error messages
    if message_type == IncomingMessages::Error {
        // Error messages have: message_type, version, request_id, error_code
        let request_id = message.peek_int(2).unwrap_or(-1);
        let error_code = message.peek_int(3).unwrap_or(0);
        return RoutingDecision::Error { request_id, error_code };
    }

    // Check if this is an order-related message type
    // This matches the original implementation's explicit list
    match message_type {
        IncomingMessages::OrderStatus
        | IncomingMessages::OpenOrder
        | IncomingMessages::OpenOrderEnd
        | IncomingMessages::CompletedOrder
        | IncomingMessages::CompletedOrdersEnd
        | IncomingMessages::ExecutionData
        | IncomingMessages::ExecutionDataEnd
        | IncomingMessages::CommissionsReport => {
            // For order messages that have an order ID, route by order ID
            // Otherwise, it will be handled by process_orders which checks other routing options
            if let Some(order_id) = message.order_id() {
                return RoutingDecision::ByOrderId(order_id);
            } else {
                // Even without order ID, these are still order messages
                return RoutingDecision::ByOrderId(-1);
            }
        }
        _ => {}
    }

    // Check if message has a request ID
    if let Some(request_id) = message.request_id() {
        return RoutingDecision::ByRequestId(request_id);
    }

    // Certain messages are always shared
    match message_type {
        IncomingMessages::ManagedAccounts | IncomingMessages::NextValidId | IncomingMessages::CurrentTime => {
            RoutingDecision::SharedMessage(message_type)
        }
        _ => RoutingDecision::ByMessageType(message_type),
    }
}

/// Check if an error code is a warning
pub fn is_warning_error(error_code: i32) -> bool {
    WARNING_CODE_RANGE.contains(&error_code)
}

/// Request ID for unspecified errors
pub const UNSPECIFIED_REQUEST_ID: i32 = -1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::ResponseMessage;

    #[test]
    fn test_determine_routing_by_request_id() {
        // Create a mock message with request ID (AccountSummary = 63)
        let message_str = "63\01\0123\0DU123456\0AccountType\0ADVISOR\0USD\0";
        let message = ResponseMessage::from(message_str);

        match determine_routing(&message) {
            RoutingDecision::ByRequestId(id) => assert_eq!(id, 123),
            routing => panic!("Expected ByRequestId routing, got {routing:?}"),
        }
    }

    #[test]
    fn test_determine_routing_error() {
        // Error message format: message_type|version|request_id|error_code|error_msg
        let message_str = "4\02\0123\0200\0No security definition found\0";
        let message = ResponseMessage::from(message_str);

        match determine_routing(&message) {
            RoutingDecision::Error { request_id, error_code } => {
                assert_eq!(request_id, 123);
                assert_eq!(error_code, 200);
            }
            routing => panic!("Expected Error routing, got {routing:?}"),
        }
    }

    #[test]
    fn test_determine_routing_shared_message() {
        // ManagedAccounts message (type 15)
        let message_str = "15\01\0DU123456,DU234567\0";
        let message = ResponseMessage::from(message_str);

        match determine_routing(&message) {
            RoutingDecision::SharedMessage(msg_type) => {
                assert_eq!(msg_type, IncomingMessages::ManagedAccounts);
            }
            routing => panic!("Expected SharedMessage routing, got {routing:?}"),
        }
    }

    #[test]
    fn test_is_warning_error() {
        // Test range boundaries
        assert!(is_warning_error(2100));
        assert!(is_warning_error(2169));

        // Test some values in the middle
        assert!(is_warning_error(2119));
        assert!(is_warning_error(2150));

        // Test values outside the range
        assert!(!is_warning_error(2099));
        assert!(!is_warning_error(2170));
        assert!(!is_warning_error(200));
        assert!(!is_warning_error(2200));
    }

    #[test]
    fn test_order_message_routing() {
        // Test OpenOrder with order ID at position 1
        let message_str = "5\0123\0AAPL\0STK\0"; // OpenOrder with order_id=123
        let message = ResponseMessage::from(message_str);
        match determine_routing(&message) {
            RoutingDecision::ByOrderId(id) => assert_eq!(id, 123),
            routing => panic!("Expected ByOrderId routing, got {routing:?}"),
        }

        // Test CompletedOrdersEnd (no order ID)
        let message_str = "102\01\0"; // CompletedOrdersEnd
        let message = ResponseMessage::from(message_str);
        match determine_routing(&message) {
            RoutingDecision::ByOrderId(id) => assert_eq!(id, -1),
            routing => panic!("Expected ByOrderId(-1) routing, got {routing:?}"),
        }

        // Test ExecutionData with order ID at position 2
        let message_str = "11\01\0123\0456\0"; // ExecutionData with request_id=1, order_id=123
        let message = ResponseMessage::from(message_str);
        match determine_routing(&message) {
            RoutingDecision::ByOrderId(id) => assert_eq!(id, 123),
            routing => panic!("Expected ByOrderId routing, got {routing:?}"),
        }

        // Test CommissionsReport (no order ID but still an order message)
        let message_str = "59\01\0exec123\0100.0\0USD\0"; // CommissionsReport
        let message = ResponseMessage::from(message_str);
        match determine_routing(&message) {
            RoutingDecision::ByOrderId(id) => assert_eq!(id, -1),
            routing => panic!("Expected ByOrderId(-1) routing, got {routing:?}"),
        }
    }
}
