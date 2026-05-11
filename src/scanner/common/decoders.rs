use prost::Message;

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

use super::super::ScannerData;

/// Shared decode function for scanner data messages.
/// Handles message type matching and error conversion.
pub(in crate::scanner) fn decode_scanner_message(message: &mut ResponseMessage) -> Result<Vec<ScannerData>, Error> {
    match message.message_type() {
        IncomingMessages::ScannerData => decode_scanner_data(message),
        IncomingMessages::Error => Err(Error::from(message.clone())),
        _ => Err(Error::UnexpectedResponse(message.clone())),
    }
}

// Both ScannerParameters and ScannerData gate at `PROTOBUF_SCAN_DATA` (210),
// which equals the connection floor (`require_protobuf_support`), so the server
// always emits proto framing for these messages — text-framed arrival is
// rejected via `ResponseMessage::require_proto` and skip-classifies (rule 20).

pub(in crate::scanner) fn decode_scanner_parameters(message: &ResponseMessage) -> Result<String, Error> {
    decode_scanner_parameters_proto(message.require_proto()?)
}

pub(crate) fn decode_scanner_parameters_proto(bytes: &[u8]) -> Result<String, Error> {
    let p = crate::proto::ScannerParameters::decode(bytes)?;
    Ok(p.xml.unwrap_or_default())
}

pub(in crate::scanner) fn decode_scanner_data(message: &ResponseMessage) -> Result<Vec<ScannerData>, Error> {
    decode_scanner_data_proto(message.require_proto()?)
}

pub(crate) fn decode_scanner_data_proto(bytes: &[u8]) -> Result<Vec<ScannerData>, Error> {
    let p = crate::proto::ScannerData::decode(bytes)?;

    let mut results = Vec::with_capacity(p.scanner_data_element.len());
    for elem in p.scanner_data_element {
        let contract = elem
            .contract
            .as_ref()
            .map(crate::proto::decoders::decode_contract)
            .transpose()?
            .unwrap_or_default();
        results.push(ScannerData {
            rank: elem.rank.unwrap_or_default(),
            contract_details: crate::contracts::ContractDetails {
                contract,
                market_name: elem.market_name.unwrap_or_default(),
                ..Default::default()
            },
            leg: elem.combo_key.unwrap_or_default(),
        });
    }

    Ok(results)
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
