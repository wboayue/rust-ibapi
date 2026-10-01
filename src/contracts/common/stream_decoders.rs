//! Common StreamDecoder implementations for contracts module
//!
//! This module contains the StreamDecoder trait implementations that are shared
//! between sync and async versions, avoiding code duplication.

use crate::contracts::*;
use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::protocol::{check_version, Features};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

use super::decoders;
use super::encoders;

impl StreamDecoder<OptionComputation> for OptionComputation {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickOptionComputation];

    fn decode(_context: &DecoderContext, message: &ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickOptionComputation => decoders::decode_tick_option_computation(message),
            _ => Err(Error::unexpected_response(message)),
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

impl StreamDecoder<ContractDetails> for ContractDetails {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::ContractData,
        IncomingMessages::BondContractData,
        IncomingMessages::ContractDataEnd,
    ];

    fn decode(_context: &DecoderContext, message: &ResponseMessage) -> Result<ContractDetails, Error> {
        match message.message_type() {
            IncomingMessages::ContractData => decoders::decode_contract_details(message),
            IncomingMessages::BondContractData => decoders::decode_bond_contract_details(message),
            IncomingMessages::ContractDataEnd => Err(Error::EndOfStream),
            _ => Err(Error::unexpected_response(message)),
        }
    }

    /// Below server 215 TWS has no contract-details cancel, so dropping the
    /// subscription writes nothing.
    fn cancel_message(server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        check_version(server_version, Features::CANCEL_CONTRACT_DATA)?;
        let request_id = request_id.ok_or_else(|| Error::InvalidArgument("request id required to cancel contract details".to_string()))?;
        encoders::encode_cancel_contract_data(request_id)
    }
}

impl StreamDecoder<OptionChain> for OptionChain {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::SecurityDefinitionOptionParameter,
        IncomingMessages::SecurityDefinitionOptionParameterEnd,
    ];

    fn decode(_context: &DecoderContext, message: &ResponseMessage) -> Result<OptionChain, Error> {
        match message.message_type() {
            IncomingMessages::SecurityDefinitionOptionParameter => Ok(decoders::decode_option_chain(message)?),
            IncomingMessages::SecurityDefinitionOptionParameterEnd => Err(Error::EndOfStream),
            _ => Err(Error::unexpected_response(message)),
        }
    }
}
