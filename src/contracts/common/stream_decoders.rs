//! Common StreamDecoder implementations for contracts module
//!
//! This module contains the StreamDecoder trait implementations that are shared
//! between sync and async versions, avoiding code duplication.

use crate::common::error_helpers;
use crate::common::request_helpers::CancellableOneShot;
use crate::contracts::*;
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::protocol::{check_version, Features};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

use super::decoders;
use super::encoders;

/// The request behind `calculate_option_price`.
pub(crate) enum OptionPriceRequest {}

impl CancellableOneShot for OptionPriceRequest {
    type Proto = crate::proto::TickOptionComputation;
    type Output = OptionComputation;

    fn decode(payload: Self::Proto) -> Result<OptionComputation, Error> {
        decoders::decode_tick_option_computation_proto(payload)
    }

    fn cancel(request_id: i32) -> Result<Vec<u8>, Error> {
        encoders::encode_cancel_option_price(request_id)
    }
}

/// The request behind `calculate_implied_volatility`.
pub(crate) enum ImpliedVolatilityRequest {}

impl CancellableOneShot for ImpliedVolatilityRequest {
    type Proto = crate::proto::TickOptionComputation;
    type Output = OptionComputation;

    fn decode(payload: Self::Proto) -> Result<OptionComputation, Error> {
        decoders::decode_tick_option_computation_proto(payload)
    }

    fn cancel(request_id: i32) -> Result<Vec<u8>, Error> {
        encoders::encode_cancel_implied_volatility(request_id)
    }
}

impl StreamDecoder<ContractDetails> for ContractDetails {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::ContractData,
        IncomingMessages::BondContractData,
        IncomingMessages::ContractDataEnd,
    ];
    const END_MESSAGES: &'static [IncomingMessages] = &[IncomingMessages::ContractDataEnd];

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
        let request_id = error_helpers::require_request_id_for(request_id, "cancel contract details")?;
        encoders::encode_cancel_contract_data(request_id)
    }
}

impl StreamDecoder<OptionChain> for OptionChain {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::SecurityDefinitionOptionParameter,
        IncomingMessages::SecurityDefinitionOptionParameterEnd,
    ];
    const END_MESSAGES: &'static [IncomingMessages] = &[IncomingMessages::SecurityDefinitionOptionParameterEnd];

    fn decode(_context: &DecoderContext, message: &ResponseMessage) -> Result<OptionChain, Error> {
        match message.message_type() {
            IncomingMessages::SecurityDefinitionOptionParameter => Ok(decoders::decode_option_chain(message)?),
            IncomingMessages::SecurityDefinitionOptionParameterEnd => Err(Error::EndOfStream),
            _ => Err(Error::unexpected_response(message)),
        }
    }
}
