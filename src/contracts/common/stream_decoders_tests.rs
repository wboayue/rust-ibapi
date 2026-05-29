use super::*;
use crate::common::test_utils::helpers::{assert_tws_error_message, proto_error_response};

fn test_context() -> DecoderContext {
    DecoderContext::new(176, None)
}

fn error_message() -> ResponseMessage {
    proto_error_response(9000, 10089, "Requested market data is not subscribed")
}

#[test]
fn test_option_computation_decode_error_message() {
    // Error on the subscription's request_id channel surfaces as Error::Notice,
    // not a parse failure or "unexpected message" error (#434).
    let mut message = error_message();
    let err = OptionComputation::decode(&test_context(), &mut message).unwrap_err();
    assert_tws_error_message(err, 10089, "not subscribed");
}

#[test]
fn test_option_chain_decode_error_message() {
    let mut message = error_message();
    let err = OptionChain::decode(&test_context(), &mut message).unwrap_err();
    assert_tws_error_message(err, 10089, "not subscribed");
}
