use super::*;
use crate::server_versions;

use prost::Message;

#[test]
fn test_decode_contract_data_proto() {
    let proto_msg = crate::proto::ContractData {
        req_id: Some(1),
        contract: Some(crate::proto::Contract {
            con_id: Some(265598),
            symbol: Some("AAPL".into()),
            sec_type: Some("STK".into()),
            exchange: Some("SMART".into()),
            currency: Some("USD".into()),
            local_symbol: Some("AAPL".into()),
            trading_class: Some("NMS".into()),
            ..Default::default()
        }),
        contract_details: Some(crate::proto::ContractDetails {
            market_name: Some("NMS".into()),
            min_tick: Some("0.01".into()),
            long_name: Some("APPLE INC".into()),
            industry: Some("Technology".into()),
            category: Some("Computers".into()),
            subcategory: Some("Consumer Electronics".into()),
            ..Default::default()
        }),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_contract_data_proto(&bytes).unwrap();
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.contract.currency.to_string(), "USD");
    assert_eq!(result.contract.local_symbol, "AAPL");
    assert_eq!(result.market_name, "NMS");
    assert_eq!(result.min_tick, 0.01);
    assert_eq!(result.long_name, "APPLE INC");
    assert_eq!(result.industry, "Technology");
    assert_eq!(result.category, "Computers");
    assert_eq!(result.subcategory, "Consumer Electronics");
}

#[test]
fn test_decode_symbol_samples_proto() {
    let proto_msg = crate::proto::SymbolSamples {
        req_id: Some(1),
        contract_descriptions: vec![
            crate::proto::ContractDescription {
                contract: Some(crate::proto::Contract {
                    con_id: Some(265598),
                    symbol: Some("AAPL".into()),
                    sec_type: Some("STK".into()),
                    primary_exch: Some("NASDAQ".into()),
                    currency: Some("USD".into()),
                    ..Default::default()
                }),
                derivative_sec_types: vec!["OPT".into(), "WAR".into()],
            },
            crate::proto::ContractDescription {
                contract: Some(crate::proto::Contract {
                    con_id: Some(76792991),
                    symbol: Some("TSLA".into()),
                    sec_type: Some("STK".into()),
                    primary_exch: Some("NASDAQ".into()),
                    currency: Some("USD".into()),
                    ..Default::default()
                }),
                derivative_sec_types: vec![],
            },
        ],
    };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_symbol_samples_proto(&bytes).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].contract.contract_id, 265598);
    assert_eq!(result[0].contract.symbol.to_string(), "AAPL");
    assert_eq!(result[0].derivative_security_types, vec!["OPT", "WAR"]);
    assert_eq!(result[1].contract.contract_id, 76792991);
    assert!(result[1].derivative_security_types.is_empty());
}

#[test]
fn test_decode_market_rule_proto() {
    let proto_msg = crate::proto::MarketRule {
        market_rule_id: Some(26),
        price_increments: vec![
            crate::proto::PriceIncrement {
                low_edge: Some(0.0),
                increment: Some(0.01),
            },
            crate::proto::PriceIncrement {
                low_edge: Some(1000.0),
                increment: Some(0.05),
            },
        ],
    };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_market_rule_proto(&bytes).unwrap();
    assert_eq!(result.market_rule_id, 26);
    assert_eq!(result.price_increments.len(), 2);
    assert_eq!(result.price_increments[0].low_edge, 0.0);
    assert_eq!(result.price_increments[0].increment, 0.01);
    assert_eq!(result.price_increments[1].low_edge, 1000.0);
    assert_eq!(result.price_increments[1].increment, 0.05);
}

#[test]
fn test_decode_option_chain_proto() {
    let proto_msg = crate::proto::SecDefOptParameter {
        req_id: Some(1),
        exchange: Some("SMART".into()),
        underlying_con_id: Some(265598),
        trading_class: Some("AAPL".into()),
        multiplier: Some("100".into()),
        expirations: vec!["20260619".into(), "20260918".into()],
        strikes: vec![150.0, 175.0, 200.0],
    };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_option_chain_proto(&bytes).unwrap();
    assert_eq!(result.exchange, "SMART");
    assert_eq!(result.underlying_contract_id, 265598);
    assert_eq!(result.trading_class, "AAPL");
    assert_eq!(result.multiplier, "100");
    assert_eq!(result.expirations, vec!["20260619", "20260918"]);
    assert_eq!(result.strikes, vec![150.0, 175.0, 200.0]);
}

// Servers ≥ the connection floor always emit ContractData / SymbolSamples /
// MarketRule / SecurityDefinitionOptionParameter in proto. Text-framed arrival
// skip-classifies via `UnexpectedResponse` (rule 20) rather than terminating
// the subscription.

#[test]
fn test_decode_contract_details_rejects_text_framing() {
    let mut message = ResponseMessage::from("10\09001\0AAPL\0STK\0\00\0\0SMART\0USD\0AAPL\0NMS\0NMS\0265598\00.01\0\0");
    let err = decode_contract_details(server_versions::PROTOBUF_REST_MESSAGES_3, &mut message).expect_err("text framing must be rejected");
    assert!(
        matches!(err, Error::UnexpectedResponse(_)),
        "expected Error::UnexpectedResponse, got {err:?}"
    );
}

#[test]
fn test_decode_contract_descriptions_rejects_text_framing() {
    let mut message = ResponseMessage::from("79\09000\01\012345\0AAPL\0STK\0NASDAQ\0USD\00\0APPLE INC\0\0");
    let err = decode_contract_descriptions(server_versions::PROTOBUF_REST_MESSAGES_3, &mut message).expect_err("text framing must be rejected");
    assert!(
        matches!(err, Error::UnexpectedResponse(_)),
        "expected Error::UnexpectedResponse, got {err:?}"
    );
}

#[test]
fn test_decode_market_rule_rejects_text_framing() {
    let mut message = ResponseMessage::from("87\026\01\00\00.01\0");
    let err = decode_market_rule(&mut message).expect_err("text framing must be rejected");
    assert!(
        matches!(err, Error::UnexpectedResponse(_)),
        "expected Error::UnexpectedResponse, got {err:?}"
    );
}

#[test]
fn test_decode_option_chain_rejects_text_framing() {
    let mut message = ResponseMessage::from("75\09000\0SMART\0265598\0AAPL\0100\01\020260619\01\0150.0\0");
    let err = decode_option_chain(&mut message).expect_err("text framing must be rejected");
    assert!(
        matches!(err, Error::UnexpectedResponse(_)),
        "expected Error::UnexpectedResponse, got {err:?}"
    );
}
