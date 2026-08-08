use crate::messages::{IncomingMessages, ResponseMessage};
use crate::scanner::common::decoders;
use crate::scanner::common::encoders;
use crate::scanner::ScannerData;
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

impl StreamDecoder<Vec<ScannerData>> for Vec<ScannerData> {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::ScannerData];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Vec<ScannerData>, Error> {
        // A single-type decoder has no match, so `expect_type` is its `_ =>`
        // backstop — without it `decode` claims an arm for every message type.
        decoders::decode_scanner_data(message.expect_type(IncomingMessages::ScannerData)?)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel scanner subscription.");
        encoders::encode_cancel_scanner_subscription(request_id)
    }
}
