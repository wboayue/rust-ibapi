//! Common StreamDecoder implementations for contracts module
//!
//! This module contains the StreamDecoder trait implementations that are shared
//! between sync and async versions, avoiding code duplication.

use crate::contracts::*;
use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

use super::decoders;
use super::encoders;

impl StreamDecoder<OptionComputation> for OptionComputation {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickOptionComputation, IncomingMessages::Error];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickOptionComputation => Ok(decoders::decode_option_computation(context.server_version, message)?),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        let request_id = request_id.expect("request id required to cancel option calculations");
        match context.and_then(|c| c.request_type) {
            Some(OutgoingMessages::ReqCalcImpliedVolat) => {
                encoders::encode_cancel_option_computation(OutgoingMessages::CancelImpliedVolatility, request_id)
            }
            Some(OutgoingMessages::ReqCalcOptionPrice) => encoders::encode_cancel_option_computation(OutgoingMessages::CancelOptionPrice, request_id),
            _ => panic!(
                "Unsupported request message type option computation cancel: {:?}",
                context.and_then(|c| c.request_type)
            ),
        }
    }
}

impl StreamDecoder<OptionChain> for OptionChain {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::SecurityDefinitionOptionParameter,
        IncomingMessages::SecurityDefinitionOptionParameterEnd,
        IncomingMessages::Error,
    ];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<OptionChain, Error> {
        match message.message_type() {
            IncomingMessages::SecurityDefinitionOptionParameter => Ok(decoders::decode_option_chain(message)?),
            IncomingMessages::SecurityDefinitionOptionParameterEnd => Err(Error::EndOfStream),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_utils::helpers::assert_tws_error_message;

    fn test_context() -> DecoderContext {
        DecoderContext::new(176, None)
    }

    fn error_message() -> ResponseMessage {
        ResponseMessage::from_simple("4|2|9000|10089|Requested market data is not subscribed|")
    }

    #[test]
    fn test_option_computation_decode_error_message() {
        // Error on the subscription's request_id channel surfaces as Error::Message,
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
}
