use crate::messages::OutgoingMessages;
use crate::orders::TagValue;
use crate::Error;

use super::super::ScannerSubscription;

pub(in crate::scanner) fn encode_scanner_parameters() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(ScannerParametersRequest, OutgoingMessages::RequestScannerParameters)
}

pub(in crate::scanner) fn encode_scanner_subscription(
    request_id: i32,
    subscription: &ScannerSubscription,
    filter: &[TagValue],
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::ScannerSubscriptionRequest {
        req_id: Some(request_id),
        scanner_subscription: Some(crate::proto::encoders::encode_scanner_subscription(subscription, filter)),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestScannerSubscription as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::scanner) fn encode_cancel_scanner_subscription(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelScannerSubscription, OutgoingMessages::CancelScannerSubscription)
}

// Encoder body assertions live in the migrated sync/async tests via
// `assert_request<B>(builder)`.
