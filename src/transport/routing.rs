//! Common message routing logic for sync and async implementations

use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};

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
}

/// Determine how to route an incoming message
pub fn determine_routing(message: &ResponseMessage) -> RoutingDecision {
    let message_type = message.message_type();
    
    // Special handling for error messages
    if message_type == IncomingMessages::Error {
        // Error messages have: message_type, version, request_id, error_code
        let request_id = message.peek_int(2).unwrap_or(-1);
        let error_code = message.peek_int(3).unwrap_or(0);
        return RoutingDecision::Error { request_id, error_code };
    }
    
    // Check if message has a request ID
    if let Some(request_id) = message.request_id() {
        return RoutingDecision::ByRequestId(request_id);
    }
    
    // Check if message has an order ID
    if let Some(order_id) = message.order_id() {
        return RoutingDecision::ByOrderId(order_id);
    }
    
    // Certain messages are always shared
    match message_type {
        IncomingMessages::ManagedAccounts 
        | IncomingMessages::NextValidId 
        | IncomingMessages::CurrentTime => RoutingDecision::SharedMessage(message_type),
        _ => RoutingDecision::ByMessageType(message_type),
    }
}

/// Maps incoming message types to their corresponding outgoing request types
/// This is used for shared channel routing
pub fn map_incoming_to_outgoing(message_type: IncomingMessages) -> Option<OutgoingMessages> {
    match message_type {
        IncomingMessages::ManagedAccounts => Some(OutgoingMessages::RequestManagedAccounts),
        IncomingMessages::NextValidId => Some(OutgoingMessages::RequestIds),
        IncomingMessages::CurrentTime => Some(OutgoingMessages::RequestCurrentTime),
        IncomingMessages::Position => Some(OutgoingMessages::RequestPositions),
        IncomingMessages::PositionEnd => Some(OutgoingMessages::RequestPositions),
        IncomingMessages::AccountValue => Some(OutgoingMessages::RequestAccountData),
        IncomingMessages::PortfolioValue => Some(OutgoingMessages::RequestAccountData),
        IncomingMessages::AccountUpdateTime => Some(OutgoingMessages::RequestAccountData),
        IncomingMessages::AccountDownloadEnd => Some(OutgoingMessages::RequestAccountData),
        IncomingMessages::MarketDataType => Some(OutgoingMessages::RequestMarketDataType),
        IncomingMessages::TickPrice => Some(OutgoingMessages::RequestMarketData),
        IncomingMessages::TickSize => Some(OutgoingMessages::RequestMarketData),
        IncomingMessages::TickString => Some(OutgoingMessages::RequestMarketData),
        IncomingMessages::TickGeneric => Some(OutgoingMessages::RequestMarketData),
        IncomingMessages::TickOptionComputation => Some(OutgoingMessages::RequestMarketData),
        IncomingMessages::TickSnapshotEnd => Some(OutgoingMessages::RequestMarketData),
        IncomingMessages::MarketDepth => Some(OutgoingMessages::RequestMarketDepth),
        IncomingMessages::MarketDepthL2 => Some(OutgoingMessages::RequestMarketDepth),
        IncomingMessages::SmartComponents => Some(OutgoingMessages::RequestSmartComponents),
        IncomingMessages::TickReqParams => Some(OutgoingMessages::RequestMarketData),
        _ => None,
    }
}

/// Error codes that are considered warnings (2100-2169)
pub const WARNING_CODES: &[i32] = &[
    2100, 2101, 2102, 2103, 2104, 2105, 2106, 2107, 2108, 2109,
    2110, 2119, 2137, 2151, 2152, 2158, 2167, 2168, 2169
];

/// Check if an error code is a warning
pub fn is_warning_error(error_code: i32) -> bool {
    WARNING_CODES.contains(&error_code)
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
            routing => panic!("Expected ByRequestId routing, got {:?}", routing),
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
            routing => panic!("Expected Error routing, got {:?}", routing),
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
            routing => panic!("Expected SharedMessage routing, got {:?}", routing),
        }
    }

    #[test]
    fn test_is_warning_error() {
        assert!(is_warning_error(2100));
        assert!(is_warning_error(2119));
        assert!(!is_warning_error(200));
        assert!(!is_warning_error(2200));
    }

    #[test]
    fn test_map_incoming_to_outgoing() {
        assert_eq!(
            map_incoming_to_outgoing(IncomingMessages::ManagedAccounts),
            Some(OutgoingMessages::RequestManagedAccounts)
        );
        assert_eq!(
            map_incoming_to_outgoing(IncomingMessages::Position),
            Some(OutgoingMessages::RequestPositions)
        );
        assert_eq!(
            map_incoming_to_outgoing(IncomingMessages::ContractData),
            None
        );
    }
}