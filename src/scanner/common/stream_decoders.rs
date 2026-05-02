use crate::messages::{IncomingMessages, ResponseMessage};
use crate::scanner::common::decoders;
use crate::scanner::common::encoders;
use crate::scanner::ScannerData;
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

impl StreamDecoder<Vec<ScannerData>> for Vec<ScannerData> {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::ScannerData, IncomingMessages::Error];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Vec<ScannerData>, Error> {
        match message.message_type() {
            IncomingMessages::ScannerData => decoders::decode_scanner_message(message),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel scanner subscription.");
        encoders::encode_cancel_scanner_subscription(request_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context() -> DecoderContext {
        DecoderContext::new(176, None)
    }

    #[test]
    fn test_decode_error_message_surfaces_tws_error() {
        // Issue #434: error arriving on the scanner request_id channel must surface
        // as Error::Message — previously decode_scanner_message was called blindly,
        // producing a parse failure.
        let mut message = ResponseMessage::from_simple("4|2|9000|10089|Requested market data is not subscribed|");

        let err = <Vec<ScannerData>>::decode(&test_context(), &mut message).unwrap_err();

        match err {
            Error::Message(code, msg) => {
                assert_eq!(code, 10089);
                assert!(msg.contains("not subscribed"));
            }
            other => panic!("expected Error::Message, got {other:?}"),
        }
    }
}
