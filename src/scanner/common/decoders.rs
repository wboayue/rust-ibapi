use crate::contracts::SecurityType;
use crate::messages::ResponseMessage;
use crate::Error;

use super::super::ScannerData;

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
        scanner_data.contract_details.contract.symbol = message.next_string()?;
        scanner_data.contract_details.contract.security_type = SecurityType::from(&message.next_string()?);
        scanner_data.contract_details.contract.last_trade_date_or_contract_month = message.next_string()?;
        scanner_data.contract_details.contract.strike = message.next_double()?;
        scanner_data.contract_details.contract.right = message.next_string()?;
        scanner_data.contract_details.contract.exchange = message.next_string()?;
        scanner_data.contract_details.contract.currency = message.next_string()?;
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
