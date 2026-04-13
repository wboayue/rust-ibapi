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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_utils::helpers::assert_proto_msg_id;

    #[test]
    fn test_encode_scanner_parameters() {
        let bytes = encode_scanner_parameters().unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestScannerParameters);
    }

    #[test]
    fn test_encode_cancel_scanner_subscription() {
        let bytes = encode_cancel_scanner_subscription(9000).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelScannerSubscription);
    }

    #[test]
    fn test_encode_scanner_subscription() {
        let subscription = ScannerSubscription {
            number_of_rows: 10,
            instrument: Some("STK".to_string()),
            location_code: Some("STK.US".to_string()),
            scan_code: Some("TOP_PERC_GAIN".to_string()),
            ..Default::default()
        };
        let filter = vec![];
        let bytes = encode_scanner_subscription(9000, &subscription, &filter).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestScannerSubscription);
    }
}
