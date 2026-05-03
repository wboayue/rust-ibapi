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
    use crate::common::test_utils::helpers::assert_tws_error_message;

    fn test_context() -> DecoderContext {
        DecoderContext::new(176, None)
    }

    #[test]
    fn test_decode_error_message_surfaces_tws_error() {
        // Previously decode_scanner_message was called blindly, producing a parse
        // failure. Now the scanner request_id channel surfaces Error::Message (#434).
        let mut message = ResponseMessage::from_simple("4|2|9000|10089|Requested market data is not subscribed|");
        let err = Vec::<ScannerData>::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }
}
