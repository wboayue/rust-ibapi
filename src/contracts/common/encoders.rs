use super::super::{Contract, SecurityType};
use crate::messages::OutgoingMessages;
use crate::Error;

pub(crate) fn encode_request_contract_data(request_id: i32, contract: &Contract) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::ContractDataRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestContractData as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_request_matching_symbols(request_id: i32, pattern: &str) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::MatchingSymbolsRequest {
        req_id: Some(request_id),
        pattern: Some(pattern.to_string()),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestMatchingSymbols as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_request_market_rule(market_rule_id: i32) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::MarketRuleRequest {
        market_rule_id: Some(market_rule_id),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestMarketRule as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_calculate_option_price(request_id: i32, contract: &Contract, volatility: f64, underlying_price: f64) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::CalculateOptionPriceRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        volatility: Some(volatility),
        under_price: Some(underlying_price),
        option_price_options: Default::default(),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::ReqCalcOptionPrice as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_calculate_implied_volatility(
    request_id: i32,
    contract: &Contract,
    option_price: f64,
    underlying_price: f64,
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::CalculateImpliedVolatilityRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        option_price: Some(option_price),
        under_price: Some(underlying_price),
        implied_volatility_options: Default::default(),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::ReqCalcImpliedVolat as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_cancel_option_computation(message_type: OutgoingMessages, request_id: i32) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    match message_type {
        OutgoingMessages::CancelOptionPrice => {
            let request = crate::proto::CancelCalculateOptionPrice { req_id: Some(request_id) };
            Ok(encode_protobuf_message(
                OutgoingMessages::CancelOptionPrice as i32,
                &request.encode_to_vec(),
            ))
        }
        OutgoingMessages::CancelImpliedVolatility => {
            let request = crate::proto::CancelCalculateImpliedVolatility { req_id: Some(request_id) };
            Ok(encode_protobuf_message(
                OutgoingMessages::CancelImpliedVolatility as i32,
                &request.encode_to_vec(),
            ))
        }
        _ => Err(Error::Simple(format!(
            "unexpected message type for cancel option computation: {message_type:?}"
        ))),
    }
}

pub(crate) fn encode_cancel_contract_data(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelContractData, OutgoingMessages::CancelContractData)
}

pub(in crate::contracts) fn encode_request_option_chain(
    request_id: i32,
    symbol: &str,
    exchange: &str,
    security_type: SecurityType,
    contract_id: i32,
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::SecDefOptParamsRequest {
        req_id: Some(request_id),
        underlying_symbol: Some(symbol.to_string()),
        fut_fop_exchange: Some(exchange.to_string()),
        underlying_sec_type: Some(security_type.to_string()),
        underlying_con_id: Some(contract_id),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestSecurityDefinitionOptionalParameters as i32,
        &request.encode_to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
    use crate::messages::OutgoingMessages;

    #[test]
    fn test_encode_request_contract_data() {
        let contract = Contract {
            contract_id: 265598,
            symbol: Symbol::from("AAPL"),
            security_type: SecurityType::Stock,
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            ..Default::default()
        };
        let bytes = encode_request_contract_data(1000, &contract).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, OutgoingMessages::RequestContractData as i32 + 200);
        use prost::Message;
        let req = crate::proto::ContractDataRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(1000));
        assert_eq!(req.contract.unwrap().con_id, Some(265598));
    }

    #[test]
    fn test_encode_request_matching_symbols() {
        let bytes = encode_request_matching_symbols(2000, "AAPL").unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, OutgoingMessages::RequestMatchingSymbols as i32 + 200);
        use prost::Message;
        let req = crate::proto::MatchingSymbolsRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(2000));
        assert_eq!(req.pattern, Some("AAPL".to_string()));
    }

    #[test]
    fn test_encode_request_market_rule() {
        let bytes = encode_request_market_rule(26).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, OutgoingMessages::RequestMarketRule as i32 + 200);
        use prost::Message;
        let req = crate::proto::MarketRuleRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.market_rule_id, Some(26));
    }

    #[test]
    fn test_encode_calculate_option_price() {
        let contract = Contract {
            contract_id: 67890,
            symbol: Symbol::from("AAPL"),
            security_type: SecurityType::Option,
            last_trade_date_or_contract_month: "20231215".to_string(),
            strike: 150.0,
            right: "C".to_string(),
            multiplier: "100".to_string(),
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            ..Default::default()
        };
        let bytes = encode_calculate_option_price(3000, &contract, 0.3, 145.0).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, OutgoingMessages::ReqCalcOptionPrice as i32 + 200);
        use prost::Message;
        let req = crate::proto::CalculateOptionPriceRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(3000));
        assert_eq!(req.volatility, Some(0.3));
        assert_eq!(req.under_price, Some(145.0));
        assert_eq!(req.contract.unwrap().con_id, Some(67890));
    }

    #[test]
    fn test_encode_calculate_implied_volatility() {
        let contract = Contract {
            contract_id: 67890,
            symbol: Symbol::from("AAPL"),
            security_type: SecurityType::Option,
            strike: 150.0,
            right: "C".to_string(),
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            ..Default::default()
        };
        let bytes = encode_calculate_implied_volatility(4000, &contract, 5.0, 145.0).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, OutgoingMessages::ReqCalcImpliedVolat as i32 + 200);
        use prost::Message;
        let req = crate::proto::CalculateImpliedVolatilityRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(4000));
        assert_eq!(req.option_price, Some(5.0));
        assert_eq!(req.under_price, Some(145.0));
        assert_eq!(req.contract.unwrap().con_id, Some(67890));
    }

    #[test]
    fn test_encode_cancel_contract_data() {
        let bytes = encode_cancel_contract_data(5000).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, OutgoingMessages::CancelContractData as i32 + 200);
    }

    #[test]
    fn test_encode_cancel_option_computation_price() {
        let bytes = encode_cancel_option_computation(OutgoingMessages::CancelOptionPrice, 2000).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, OutgoingMessages::CancelOptionPrice as i32 + 200);
        use prost::Message;
        let req = crate::proto::CancelCalculateOptionPrice::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(2000));
    }

    #[test]
    fn test_encode_cancel_option_computation_volatility() {
        let bytes = encode_cancel_option_computation(OutgoingMessages::CancelImpliedVolatility, 3000).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, OutgoingMessages::CancelImpliedVolatility as i32 + 200);
        use prost::Message;
        let req = crate::proto::CancelCalculateImpliedVolatility::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(3000));
    }

    #[test]
    fn test_encode_request_option_chain() {
        let bytes = encode_request_option_chain(6000, "AAPL", "", SecurityType::Stock, 265598).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, OutgoingMessages::RequestSecurityDefinitionOptionalParameters as i32 + 200);
        use prost::Message;
        let req = crate::proto::SecDefOptParamsRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(6000));
        assert_eq!(req.underlying_symbol, Some("AAPL".to_string()));
        assert_eq!(req.underlying_sec_type, Some("STK".to_string()));
        assert_eq!(req.underlying_con_id, Some(265598));
    }
}
