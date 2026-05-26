#[test]
fn test_decode_family_codes_rejects_text_framing() {
    let message = super::ResponseMessage::from("78\02\0ACC1\0FC1\0ACC2\0FC2\0");
    let err = super::decode_family_codes(&message).unwrap_err();
    assert!(
        matches!(err, super::Error::UnexpectedResponse(_)),
        "expected UnexpectedResponse, got {err:?}"
    );
}

#[test]
fn test_decode_managed_accounts_rejects_text_framing() {
    let message = super::ResponseMessage::from("15\01\0DU1234567,DU7654321\0");
    let err = super::decode_managed_accounts(&message).unwrap_err();
    assert!(
        matches!(err, super::Error::UnexpectedResponse(_)),
        "expected UnexpectedResponse, got {err:?}"
    );
}

#[test]
fn test_decode_server_time_rejects_text_framing() {
    let message = super::ResponseMessage::from("49\01\01678890000\0");
    let err = super::decode_server_time(&message).unwrap_err();
    assert!(
        matches!(err, super::Error::UnexpectedResponse(_)),
        "expected UnexpectedResponse, got {err:?}"
    );
}

#[test]
fn test_decode_server_time_millis_rejects_text_framing() {
    let message = super::ResponseMessage::from("109\01678890000123\0");
    let err = super::decode_server_time_millis(&message).unwrap_err();
    assert!(
        matches!(err, super::Error::UnexpectedResponse(_)),
        "expected UnexpectedResponse, got {err:?}"
    );
}

#[test]
fn test_decode_server_time_proto_via_builder() {
    use crate::testdata::builders::accounts::current_time;
    use crate::testdata::builders::ResponseProtoEncoder;
    use time::macros::datetime;

    let bytes = current_time().timestamp(1678890000).encode_proto();
    let result = super::decode_server_time_proto(&bytes).unwrap();
    assert_eq!(result, datetime!(2023-03-15 14:20:00 UTC));
}

#[test]
fn test_decode_server_time_millis_proto_via_builder() {
    use crate::testdata::builders::accounts::current_time_in_millis;
    use crate::testdata::builders::ResponseProtoEncoder;
    use time::macros::datetime;

    let bytes = current_time_in_millis().millis(1_678_890_000_123).encode_proto();
    let result = super::decode_server_time_millis_proto(&bytes).unwrap();
    assert_eq!(result, datetime!(2023-03-15 14:20:00.123 UTC));
}

#[test]
fn test_decode_account_update_time_proto() {
    use prost::Message;

    let proto_msg = crate::proto::AccountUpdateTime {
        time_stamp: Some("12:34:56".into()),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_account_update_time_proto(&bytes).unwrap();
    assert_eq!(result.timestamp, "12:34:56");
}

#[test]
fn test_decode_position_proto() {
    use prost::Message;

    let proto_msg = crate::proto::Position {
        account: Some("DU1234".into()),
        contract: Some(crate::proto::Contract {
            con_id: Some(265598),
            symbol: Some("AAPL".into()),
            sec_type: Some("STK".into()),
            exchange: Some("SMART".into()),
            currency: Some("USD".into()),
            ..Default::default()
        }),
        position: Some("100".into()),
        avg_cost: Some(150.25),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_position_proto(&bytes).unwrap();
    assert_eq!(result.account, "DU1234");
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.position, 100.0);
    assert_eq!(result.average_cost, 150.25);
}

#[test]
fn test_decode_position_proto_round_trips_via_builder() {
    use crate::testdata::builders::positions::position;
    use crate::testdata::builders::ResponseProtoEncoder;

    let bytes = position()
        .account("DU1234")
        .contract_id(265598)
        .symbol("AAPL")
        .exchange("SMART")
        .position(100.0)
        .average_cost(150.25)
        .encode_proto();

    let result = super::decode_position_proto(&bytes).unwrap();

    assert_eq!(result.account, "DU1234");
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.contract.exchange.to_string(), "SMART");
    assert_eq!(result.position, 100.0);
    assert_eq!(result.average_cost, 150.25);
}

