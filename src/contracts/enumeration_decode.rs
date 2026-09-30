//! Allocation-free wire preflight before prost allocates arrays/maps/strings.

use super::QueryLimits;
use crate::contracts::{ContractDescription, ContractDetails, OptionChain};
use crate::messages::ResponseMessage;
use crate::Error;
use prost::encoding::{decode_key, decode_varint, skip_field, DecodeContext, WireType};
use prost::Message;

#[derive(Default)]
pub(super) struct Budget {
    limits: QueryLimits,
    rows: usize,
    entries: usize,
}

impl Budget {
    pub(super) fn new(limits: QueryLimits) -> Self {
        Self { limits, rows: 0, entries: 0 }
    }

    fn entries(&mut self, count: usize) -> Result<(), Error> {
        if count > self.limits.decode_entries.saturating_sub(self.entries) {
            return Err(Error::ResponseLimitExceeded {
                resource: "decode entries",
                limit: self.limits.decode_entries,
            });
        }
        self.entries += count;
        Ok(())
    }

    fn row(&mut self) -> Result<(), Error> {
        if self.rows >= self.limits.rows {
            return Err(Error::ResponseLimitExceeded {
                resource: "rows",
                limit: self.limits.rows,
            });
        }
        self.rows += 1;
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Schema {
    Details,
    Contract,
    ContractMetadata,
    Symbols,
    Description,
    Chain,
    Scalars,
}

impl Schema {
    fn nested(self, tag: u32) -> Option<Self> {
        match (self, tag) {
            (Self::Details, 2) | (Self::Description, 1) => Some(Self::Contract),
            (Self::Details, 3) => Some(Self::ContractMetadata),
            (Self::Symbols, 2) => Some(Self::Description),
            (Self::Contract, 17 | 20) | (Self::ContractMetadata, 17 | 58) => Some(Self::Scalars),
            _ => None,
        }
    }
}

fn length_delimited<'a>(bytes: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    let length = decode_varint(bytes)?;
    let length = usize::try_from(length).map_err(|_| Error::UnexpectedResponse("length does not fit usize".into()))?;
    if length > bytes.len() {
        return Err(Error::UnexpectedResponse("truncated length-delimited field".into()));
    }
    let (value, rest) = bytes.split_at(length);
    *bytes = rest;
    Ok(value)
}

fn scan(mut bytes: &[u8], schema: Schema, budget: &mut Budget) -> Result<(), Error> {
    while !bytes.is_empty() {
        let (tag, wire) = decode_key(&mut bytes)?;
        // `skip_field` would walk a group's fields without this budget. TWS
        // schemas are proto3, which cannot declare groups, so refuse one.
        if matches!(wire, WireType::StartGroup | WireType::EndGroup) {
            return Err(Error::UnexpectedResponse("unexpected protobuf group".into()));
        }
        budget.entries(1)?;
        if matches!(schema, Schema::Symbols) && tag == 2 {
            budget.row()?;
        }
        if let Some(nested) = schema.nested(tag) {
            if wire != WireType::LengthDelimited {
                return Err(Error::UnexpectedResponse("invalid nested-message wire type".into()));
            }
            scan(length_delimited(&mut bytes)?, nested, budget)?;
        } else if matches!(schema, Schema::Chain) && tag == 7 && wire == WireType::LengthDelimited {
            let packed = length_delimited(&mut bytes)?;
            if !packed.len().is_multiple_of(8) {
                return Err(Error::UnexpectedResponse("invalid packed strike length".into()));
            }
            budget.entries(packed.len() / 8)?;
        } else {
            skip_field(wire, tag, &mut bytes, DecodeContext::default())?;
        }
    }
    Ok(())
}

pub(super) fn details(message: &ResponseMessage, budget: &mut Budget) -> Result<ContractDetails, Error> {
    let bytes = message.require_proto()?;
    budget.row()?;
    scan(bytes, Schema::Details, budget)?;
    crate::contracts::common::decoders::decode_contract_data_proto(bytes)
}

pub(super) fn chain(message: &ResponseMessage, budget: &mut Budget) -> Result<OptionChain, Error> {
    let bytes = message.require_proto()?;
    budget.row()?;
    scan(bytes, Schema::Chain, budget)?;
    crate::contracts::common::decoders::decode_option_chain_proto(bytes)
}

pub(super) fn symbols(message: &ResponseMessage, budget: &mut Budget) -> Result<Vec<ContractDescription>, Error> {
    let bytes = message.require_proto()?;
    scan(bytes, Schema::Symbols, budget)?;
    crate::contracts::common::decoders::decode_symbol_samples_proto(crate::proto::SymbolSamples::decode(bytes)?)
}

#[cfg(test)]
#[path = "enumeration_decode_tests.rs"]
mod tests;
