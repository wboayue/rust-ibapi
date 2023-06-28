use crate::contracts::SecurityType;
use crate::messages::ResponseMessage;
use crate::Error;

use super::{Position, FamilyCode};

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

pub(crate) fn family_code(message: &mut ResponseMessage) -> Result<Vec<FamilyCode>, Error> {
    message.skip(); // message type

    let _request_id = message.next_int()?;
    let family_codes_count = message.next_int()?;

    if family_codes_count < 1 {
        return Ok(Vec::default());
    }

    let mut family_codes: Vec<FamilyCode> = Vec::with_capacity(family_codes_count as usize);

    for _ in 0.. family_codes_count {
        let family_code = FamilyCode {
            account_id: message.next_string()?,
            family_code: message.next_string()?,
            ..Default::default()
            
        };
        family_codes.push(family_code);
      }
   
    Ok(family_codes)
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
    fn decode_family_codes() {
        let mut message = super::ResponseMessage::from("0account1\0code1");

        let results = super::family_code(&mut message);

        if let Ok(family_codes) = results {
            assert_eq!(family_codes[0].account_id, "account1", "family_codes.account_id");
            assert_eq!(family_codes[0].family_code, "code1", "family_codes.family_code");
        } else if let Err(err) = results {
            panic!("Error decoding family_codes: {}", err);
            // assert!(false, "error decoding family_code: {}", err);
        }
    }
}

