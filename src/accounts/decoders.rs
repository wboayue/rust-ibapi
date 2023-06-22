use std::default;

use crate::{contracts::SecurityType, messages};
use crate::messages::ResponseMessage;
use crate::Error;

use super::{Position, FamilyCode, PositionMulti};

pub(crate) fn position(message: &mut ResponseMessage) -> Result<Position, Error> {
    message.skip(); // message type

    let message_version = message.next_int()?; // message version

    let mut position = Position {
        account: message.next_string()?,
        ..Default::default()
    };

    position.contract.contract_id = message.next_int()?;
    position.contract.symbol = message.next_string()?;
    position.contract.security_type = SecurityType::from(&message.next_string()?);
    position.contract.last_trade_date_or_contract_month = message.next_string()?;
    position.contract.strike = message.next_double()?;
    position.contract.right = message.next_string()?;
    position.contract.multiplier = message.next_string()?;
    position.contract.exchange = message.next_string()?;
    position.contract.currency = message.next_string()?;
    position.contract.local_symbol = message.next_string()?;

    if message_version >= 2 {
        position.contract.trading_class = message.next_string()?;
    }

    position.position = message.next_double()?;

    if message_version >= 3 {
        position.average_cost = message.next_double()?;
    }

    Ok(position)
}

pub(crate) fn position_multi(message: &mut ResponseMessage) -> Result<PositionMulti, Error> {
    message.skip();

    let message_version = message.next_int()?;
    
    let mut position_multi = PositionMulti {
        account: message.next_string()?, 
        ..Default::default()
    };
    position_multi.contract.contract_id = message.next_int()?;
    position_multi.contract.symbol = message.next_string()?;
    position_multi.contract.security_type = SecurityType::from(&message.next_string()?);
    position_multi.contract.last_trade_date_or_contract_month = message.next_string()?;
    position_multi.contract.strike = message.next_double()?;
    position_multi.contract.right = message.next_string()?;
    position_multi.contract.multiplier = message.next_string()?;
    position_multi.contract.exchange = message.next_string()?;
    position_multi.contract.currency = message.next_string()?;
    position_multi.contract.local_symbol = message.next_string()?;

    if message_version >= 2 {
        position_multi.contract.trading_class = message.next_string()?;
    }

    position_multi.position = message.next_double()?;

    if message_version >= 3 {
        position_multi.average_cost = message.next_double()?;
    }

    Ok(position_multi)
}

pub(crate) fn family_code(message: &mut ResponseMessage) -> Result<FamilyCode, Error> {
    message.skip(); // message type

    let family_code = FamilyCode {
        account_id: message.next_string()?,
        family_code: message.next_string()?,
    };
   
    Ok(family_code)
}

mod tests {
    #[test]
    fn decode_positions() {
        let mut message = super::ResponseMessage::from("61\03\0DU1236109\076792991\0TSLA\0STK\0\00.0\0\0\0NASDAQ\0USD\0TSLA\0NMS\0500\0196.77\0");

        let results = super::position(&mut message);

        if let Ok(position) = results {
            assert_eq!(position.account, "DU1236109", "position.account");
            assert_eq!(position.contract.contract_id, 76792991, "position.contract.contract_id");
            assert_eq!(position.contract.symbol, "TSLA", "position.contract.symbol");
            assert_eq!(
                position.contract.security_type,
                super::SecurityType::Stock,
                "position.contract.security_type"
            );
            assert_eq!(
                position.contract.last_trade_date_or_contract_month, "",
                "position.contract.last_trade_date_or_contract_month"
            );
            assert_eq!(position.contract.strike, 0.0, "position.contract.strike");
            assert_eq!(position.contract.right, "", "position.contract.right");
            assert_eq!(position.contract.multiplier, "", "position.contract.multiplier");
            assert_eq!(position.contract.exchange, "NASDAQ", "position.contract.exchange");
            assert_eq!(position.contract.currency, "USD", "position.contract.currency");
            assert_eq!(position.contract.local_symbol, "TSLA", "position.contract.local_symbol");
            assert_eq!(position.contract.trading_class, "NMS", "position.contract.trading_class");
            assert_eq!(position.position, 500.0, "position.position");
            assert_eq!(position.average_cost, 196.77, "position.average_cost");
        } else if let Err(err) = results {
            assert!(false, "error decoding position: {err}");
        }
    }

    #[test]
    fn decode_positions_multi() {
        let mut message = super::ResponseMessage::from("61\03\0DU1236109\076792991\0TSLA\0STK\0\00.0\0\0\0NASDAQ\0USD\0TSLA\0NMS\0500\0196.77\0");

        let results = super::position_multi(&mut message);

        if let Ok(position_multi) = results {
            assert_eq!(position_multi.account, "DU1236109", "position.account");
            assert_eq!(position_multi.contract.contract_id, 76792991, "position.contract.contract_id");
            assert_eq!(position_multi.contract.symbol, "TSLA", "position.contract.symbol");
            assert_eq!(
                position_multi.contract.security_type,
                super::SecurityType::Stock,
                "position.contract.security_type"
            );
            assert_eq!(
                position_multi.contract.last_trade_date_or_contract_month, "",
                "position.contract.last_trade_date_or_contract_month"
            );
            assert_eq!(position_multi.contract.strike, 0.0, "position_multi.contract.strike");
            assert_eq!(position_multi.contract.right, "", "position_multi.contract.right");
            assert_eq!(position_multi.contract.multiplier, "", "position_multi.contract.multiplier");
            assert_eq!(position_multi.contract.exchange, "NASDAQ", "position_multi.contract.exchange");
            assert_eq!(position_multi.contract.currency, "USD", "position_multi.contract.currency");
            assert_eq!(position_multi.contract.local_symbol, "TSLA", "position_multi.contract.local_symbol");
            assert_eq!(position_multi.contract.trading_class, "NMS", "position_multi.contract.trading_class");
            assert_eq!(position_multi.position, 500.0, "position_multi.position");
            assert_eq!(position_multi.average_cost, 196.77, "position_multi.average_cost");
        } else if let Err(err) = results {
            assert!(false, "error decoding position mulit: {err}");
        }
    }

    #[test]
    fn decode_family_codes() {
        let mut message = super::ResponseMessage::from("0DU1236109\0F445566");

        let results = super::family_code(&mut message);

        if let Ok(family_code) = results {
            assert_eq!(family_code.account_id, "DU1236109", "family_code.account_id");
            assert_eq!(family_code.family_code, "F445566", "family_code.family_code");
        } else if let Err(err) = results {
            assert!(false, "error decoding family_code: {err}");
        }
    }
}

