use crate::contracts::{Contract, SecurityType};
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::{server_versions, ToField};

#[test]
fn test_encode_request_contract_data() {
    let server_version = server_versions::BOND_ISSUERID;
    let request_id = 1000;
    let message_version = 8;

    let contract = Contract {
        contract_id: 12345,
        symbol: "AAPL".to_string(),
        security_type: SecurityType::Stock,
        last_trade_date_or_contract_month: "".to_string(),
        strike: 0.0,
        right: "".to_string(),
        multiplier: "".to_string(),
        exchange: "SMART".to_string(),
        primary_exchange: "NASDAQ".to_string(),
        currency: "USD".to_string(),
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
    assert_eq!(message[4], contract.symbol, "message.symbol");
    assert_eq!(message[5], contract.security_type.to_field(), "message.security_type");
    assert_eq!(
        message[6], contract.last_trade_date_or_contract_month,
        "message.last_trade_date_or_contract_month"
    );
    assert_eq!(message[7], contract.strike.to_field(), "message.strike");
    assert_eq!(message[8], contract.right, "message.right");
    assert_eq!(message[9], contract.multiplier, "message.multiplier");
    assert_eq!(message[10], contract.exchange, "message.exchange");
    assert_eq!(message[11], contract.primary_exchange, "message.primary_exchange");
    assert_eq!(message[12], contract.currency, "message.currency");
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
        symbol: "AAPL".to_string(),
        security_type: SecurityType::Option,
        last_trade_date_or_contract_month: "20231215".to_string(),
        strike: 150.0,
        right: "C".to_string(),
        multiplier: "100".to_string(),
        exchange: "SMART".to_string(),
        primary_exchange: "CBOE".to_string(),
        currency: "USD".to_string(),
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
    assert_eq!(message[4], contract.symbol, "message.symbol");
    assert_eq!(message[5], contract.security_type.to_field(), "message.security_type");
    assert_eq!(
        message[6], contract.last_trade_date_or_contract_month,
        "message.last_trade_date_or_contract_month"
    );
    assert_eq!(message[7], contract.strike.to_field(), "message.strike");
    assert_eq!(message[8], contract.right, "message.right");
    assert_eq!(message[9], contract.multiplier, "message.multiplier");
    assert_eq!(message[10], contract.exchange, "message.exchange");
    assert_eq!(message[11], contract.primary_exchange, "message.primary_exchange");
    assert_eq!(message[12], contract.currency, "message.currency");
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
        symbol: "AAPL".to_string(),
        security_type: SecurityType::Option,
        last_trade_date_or_contract_month: "20231215".to_string(),
        strike: 150.0,
        right: "C".to_string(),
        multiplier: "100".to_string(),
        exchange: "SMART".to_string(),
        primary_exchange: "CBOE".to_string(),
        currency: "USD".to_string(),
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
    assert_eq!(message[4], contract.symbol, "message.symbol");
    assert_eq!(message[5], contract.security_type.to_field(), "message.security_type");
    assert_eq!(
        message[6], contract.last_trade_date_or_contract_month,
        "message.last_trade_date_or_contract_month"
    );
    assert_eq!(message[7], contract.strike.to_field(), "message.strike");
    assert_eq!(message[8], contract.right, "message.right");
    assert_eq!(message[9], contract.multiplier, "message.multiplier");
    assert_eq!(message[10], contract.exchange, "message.exchange");
    assert_eq!(message[11], contract.primary_exchange, "message.primary_exchange");
    assert_eq!(message[12], contract.currency, "message.currency");
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
        symbol: "AAPL".to_string(),
        security_type: SecurityType::Stock,
        last_trade_date_or_contract_month: "".to_string(),
        strike: 0.0,
        right: "".to_string(),
        multiplier: "".to_string(),
        exchange: "SMART".to_string(),
        primary_exchange: "NASDAQ".to_string(),
        currency: "USD".to_string(),
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
    assert_eq!(message[1], contract.symbol, "message.symbol");
    assert_eq!(message[2], contract.security_type.to_field(), "message.security_type");
    assert_eq!(
        message[3], contract.last_trade_date_or_contract_month,
        "message.last_trade_date_or_contract_month"
    );
    assert_eq!(message[4], contract.strike.to_field(), "message.strike");
    assert_eq!(message[5], contract.right, "message.right");
    assert_eq!(message[6], contract.multiplier, "message.multiplier");
    assert_eq!(message[7], contract.exchange, "message.exchange");
    assert_eq!(message[8], contract.primary_exchange, "message.primary_exchange");
    assert_eq!(message[9], contract.currency, "message.currency");
    assert_eq!(message[10], contract.local_symbol, "message.local_symbol");
    assert_eq!(message[11], contract.trading_class, "message.trading_class");
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
