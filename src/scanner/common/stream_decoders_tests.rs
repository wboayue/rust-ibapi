use super::*;
use crate::common::test_utils::helpers::{assert_tws_error_message, proto_error_response};

fn test_context() -> DecoderContext {
    DecoderContext::new(176, None)
}

#[test]
fn test_decode_error_message_surfaces_tws_error() {
    // Previously decode_scanner_message was called blindly, producing a parse
    // failure. Now the scanner request_id channel surfaces Error::Notice (#434).
    let mut message = proto_error_response(9000, 10089, "Requested market data is not subscribed");
    let err = Vec::<ScannerData>::decode(&test_context(), &mut message).unwrap_err();
    assert_tws_error_message(err, 10089, "not subscribed");
}
