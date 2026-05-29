use super::*;
use crate::common::test_utils::helpers::{assert_tws_error_message, proto_error_response};

fn test_context() -> DecoderContext {
    DecoderContext::new(176, None)
}

fn error_message() -> ResponseMessage {
    proto_error_response(9000, 10089, "Requested market data is not subscribed")
}

#[test]
fn test_wsh_metadata_decode_error_message() {
    // Error on the wsh request_id channel surfaces as Error::Notice (#434).
    let mut message = error_message();
    let err = WshMetadata::decode(&test_context(), &mut message).unwrap_err();
    assert_tws_error_message(err, 10089, "not subscribed");
}

#[test]
fn test_wsh_event_data_decode_error_message() {
    let mut message = error_message();
    let err = WshEventData::decode(&test_context(), &mut message).unwrap_err();
    assert_tws_error_message(err, 10089, "not subscribed");
}
