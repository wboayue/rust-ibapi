use super::super::{Contract, SecurityType};
use crate::messages::OutgoingMessages;
use crate::messages::RequestMessage;
use crate::{server_versions, Error};

pub(crate) fn encode_request_contract_data(server_version: i32, request_id: i32, contract: &Contract) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 8;

    let mut packet = RequestMessage::default();

    packet.push_field(&OutgoingMessages::RequestContractData);
    packet.push_field(&VERSION);

    if server_version >= server_versions::CONTRACT_DATA_CHAIN {
        packet.push_field(&request_id);
    }

    if server_version >= server_versions::CONTRACT_CONID {
        packet.push_field(&contract.contract_id);
    }

    packet.push_field(&contract.symbol);
    packet.push_field(&contract.security_type);
    packet.push_field(&contract.last_trade_date_or_contract_month);
    packet.push_field(&contract.strike);
    packet.push_field(&contract.right);

    if server_version >= 15 {
        packet.push_field(&contract.multiplier);
    }

    if server_version >= server_versions::PRIMARYEXCH {
        packet.push_field(&contract.exchange);
        packet.push_field(&contract.primary_exchange);
    } else if server_version >= server_versions::LINKING {
        if !contract.primary_exchange.is_empty() && (contract.exchange.as_str() == "BEST" || contract.exchange.as_str() == "SMART") {
            packet.push_field(&format!("{}:{}", contract.exchange, contract.primary_exchange));
        } else {
            packet.push_field(&contract.exchange);
        }
    }

    packet.push_field(&contract.currency);
    packet.push_field(&contract.local_symbol);

    if server_version >= server_versions::TRADING_CLASS {
        packet.push_field(&contract.trading_class);
    }
    if server_version >= 31 {
        packet.push_field(&contract.include_expired);
    }
    if server_version >= server_versions::SEC_ID_TYPE {
        packet.push_field(&contract.security_id_type);
        packet.push_field(&contract.security_id);
    }
    if server_version >= server_versions::BOND_ISSUERID {
        packet.push_field(&contract.issuer_id);
    }

    Ok(packet)
}

pub(crate) fn encode_request_matching_symbols(request_id: i32, pattern: &str) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestMatchingSymbols);
    message.push_field(&request_id);
    message.push_field(&pattern);

    Ok(message)
}

pub(crate) fn encode_request_market_rule(market_rule_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestMarketRule);
    message.push_field(&market_rule_id);

    Ok(message)
}

pub(crate) fn encode_calculate_option_price(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    volatility: f64,
    underlying_price: f64,
) -> Result<RequestMessage, Error> {
    encode_option_computation(server_version, request_id, contract, volatility, underlying_price)
}

pub(crate) fn encode_calculate_implied_volatility(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    option_price: f64,
    underlying_price: f64,
) -> Result<RequestMessage, Error> {
    encode_option_computation(server_version, request_id, contract, option_price, underlying_price)
}

pub(crate) fn encode_option_computation(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    price_or_volatility: f64,
    underlying_price: f64,
) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 3;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::ReqCalcImpliedVolat);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    encode_contract(server_version, &mut message, contract);
    message.push_field(&price_or_volatility);
    message.push_field(&underlying_price);
    if server_version >= server_versions::LINKING {
        message.push_field(&"");
    }

    Ok(message)
}

fn encode_contract(server_version: i32, message: &mut RequestMessage, contract: &Contract) {
    message.push_field(&contract.contract_id);
    message.push_field(&contract.symbol);
    message.push_field(&contract.security_type);
    message.push_field(&contract.last_trade_date_or_contract_month);
    message.push_field(&contract.strike);
    message.push_field(&contract.right);
    message.push_field(&contract.multiplier);
    message.push_field(&contract.exchange);
    message.push_field(&contract.primary_exchange);
    message.push_field(&contract.currency);
    message.push_field(&contract.local_symbol);
    if server_version >= server_versions::TRADING_CLASS {
        message.push_field(&contract.trading_class);
    }
}

pub(crate) fn encode_cancel_option_computation(message_type: OutgoingMessages, request_id: i32) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();

    message.push_field(&message_type);
    message.push_field(&VERSION);
    message.push_field(&request_id);

    Ok(message)
}

pub(crate) fn encode_cancel_contract_data(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();
    message.push_field(&OutgoingMessages::CancelContractData);
    message.push_field(&request_id);
    Ok(message)
}

pub(in crate::contracts) fn encode_request_option_chain(
    request_id: i32,
    symbol: &str,
    exchange: &str,
    security_type: SecurityType,
    contract_id: i32,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestSecurityDefinitionOptionalParameters);
    message.push_field(&request_id);
    message.push_field(&symbol);
    message.push_field(&exchange);
    message.push_field(&security_type);
    message.push_field(&contract_id);

    Ok(message)
}

#[cfg(test)]
mod tests {
    use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
    use crate::messages::{OutgoingMessages, RequestMessage};
    use crate::{server_versions, ToField};

