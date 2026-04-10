use crate::scanner::common::decoders;
use crate::scanner::common::encoders;
use crate::messages::{IncomingMessages, RequestMessage, ResponseMessage};
use crate::scanner::ScannerData;
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

impl StreamDecoder<Vec<ScannerData>> for Vec<ScannerData> {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::ScannerData];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Vec<ScannerData>, Error> {
        decoders::decode_scanner_message(message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel scanner subscription.");
        encoders::encode_cancel_scanner_subscription(request_id)
    }
}