#[test]
fn test_decode_pnl_proto() {
    use prost::Message;

    let proto_msg = crate::proto::PnL {
        req_id: Some(1),
        daily_pn_l: Some(1234.56),
        unrealized_pn_l: Some(500.0),
        realized_pn_l: Some(f64::MAX),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_pnl_proto(&bytes).unwrap();
    assert_eq!(result.daily_pnl, 1234.56);
    assert_eq!(result.unrealized_pnl, Some(500.0));
    assert_eq!(result.realized_pnl, None); // f64::MAX filtered out
}

#[test]
fn test_decode_account_value_proto() {
    use prost::Message;

    let proto_msg = crate::proto::AccountValue {
        key: Some("NetLiquidation".into()),
        value: Some("100000".into()),
        currency: Some("USD".into()),
        account_name: Some("DU1234".into()),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_account_value_proto(&bytes).unwrap();
    assert_eq!(result.key, "NetLiquidation");
    assert_eq!(result.value, "100000");
    assert_eq!(result.currency, "USD");
    assert_eq!(result.account, Some("DU1234".into()));
}

#[test]
fn test_decode_account_portfolio_value_proto() {
    use prost::Message;

    let proto_msg = crate::proto::PortfolioValue {
        contract: Some(crate::proto::Contract {
            con_id: Some(265598),
            symbol: Some("AAPL".into()),
            ..Default::default()
        }),
        position: Some("100".into()),
        market_price: Some(150.0),
        market_value: Some(15000.0),
        average_cost: Some(145.0),
        unrealized_pnl: Some(500.0),
        realized_pnl: Some(0.0),
        account_name: Some("DU1234".into()),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_account_portfolio_value_proto(&bytes).unwrap();
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.position, 100.0);
    assert_eq!(result.market_price, 150.0);
    assert_eq!(result.account, Some("DU1234".into()));
}

#[test]
fn test_decode_pnl_single_proto() {
    use prost::Message;

    let proto_msg = crate::proto::PnLSingle {
        req_id: Some(1),
        position: Some("500".into()),
        daily_pn_l: Some(1000.0),
        unrealized_pn_l: Some(2000.0),
        realized_pn_l: Some(500.0),
        value: Some(75000.0),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_pnl_single_proto(&bytes).unwrap();
    assert_eq!(result.position, 500.0);
    assert_eq!(result.daily_pnl, 1000.0);
    assert_eq!(result.unrealized_pnl, 2000.0);
    assert_eq!(result.realized_pnl, 500.0);
    assert_eq!(result.value, 75000.0);
}

#[test]
fn test_decode_account_summary_proto() {
    use prost::Message;

    let proto_msg = crate::proto::AccountSummary {
        req_id: Some(1),
        account: Some("DU1234".into()),
        tag: Some("NetLiquidation".into()),
        value: Some("100000".into()),
        currency: Some("USD".into()),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_account_summary_proto(&bytes).unwrap();
    assert_eq!(result.account, "DU1234");
    assert_eq!(result.tag, "NetLiquidation");
    assert_eq!(result.value, "100000");
    assert_eq!(result.currency, "USD");
}

#[test]
fn test_decode_position_multi_proto() {
    use prost::Message;

    let proto_msg = crate::proto::PositionMulti {
        req_id: Some(1),
        account: Some("DU1234".into()),
        contract: Some(crate::proto::Contract {
            con_id: Some(265598),
            symbol: Some("AAPL".into()),
            ..Default::default()
        }),
        position: Some("50".into()),
        avg_cost: Some(148.5),
        model_code: Some("Tech".into()),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_position_multi_proto(&bytes).unwrap();
    assert_eq!(result.account, "DU1234");
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.position, 50.0);
    assert_eq!(result.average_cost, 148.5);
    assert_eq!(result.model_code, "Tech");
}

#[test]
fn test_decode_account_multi_value_proto() {
    use prost::Message;

    let proto_msg = crate::proto::AccountUpdateMulti {
        req_id: Some(1),
        account: Some("DU1234".into()),
        model_code: Some("Tech".into()),
        key: Some("NetLiquidation".into()),
        value: Some("100000".into()),
        currency: Some("USD".into()),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_account_multi_value_proto(&bytes).unwrap();
    assert_eq!(result.account, "DU1234");
    assert_eq!(result.model_code, "Tech");
    assert_eq!(result.key, "NetLiquidation");
    assert_eq!(result.value, "100000");
    assert_eq!(result.currency, "USD");
}

// === Builder → production proto decoder integration tests ===
//
// Each test confirms that bytes produced by a typed builder decode through the
// real production proto decoder into a domain object whose fields match the
// builder setters. This is the "useful in the context of bytes captured and
// verified" pattern — anything the builder gets wrong about wire layout fails
// here, not in a self-loop.

#[test]
fn test_decode_position_multi_proto_via_builder() {
    use crate::testdata::builders::positions::position_multi;
    use crate::testdata::builders::ResponseProtoEncoder;

    let bytes = position_multi()
        .request_id(42)
        .account("DU8")
        .contract_id(265598)
        .symbol("AAPL")
        .position(50.0)
        .average_cost(148.5)
        .model_code("Tech")
        .encode_proto();

    let result = super::decode_position_multi_proto(&bytes).unwrap();

    assert_eq!(result.account, "DU8");
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.position, 50.0);
    assert_eq!(result.average_cost, 148.5);
    assert_eq!(result.model_code, "Tech");
}

#[test]
fn test_decode_account_value_proto_via_builder() {
    use crate::testdata::builders::accounts::account_value;
    use crate::testdata::builders::ResponseProtoEncoder;

    let bytes = account_value()
        .key("NetLiquidation")
        .value("100000")
        .currency("USD")
        .account("DU1")
        .encode_proto();

    let result = super::decode_account_value_proto(&bytes).unwrap();

    assert_eq!(result.key, "NetLiquidation");
    assert_eq!(result.value, "100000");
    assert_eq!(result.currency, "USD");
    assert_eq!(result.account, Some("DU1".into()));
}

#[test]
fn test_decode_account_summary_proto_via_builder() {
    use crate::testdata::builders::accounts::account_summary;
    use crate::testdata::builders::ResponseProtoEncoder;

    let bytes = account_summary()
        .request_id(7)
        .account("DU1234")
        .tag("NetLiquidation")
        .value("99500")
        .currency("USD")
        .encode_proto();

    let result = super::decode_account_summary_proto(&bytes).unwrap();

    assert_eq!(result.account, "DU1234");
    assert_eq!(result.tag, "NetLiquidation");
    assert_eq!(result.value, "99500");
    assert_eq!(result.currency, "USD");
}

#[test]
fn test_decode_account_multi_value_proto_via_builder() {
    use crate::testdata::builders::accounts::account_update_multi;
    use crate::testdata::builders::ResponseProtoEncoder;

    let bytes = account_update_multi()
        .account("DU1")
        .model_code("Tech")
        .key("NetLiquidation")
        .value("100000")
        .currency("USD")
        .encode_proto();

    let result = super::decode_account_multi_value_proto(&bytes).unwrap();

    assert_eq!(result.account, "DU1");
    assert_eq!(result.model_code, "Tech");
    assert_eq!(result.key, "NetLiquidation");
    assert_eq!(result.value, "100000");
    assert_eq!(result.currency, "USD");
}

#[test]
fn test_decode_pnl_proto_via_builder() {
    use crate::testdata::builders::accounts::pnl;
    use crate::testdata::builders::ResponseProtoEncoder;

    let bytes = pnl()
        .daily_pnl(1234.56)
        .unrealized_pnl(Some(500.0))
        .realized_pnl(Some(250.0))
        .encode_proto();

    let result = super::decode_pnl_proto(&bytes).unwrap();

    assert_eq!(result.daily_pnl, 1234.56);
    assert_eq!(result.unrealized_pnl, Some(500.0));
    assert_eq!(result.realized_pnl, Some(250.0));
}

#[test]
fn test_decode_pnl_single_proto_via_builder() {
    use crate::testdata::builders::accounts::pnl_single;
    use crate::testdata::builders::ResponseProtoEncoder;

    let bytes = pnl_single()
        .position(500.0)
        .daily_pnl(1000.0)
        .unrealized_pnl(2000.0)
        .realized_pnl(500.0)
        .value(75000.0)
        .encode_proto();

    let result = super::decode_pnl_single_proto(&bytes).unwrap();

    assert_eq!(result.position, 500.0);
    assert_eq!(result.daily_pnl, 1000.0);
    assert_eq!(result.unrealized_pnl, 2000.0);
    assert_eq!(result.realized_pnl, 500.0);
    assert_eq!(result.value, 75000.0);
}

#[test]
fn test_decode_managed_accounts_proto() {
    use prost::Message;
    let proto_msg = crate::proto::ManagedAccounts {
        accounts_list: Some("DU1111,DU2222,DU3333".into()),
    };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_managed_accounts_proto(&bytes).unwrap();
    assert_eq!(result, vec!["DU1111", "DU2222", "DU3333"]);
}

#[test]
fn test_decode_managed_accounts_proto_skips_empty() {
    use prost::Message;
    // Trailing comma is the wire shape TWS sometimes emits; empty fields filtered out.
    let proto_msg = crate::proto::ManagedAccounts {
        accounts_list: Some("DU1111,,DU2222,".into()),
    };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_managed_accounts_proto(&bytes).unwrap();
    assert_eq!(result, vec!["DU1111", "DU2222"]);
}

#[test]
fn test_decode_managed_accounts_proto_empty_list() {
    use prost::Message;
    let proto_msg = crate::proto::ManagedAccounts { accounts_list: None };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_managed_accounts_proto(&bytes).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_decode_family_codes_proto() {
    use prost::Message;
    let proto_msg = crate::proto::FamilyCodes {
        family_codes: vec![
            crate::proto::FamilyCode {
                account_id: Some("DU1111".into()),
                family_code: Some("FAM_A".into()),
            },
            crate::proto::FamilyCode {
                account_id: Some("DU2222".into()),
                family_code: Some("FAM_B".into()),
            },
        ],
    };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_family_codes_proto(&bytes).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].account_id, "DU1111");
    assert_eq!(result[0].family_code, "FAM_A");
    assert_eq!(result[1].account_id, "DU2222");
    assert_eq!(result[1].family_code, "FAM_B");
}

#[test]
fn test_decode_family_codes_proto_empty() {
    use prost::Message;
    let proto_msg = crate::proto::FamilyCodes { family_codes: vec![] };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = super::decode_family_codes_proto(&bytes).unwrap();
    assert!(result.is_empty());
}
