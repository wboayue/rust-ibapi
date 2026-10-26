use crate::common::error_helpers;
use crate::messages::{IncomingMessages, Notice, ResponseMessage, HMDS_ERROR_CODE, HMDS_QUERY_MESSAGE_CODE};
use crate::scanner::common::decoders;
use crate::scanner::common::encoders;
use crate::scanner::ScannerData;
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

impl StreamDecoder<Vec<ScannerData>> for Vec<ScannerData> {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::ScannerData];

    fn is_nonterminal_notice(notice: &Notice) -> bool {
        // An empty scanner query can report 165 while its continuous request
        // remains active. Only ScannerData itself represents a completed batch.
        notice.code == HMDS_QUERY_MESSAGE_CODE && notice.message.to_ascii_lowercase().contains("no items retrieved")
    }

    fn decode(_context: &DecoderContext, message: &ResponseMessage) -> Result<Vec<ScannerData>, Error> {
        // A single-type decoder has no match, so `expect_type` is its `_ =>`
        // backstop — without it `decode` claims an arm for every message type.
        decoders::decode_scanner_data(message.expect_type(IncomingMessages::ScannerData)?)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        let request_id = error_helpers::require_request_id_for(request_id, "encode cancel scanner subscription")?;
        encoders::encode_cancel_scanner_subscription(request_id)
    }

    /// Live: after 162 TWS answers the cancel with 365 (no scanner subscription).
    /// Verified for "scanner type … is disabled"; the other 162s seen on
    /// scanners (subscription cancelled, pacing) are believed to end it too.
    /// Not 165: its "no items retrieved" form keeps the scanner alive (#886).
    fn rejection_ends_request(notice: &Notice) -> bool {
        notice.code == HMDS_ERROR_CODE
    }
}