    #[test]
    fn test_encode_request_contract_data() {
        let server_version = server_versions::BOND_ISSUERID;
        let request_id = 1000;
        let message_version = 8;

        let contract = Contract {
            contract_id: 12345,
            symbol: Symbol::from("AAPL"),
            security_type: SecurityType::Stock,
            last_trade_date_or_contract_month: "".to_string(),
            strike: 0.0,
            right: "".to_string(),
            multiplier: "".to_string(),
            exchange: Exchange::from("SMART"),
            primary_exchange: Exchange::from("NASDAQ"),
            currency: Currency::from("USD"),
            local_symbol: "AAPL".to_string(),
            trading_class: "".to_string(),
            include_expired: false,
            security_id_type: "".to_string(),
            security_id: "".to_string(),
            issuer_id: "".to_string(),
            ..Default::default()
        };

        let message = super::encode_request_contract_data(server_version, request_id, &contract).expect("error encoding contract data request");

        assert_eq!(message[0], OutgoingMessages::RequestContractData.to_field(), "message.type");
        assert_eq!(message[1], message_version.to_field(), "message.version");
        assert_eq!(message[2], request_id.to_field(), "message.request_id");
        assert_eq!(message[3], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(message[4], contract.symbol.to_field(), "message.symbol");
        assert_eq!(message[5], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            message[6], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(message[7], contract.strike.to_field(), "message.strike");
        assert_eq!(message[8], contract.right, "message.right");
        assert_eq!(message[9], contract.multiplier, "message.multiplier");
        assert_eq!(message[10], contract.exchange.to_field(), "message.exchange");
        assert_eq!(message[11], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(message[12], contract.currency.to_field(), "message.currency");
        assert_eq!(message[13], contract.local_symbol, "message.local_symbol");
        assert_eq!(message[14], contract.trading_class, "message.trading_class");
        assert_eq!(message[15], contract.include_expired.to_field(), "message.include_expired");
        assert_eq!(message[16], contract.security_id_type, "message.security_id_type");
        assert_eq!(message[17], contract.security_id, "message.security_id");
        assert_eq!(message[18], contract.issuer_id, "message.issuer_id");
    }

    #[test]
    fn test_encode_request_matching_symbols() {
        let request_id = 2000;
        let pattern = "AAPL";

        let message = super::encode_request_matching_symbols(request_id, pattern).expect("error encoding matching symbols request");

        assert_eq!(message[0], OutgoingMessages::RequestMatchingSymbols.to_field(), "message.type");
        assert_eq!(message[1], request_id.to_field(), "message.request_id");
        assert_eq!(message[2], pattern, "message.pattern");
    }

    #[test]
    fn test_encode_request_market_rule() {
        let market_rule_id = 26;

        let message = super::encode_request_market_rule(market_rule_id).expect("error encoding market rule request");

        assert_eq!(message[0], OutgoingMessages::RequestMarketRule.to_field(), "message.type");
        assert_eq!(message[1], market_rule_id.to_field(), "message.market_rule_id");
    }

    #[test]
    fn test_encode_calculate_option_price() {
        let server_version = server_versions::LINKING;
        let request_id = 3000;
        let message_version = 3;

        let contract = Contract {
            contract_id: 67890,
            symbol: Symbol::from("AAPL"),
            security_type: SecurityType::Option,
            last_trade_date_or_contract_month: "20231215".to_string(),
            strike: 150.0,
            right: "C".to_string(),
            multiplier: "100".to_string(),
            exchange: Exchange::from("SMART"),
            primary_exchange: Exchange::from("CBOE"),
            currency: Currency::from("USD"),
            local_symbol: "AAPL  231215C00150000".to_string(),
            trading_class: "AAPL".to_string(),
            include_expired: false,
            security_id_type: "".to_string(),
            security_id: "".to_string(),
            issuer_id: "".to_string(),
            ..Default::default()
        };

        let volatility = 0.3;
        let underlying_price = 145.0;

        let message = super::encode_calculate_option_price(server_version, request_id, &contract, volatility, underlying_price)
            .expect("error encoding calculate option price request");

        assert_eq!(message[0], OutgoingMessages::ReqCalcImpliedVolat.to_field(), "message.type");
        assert_eq!(message[1], message_version.to_field(), "message.version");
        assert_eq!(message[2], request_id.to_field(), "message.request_id");

        // Assert contract fields (index 3 to 14)
        assert_eq!(message[3], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(message[4], contract.symbol.to_field(), "message.symbol");
        assert_eq!(message[5], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            message[6], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(message[7], contract.strike.to_field(), "message.strike");
        assert_eq!(message[8], contract.right, "message.right");
        assert_eq!(message[9], contract.multiplier, "message.multiplier");
        assert_eq!(message[10], contract.exchange.to_field(), "message.exchange");
        assert_eq!(message[11], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(message[12], contract.currency.to_field(), "message.currency");
        assert_eq!(message[13], contract.local_symbol, "message.local_symbol");
        assert_eq!(message[14], contract.trading_class, "message.trading_class");

        assert_eq!(message[15], volatility.to_field(), "message.volatility");
        assert_eq!(message[16], underlying_price.to_field(), "message.underlying_price");
        assert_eq!(message[17], "", "message.empty_field");
    }

    #[test]
    fn test_encode_calculate_implied_volatility() {
        let server_version = server_versions::LINKING;
        let request_id = 4000;
        let contract = Contract {
            contract_id: 67890,
            symbol: Symbol::from("AAPL"),
            security_type: SecurityType::Option,
            last_trade_date_or_contract_month: "20231215".to_string(),
            strike: 150.0,
            right: "C".to_string(),
            multiplier: "100".to_string(),
            exchange: Exchange::from("SMART"),
            primary_exchange: Exchange::from("CBOE"),
            currency: Currency::from("USD"),
            local_symbol: "AAPL  231215C00150000".to_string(),
            trading_class: "AAPL".to_string(),
            include_expired: false,
            security_id_type: "".to_string(),
            security_id: "".to_string(),
            issuer_id: "".to_string(),
            ..Default::default()
        };

        let option_price = 5.0;
        let underlying_price = 145.0;

        let message = super::encode_calculate_implied_volatility(server_version, request_id, &contract, option_price, underlying_price)
            .expect("error encoding calculate implied volatility request");

        assert_eq!(message[0], OutgoingMessages::ReqCalcImpliedVolat.to_field(), "message.type");
        assert_eq!(message[1], "3".to_string(), "message.version");
        assert_eq!(message[2], request_id.to_field(), "message.request_id");

        // Assert contract fields (index 3 to 14)
        assert_eq!(message[3], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(message[4], contract.symbol.to_field(), "message.symbol");
        assert_eq!(message[5], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            message[6], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(message[7], contract.strike.to_field(), "message.strike");
        assert_eq!(message[8], contract.right, "message.right");
        assert_eq!(message[9], contract.multiplier, "message.multiplier");
        assert_eq!(message[10], contract.exchange.to_field(), "message.exchange");
        assert_eq!(message[11], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(message[12], contract.currency.to_field(), "message.currency");
        assert_eq!(message[13], contract.local_symbol, "message.local_symbol");
        assert_eq!(message[14], contract.trading_class, "message.trading_class");

        assert_eq!(message[15], option_price.to_field(), "message.option_price");
        assert_eq!(message[16], underlying_price.to_field(), "message.underlying_price");
        assert_eq!(message[17], "", "message.empty_field");
    }

    #[test]
    fn test_encode_contract() {
        let server_version = server_versions::TRADING_CLASS;

        let contract = Contract {
            contract_id: 12345,
            symbol: Symbol::from("AAPL"),
            security_type: SecurityType::Stock,
            last_trade_date_or_contract_month: "".to_string(),
            strike: 0.0,
            right: "".to_string(),
            multiplier: "".to_string(),
            exchange: Exchange::from("SMART"),
            primary_exchange: Exchange::from("NASDAQ"),
            currency: Currency::from("USD"),
            local_symbol: "AAPL".to_string(),
            trading_class: "AAPL".to_string(),
            include_expired: false,
            security_id_type: "".to_string(),
            security_id: "".to_string(),
            issuer_id: "".to_string(),
            ..Default::default()
        };

        let mut message = RequestMessage::default();

        super::encode_contract(server_version, &mut message, &contract);

        assert_eq!(message[0], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(message[1], contract.symbol.to_field(), "message.symbol");
        assert_eq!(message[2], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            message[3], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(message[4], contract.strike.to_field(), "message.strike");
        assert_eq!(message[5], contract.right, "message.right");
        assert_eq!(message[6], contract.multiplier, "message.multiplier");
        assert_eq!(message[7], contract.exchange.to_field(), "message.exchange");
        assert_eq!(message[8], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(message[9], contract.currency.to_field(), "message.currency");
        assert_eq!(message[10], contract.local_symbol, "message.local_symbol");
        assert_eq!(message[11], contract.trading_class, "message.trading_class");
    }

    #[test]
    fn test_encode_cancel_contract_data() {
        let request_id = 5000;

        let message = super::encode_cancel_contract_data(request_id).expect("error encoding cancel contract data");

        assert_eq!(message[0], OutgoingMessages::CancelContractData.to_field(), "message.type");
        assert_eq!(message[1], request_id.to_field(), "message.request_id");
    }

    #[test]
    fn test_encode_cancel_option_calculation() {
        let message_type = OutgoingMessages::CancelOptionPrice;
        let message_version = 1;
        let request_id = 2000;

        let message = super::encode_cancel_option_computation(message_type, request_id).expect("error encoding cancel option computation");

        assert_eq!(message[0], message_type.to_field(), "message.type");
        assert_eq!(message[1], message_version.to_field(), "message.message_version");
        assert_eq!(message[2], request_id.to_field(), "message.request_id");
    }
}
