use super::*;

use crate::common::test_utils::helpers::{assert_missing_field, assert_rejects_text_framing};
use crate::messages::IncomingMessages;
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
fn test_decode_contract_data_proto_rejects_missing_submessages() {
    // EDecoder.cs drops a ContractData frame when either submessage is null
    // rather than synthesizing a default; this crate has no skip channel, so
    // it errors, as #829's OpenOrder / CompletedOrder / ExecutionDetails do.
    let full = crate::proto::ContractData {
        req_id: Some(1),
        contract: Some(crate::proto::Contract {
            sec_type: Some("STK".into()),
            ..Default::default()
        }),
        contract_details: Some(crate::proto::ContractDetails::default()),
    };
    decode_contract_data_proto(&full.encode_to_vec()).expect("control frame must decode");

    for (name, frame) in [
        (
            "contract",
            crate::proto::ContractData {
                contract: None,
                ..full.clone()
            },
        ),
        (
            "contract_details",
            crate::proto::ContractData {
                contract_details: None,
                ..full.clone()
            },
        ),
    ] {
        assert_missing_field(decode_contract_data_proto(&frame.encode_to_vec()), name, "ContractData");
    }
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
    let result = decode_symbol_samples_proto(proto_msg).unwrap();
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
    let result = decode_market_rule_proto(proto_msg).unwrap();
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
// raises `UnexpectedWireFormat` — the message was addressed to this decoder, so
// it is not skippable. See docs/rules/wire/proto-only-decoding.md.

#[test]
fn test_decode_contract_details_rejects_text_framing() {
    assert_rejects_text_framing(
        IncomingMessages::ContractData,
        "10\09001\0AAPL\0STK\0\00\0\0SMART\0USD\0AAPL\0NMS\0NMS\0265598\00.01\0\0",
        decode_contract_details,
    );
}

#[test]
fn test_decode_option_chain_rejects_text_framing() {
    assert_rejects_text_framing(
        IncomingMessages::SecurityDefinitionOptionParameter,
        "75\09000\0SMART\0265598\0AAPL\0100\01\020260619\01\0150.0\0",
        decode_option_chain,
    );
}

fn bond_frame(last_trade_date_or_contract_month: &str) -> ResponseMessage {
    use crate::common::test_utils::helpers::proto_response;
    use crate::testdata::builders::contracts::contract_data;
    use crate::testdata::builders::ResponseProtoEncoder;

    proto_response(
        IncomingMessages::BondContractData,
        contract_data()
            .request_id(9000)
            .contract_id(15960430)
            .security_type("BOND")
            .last_trade_date_or_contract_month(last_trade_date_or_contract_month)
            .time_zone_id("US/Eastern")
            .encode_proto(),
    )
}

#[test]
fn test_decode_bond_contract_details_splits_last_trade_date() {
    // Mirrors C# EDecoderUtils.SetLastTradeDate(.., isBond: true).
    let cases = [
        ("20430504 16:00 Europe/London", "20430504", "16:00", "Europe/London"),
        ("20430504-16:00", "20430504", "16:00", "US/Eastern"),
        ("20430504", "20430504", "", "US/Eastern"),
        ("", "", "", "US/Eastern"),
    ];
    for (raw, maturity, time, zone) in cases {
        let details = decode_bond_contract_details(&bond_frame(raw)).expect("bond frame decodes");
        assert_eq!(details.maturity, maturity, "maturity for {raw:?}");
        assert_eq!(details.last_trade_time, time, "time for {raw:?}");
        assert_eq!(details.time_zone_id, zone, "zone for {raw:?}");
        // C# leaves the contract field as sent for bonds.
        assert_eq!(details.contract.last_trade_date_or_contract_month, raw);
    }
}

#[test]
fn test_decode_contract_details_leaves_maturity_unset() {
    // The split is bond-only: a ContractData frame keeps maturity empty.
    use crate::common::test_utils::helpers::proto_response;
    use crate::testdata::builders::contracts::contract_data;
    use crate::testdata::builders::ResponseProtoEncoder;

    let frame = proto_response(
        IncomingMessages::ContractData,
        contract_data()
            .request_id(9000)
            .last_trade_date_or_contract_month("20430504")
            .encode_proto(),
    );
    let details = decode_contract_details(&frame).unwrap();
    assert_eq!(details.maturity, "");
    assert_eq!(details.contract.last_trade_date_or_contract_month, "20430504");
}

#[test]
fn test_decode_bond_contract_details_rejects_text_framing() {
    assert_rejects_text_framing(IncomingMessages::BondContractData, "18\09001\0\0BOND\0", decode_bond_contract_details);
}

#[test]
fn test_decode_bond_contract_details_live_shape() {
    // Shape of a live `BondContractData` frame (US-T, 2026-10-01, server 225):
    // no symbol, no last trade date, no coupon; identity in local symbol and cusip.
    let proto_msg = crate::proto::ContractData {
        req_id: Some(9000),
        contract: Some(crate::proto::Contract {
            con_id: Some(15960430),
            sec_type: Some("BOND".into()),
            exchange: Some("SMART".into()),
            local_symbol: Some("IBCID15960430".into()),
            trading_class: Some("US-T".into()),
            ..Default::default()
        }),
        contract_details: Some(crate::proto::ContractDetails {
            min_tick: Some("0.00001".into()),
            contract_month: Some("202611".into()),
            time_zone_id: Some("US/Eastern".into()),
            sec_id_list: [("ISIN".to_string(), "US912810EY02".to_string())].into(),
            cusip: Some("IBCID15960430".into()),
            desc_append: Some("T 6 1/2 11/15/26".into()),
            ..Default::default()
        }),
    };
    let frame = ResponseMessage::from_protobuf(IncomingMessages::BondContractData as i32, proto_msg.encode_to_vec());

    let details = decode_bond_contract_details(&frame).unwrap();
    assert_eq!(details.contract.contract_id, 15960430);
    assert_eq!(details.contract.security_type, crate::contracts::SecurityType::Bond);
    assert_eq!(details.contract.trading_class, "US-T");
    assert_eq!(details.contract.symbol, crate::contracts::Symbol::from(""));
    assert_eq!(details.cusip, "IBCID15960430");
    assert_eq!(details.desc_append, "T 6 1/2 11/15/26");
    assert_eq!(details.contract_month, "202611");
    assert_eq!(details.time_zone_id, "US/Eastern");
    assert_eq!(details.maturity, "", "no last trade date on the wire, so no maturity");
}
