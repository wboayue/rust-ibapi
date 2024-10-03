#[test]
fn decode_positions() {
    let mut message = super::ResponseMessage::from("61\03\0DU1236109\076792991\0TSLA\0STK\0\00.0\0\0\0NASDAQ\0USD\0TSLA\0NMS\0500\0196.77\0");

    let results = super::decode_position(&mut message);

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
    let mut message = super::ResponseMessage::from("78\01\0*\0\0");

    let results = super::decode_family_codes(&mut message);

    if let Ok(family_codes) = results {
        assert_eq!(family_codes[0].account_id, "*", "family_codes.account_id");
        assert_eq!(family_codes[0].family_code, "", "family_codes.family_code");
    } else if let Err(err) = results {
        panic!("Error decoding family_codes: {}", err);
    }
}
