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

// Tests intentionally omitted: encoder coverage is provided end-to-end via
// `assert_request<B>` in `src/contracts/{sync,async}/tests.rs` and via
// `test_cancel_messages` exercising `OptionComputation::cancel_message` in
// `stream_decoders`. The unsupported-message-type branch in
// `encode_cancel_option_computation` is unreachable from production callers
// (the `match` in `cancel_message` only dispatches to the two supported
// variants), so an explicit error-path test would only verify hypothetical
// callers.
