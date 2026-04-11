use prost::Message;

use crate::contracts::{Currency, Exchange, SecurityType, Symbol};
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

use super::super::ScannerData;

/// Shared decode function for scanner data messages.
/// Handles message type matching and error conversion.
pub(in crate::scanner) fn decode_scanner_message(message: &mut ResponseMessage) -> Result<Vec<ScannerData>, Error> {
    match message.message_type() {
        IncomingMessages::ScannerData => decode_scanner_data(message.clone()),
        IncomingMessages::Error => Err(Error::from(message.clone())),
        _ => Err(Error::UnexpectedResponse(message.clone())),
    }
}

pub(in crate::scanner) fn decode_scanner_parameters(mut message: ResponseMessage) -> Result<String, Error> {
    message.skip(); // skip message type
    message.skip(); // skip message version

    message.next_string()
}

pub(in crate::scanner) fn decode_scanner_data(mut message: ResponseMessage) -> Result<Vec<ScannerData>, Error> {
    message.skip(); // skip message type
    message.skip(); // skip message version
    message.skip(); // request id

    let number_of_elements = message.next_int()?;
    let mut matches = Vec::with_capacity(number_of_elements as usize);

    for _ in 0..number_of_elements {
        let mut scanner_data = ScannerData {
            rank: message.next_int()?,
            ..Default::default()
        };

        scanner_data.contract_details.contract.contract_id = message.next_int()?;
        scanner_data.contract_details.contract.symbol = Symbol::from(message.next_string()?);
        scanner_data.contract_details.contract.security_type = SecurityType::from(&message.next_string()?);
        scanner_data.contract_details.contract.last_trade_date_or_contract_month = message.next_string()?;
        scanner_data.contract_details.contract.strike = message.next_double()?;
        scanner_data.contract_details.contract.right = message.next_string()?;
        scanner_data.contract_details.contract.exchange = Exchange::from(message.next_string()?);
        scanner_data.contract_details.contract.currency = Currency::from(message.next_string()?);
        scanner_data.contract_details.contract.local_symbol = message.next_string()?;
        scanner_data.contract_details.market_name = message.next_string()?;
        scanner_data.contract_details.contract.trading_class = message.next_string()?;

        message.skip(); // distance
        message.skip(); // benchmark
        message.skip(); // projection

        scanner_data.leg = message.next_string()?;

        matches.push(scanner_data);
    }

    Ok(matches)
}

#[allow(dead_code)]
pub(crate) fn decode_scanner_data_proto(bytes: &[u8]) -> Result<Vec<ScannerData>, Error> {
    let p = crate::proto::ScannerData::decode(bytes)?;

    let mut results = Vec::with_capacity(p.scanner_data_element.len());
    for elem in &p.scanner_data_element {
        let contract = elem.contract.as_ref().map(crate::proto::decoders::decode_contract).unwrap_or_default();

        let mut contract_details = crate::contracts::ContractDetails {
            contract,
            ..Default::default()
        };
        contract_details.market_name = elem.market_name.clone().unwrap_or_default();

        results.push(ScannerData {
            rank: elem.rank.unwrap_or_default(),
            contract_details,
            leg: elem.combo_key.clone().unwrap_or_default(),
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_scanner_data_proto() {
        use prost::Message;

        let proto_msg = crate::proto::ScannerData {
            req_id: Some(1),
            scanner_data_element: vec![
                crate::proto::ScannerDataElement {
                    rank: Some(0),
                    contract: Some(crate::proto::Contract {
                        con_id: Some(265598),
                        symbol: Some("AAPL".into()),
                        sec_type: Some("STK".into()),
                        ..Default::default()
                    }),
                    market_name: Some("NMS".into()),
                    distance: Some("1.5".into()),
                    benchmark: Some("".into()),
                    projection: Some("".into()),
                    combo_key: Some("".into()),
                },
                crate::proto::ScannerDataElement {
                    rank: Some(1),
                    contract: Some(crate::proto::Contract {
                        con_id: Some(76792991),
                        symbol: Some("TSLA".into()),
                        sec_type: Some("STK".into()),
                        ..Default::default()
                    }),
                    market_name: Some("NMS".into()),
                    distance: None,
                    benchmark: None,
                    projection: None,
                    combo_key: None,
                },
            ],
        };

        let mut bytes = Vec::new();
        proto_msg.encode(&mut bytes).unwrap();

        let results = decode_scanner_data_proto(&bytes).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].rank, 0);
        assert_eq!(results[0].contract_details.contract.contract_id, 265598);
        assert_eq!(results[0].contract_details.market_name, "NMS");
    }
}
