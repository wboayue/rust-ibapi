use crate::{accounts::AccountSummaryTags, server_versions, testdata::responses};

#[test]
fn test_decode_positions() {
    let mut message = super::ResponseMessage::from("61\03\0DU1234567\076792991\0TSLA\0STK\0\00.0\0\0\0NASDAQ\0USD\0TSLA\0NMS\0500\0196.77\0");

    let position = super::decode_position(&mut message).expect("error decoding position");

    assert_eq!(position.account, "DU1234567", "position.account");
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
}

#[test]
fn test_decode_position_v1_message() {
    // Assemble: version 1, no trading_class, no average_cost
    // Message format: type, version, account, conId, symbol, secType, lastTradeDateOrContractMonth, strike, right, multiplier, exchange, currency, localSymbol, position
    let mut message =
        super::ResponseMessage::from("61\x001\x00DU123\x00123\x00SYM\x00STK\x00251212\x000.0\x00P\x00MULT\x00EXCH\x00USD\x00LOCSYM\x00100.0\x00");

    // Act
    let result = super::decode_position(&mut message).expect("Failed to decode position");

    // Assert
    assert_eq!(result.account, "DU123", "account");
    assert_eq!(result.contract.contract_id, 123, "contract.contract_id");
    assert_eq!(result.contract.symbol, "SYM", "contract.symbol");
    assert_eq!(
        result.contract.security_type,
        super::super::super::contracts::SecurityType::Stock,
        "contract.security_type"
    );
    assert_eq!(
        result.contract.last_trade_date_or_contract_month, "251212",
        "contract.last_trade_date_or_contract_month"
    );
    assert_eq!(result.contract.strike, 0.0, "contract.strike");
    assert_eq!(result.contract.right, "P", "contract.right");
    assert_eq!(result.contract.multiplier, "MULT", "contract.multiplier");
    assert_eq!(result.contract.exchange, "EXCH", "contract.exchange");
    assert_eq!(result.contract.currency, "USD", "contract.currency");
    assert_eq!(result.contract.local_symbol, "LOCSYM", "contract.local_symbol");
    assert_eq!(result.position, 100.0, "position");
    assert_eq!(result.contract.trading_class, "", "contract.trading_class should be empty for v1");
    assert_eq!(result.average_cost, 0.0, "average_cost should be 0.0 for v1");
}

#[test]
fn test_decode_position_v2_message() {
    // Assemble: version 2, has trading_class, no average_cost
    // Message format: type, version, account, conId, symbol, secType, lastTradeDateOrContractMonth, strike, right, multiplier, exchange, currency, localSymbol, trading_class, position
    let mut message = super::ResponseMessage::from(
        "61\x002\x00DU123\x00123\x00SYM\x00STK\x00251212\x000.0\x00P\x00MULT\x00EXCH\x00USD\x00LOCSYM\x00TRDCLS\x00100.0\x00",
    );

    // Act
    let result = super::decode_position(&mut message).expect("Failed to decode position");

    // Assert
    assert_eq!(result.account, "DU123", "account");
    assert_eq!(result.contract.contract_id, 123, "contract.contract_id");
    assert_eq!(result.contract.symbol, "SYM", "contract.symbol");
    assert_eq!(
        result.contract.security_type,
        super::super::super::contracts::SecurityType::Stock,
        "contract.security_type"
    );
    assert_eq!(
        result.contract.last_trade_date_or_contract_month, "251212",
        "contract.last_trade_date_or_contract_month"
    );
    assert_eq!(result.contract.strike, 0.0, "contract.strike");
    assert_eq!(result.contract.right, "P", "contract.right");
    assert_eq!(result.contract.multiplier, "MULT", "contract.multiplier");
    assert_eq!(result.contract.exchange, "EXCH", "contract.exchange");
    assert_eq!(result.contract.currency, "USD", "contract.currency");
    assert_eq!(result.contract.local_symbol, "LOCSYM", "contract.local_symbol");
    assert_eq!(result.contract.trading_class, "TRDCLS", "contract.trading_class");
    assert_eq!(result.position, 100.0, "position");
    assert_eq!(result.average_cost, 0.0, "average_cost should be 0.0 for v2");
}

#[test]
fn test_decode_position_multi() {
    let mut message = super::ResponseMessage::from("61\03\06\0DU1234567\076792991\0TSLA\0STK\0\00.0\0\0\0NASDAQ\0USD\0TSLA\0NMS\0500\0196.77\0\0");

    let position = super::decode_position_multi(&mut message).expect("error decoding position multi");

    assert_eq!(position.account, "DU1234567", "position.account");
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
    assert_eq!(position.model_code, "", "position.model_code");
}

#[test]
fn test_decode_family_codes() {
    let mut message = super::ResponseMessage::from("78\01\0*\0\0");

    let family_codes = super::decode_family_codes(&mut message).expect("error decoding family codes");

    assert_eq!(family_codes[0].account_id, "*", "family_codes.account_id");
    assert_eq!(family_codes[0].family_code, "", "family_codes.family_code");
}

#[test]
fn test_decode_family_codes_empty_list() {
    // Assemble: version, 0 codes
    let mut message = super::ResponseMessage::from("78\x000\x00");

    // Act
    let result = super::decode_family_codes(&mut message).expect("Failed to decode family codes");

    // Assert
    assert!(result.is_empty(), "Result should be an empty list");
}

#[test]
fn test_decode_family_codes_multiple_codes() {
    // Assemble: version, 2 codes
    let mut message = super::ResponseMessage::from("78\x002\x00ACC1\x00FC1\x00ACC2\x00FC2\x00");

    // Act
    let result = super::decode_family_codes(&mut message).expect("Failed to decode family codes");

    // Assert
    assert_eq!(result.len(), 2, "Should have 2 family codes");
    assert_eq!(result[0].account_id, "ACC1", "First account_id");
    assert_eq!(result[0].family_code, "FC1", "First family_code");
    assert_eq!(result[1].account_id, "ACC2", "Second account_id");
    assert_eq!(result[1].family_code, "FC2", "Second family_code");
}

#[test]
fn test_decode_pnl() {
    let mut message = super::ResponseMessage::from("94\09000\00.1\00.2\00.3\0");

    let pnl = super::decode_pnl(server_versions::REALIZED_PNL, &mut message).expect("error decoding pnl");

    assert_eq!(pnl.daily_pnl, 0.10, "pnl.daily_pnl");
    assert_eq!(pnl.unrealized_pnl, Some(0.20), "pnl.unrealized_pnl");
    assert_eq!(pnl.realized_pnl, Some(0.30), "pnl.realized_pnl");

    let mut message = super::ResponseMessage::from("94\09000\00.1\00.2\00.3\0");

    let pnl = super::decode_pnl(server_versions::UNREALIZED_PNL, &mut message).expect("error decoding pnl");

    assert_eq!(pnl.daily_pnl, 0.10, "pnl.daily_pnl");
    assert_eq!(pnl.unrealized_pnl, Some(0.20), "pnl.unrealized_pnl");
    assert_eq!(pnl.realized_pnl, None, "pnl.realized_pnl");

    let mut message = super::ResponseMessage::from("94\09000\00.1\00.2\00.3\0");

    let pnl = super::decode_pnl(server_versions::PNL, &mut message).expect("error decoding pnl");

    assert_eq!(pnl.daily_pnl, 0.10, "pnl.daily_pnl");
    assert_eq!(pnl.unrealized_pnl, None, "pnl.unrealized_pnl");
    assert_eq!(pnl.realized_pnl, None, "pnl.realized_pnl");
}

#[test]
fn test_decode_pnl_single() {
    let mut message = super::ResponseMessage::from("95\09000\0100.0\00.1\00.2\00.3\00.4\0");

    let pnl_single = super::decode_pnl_single(server_versions::REALIZED_PNL, &mut message).expect("error decoding pnl single");

    assert_eq!(pnl_single.position, 100., "pnl_single.position");
    assert_eq!(pnl_single.daily_pnl, 0.10, "pnl_single.daily_pnl");
    assert_eq!(pnl_single.unrealized_pnl, 0.20, "pnl_single.unrealized_pnl");
    assert_eq!(pnl_single.realized_pnl, 0.30, "pnl_single.realized_pnl");
    assert_eq!(pnl_single.value, 0.40, "pnl_single.value");
}

#[test]
fn test_decode_account_summary() {
    let mut message = super::ResponseMessage::from("94\01\09000\0DU1234567\0AccountType\0FA\0\0");

    let account_summary = super::decode_account_summary(server_versions::REALIZED_PNL, &mut message).expect("error decoding pnl");

    assert_eq!(account_summary.account, "DU1234567", "account_summary.account");
    assert_eq!(account_summary.tag, AccountSummaryTags::ACCOUNT_TYPE, "account_summary.tag");
    assert_eq!(account_summary.value, "FA", "account_summary.value");
    assert_eq!(account_summary.currency, "", "account_summary.currency");
}

#[test]
fn test_decode_account_multi_value() {
    let mut message = super::ResponseMessage::from_simple(responses::ACCOUNT_UPDATE_MULTI_CURRENCY);

    let value = super::decode_account_multi_value(&mut message).expect("error decoding account multi value");

    assert_eq!(value.account, "DU1234567", "value.account");
    assert_eq!(value.model_code, "", "value.model_code");
    assert_eq!(value.key, "Currency", "value.key");
    assert_eq!(value.value, "USD", "value.value");
    assert_eq!(value.currency, "USD", "value.currency");
}

#[test]
fn test_decode_account_portfolio_value_version_matrix() {
    struct TestCase {
        name: &'static str,
        message_version: i32,
        server_version: i32,
        message_string: String,
        expected_contract_id: i32,
        // Standard contract fields for verification
        expected_symbol: &'static str,
        expected_sec_type: super::super::super::contracts::SecurityType,
        expected_expiry: &'static str,
        expected_strike: f64,
        expected_right: &'static str,
        expected_currency: &'static str,
        // Version-dependent contract fields
        expected_multiplier: &'static str,
        expected_primary_exchange: &'static str,
        expected_local_symbol: &'static str,
        expected_trading_class: &'static str,
        // Portfolio fields
        expected_position: f64,
        expected_market_price: f64,
        expected_market_value: f64,
        expected_average_cost: Option<f64>,
        expected_unrealized_pnl: Option<f64>,
        expected_realized_pnl: Option<f64>,
        expected_account_name: Option<&'static str>,
    }

    // Helper to construct message string based on version and fields
    fn construct_portfolio_message(
        msg_ver: i32,
        sv_ver: i32,
        con_id: &str,
        sym: &str,
        sec_t: &str,
        exp: &str,
        strike: &str,
        right: &str,
        mult: &str,
        prim_exch: &str,
        curr: &str,
        local_sym: &str,
        trading_class: &str,
        pos: &str,
        m_price: &str,
        m_val: &str,
        avg_c: &str,
        un_pnl: &str,
        r_pnl: &str,
        acc_name: &str,
        prim_exch_override_for_sv39: Option<&str>,
    ) -> String {
        let msg_ver_str = msg_ver.to_string();
        let mut fields = vec!["9", &msg_ver_str]; // type, version

        if msg_ver >= 6 {
            fields.push(con_id);
        }
        fields.push(sym);
        fields.push(sec_t);
        fields.push(exp);
        fields.push(strike);
        fields.push(right);
        if msg_ver >= 7 {
            fields.push(mult);
        }
        if msg_ver >= 7 {
            fields.push(prim_exch);
        }
        fields.push(curr);
        if msg_ver >= 2 {
            fields.push(local_sym);
        }
        if msg_ver >= 8 {
            fields.push(trading_class);
        }
        fields.push(pos);
        fields.push(m_price);
        fields.push(m_val);
        if msg_ver >= 3 {
            fields.push(avg_c);
        }
        if msg_ver >= 3 {
            fields.push(un_pnl);
        }
        if msg_ver >= 3 {
            fields.push(r_pnl);
        }
        if msg_ver >= 4 {
            fields.push(acc_name);
        }
        if msg_ver >= 6 && sv_ver == 39 {
            if let Some(pe_override) = prim_exch_override_for_sv39 {
                fields.push(pe_override);
            }
        }
        fields.join("\x00") + "\x00"
    }

    let tests = [
        TestCase {
            name: "mv1_sv_any_no_localsymbol_no_tradingclass",
            message_version: 1,
            server_version: server_versions::SIZE_RULES,
            message_string: construct_portfolio_message(
                1,
                server_versions::SIZE_RULES,
                "",
                "SYM",
                "STK",
                "251212",
                "0.0",
                "P",
                "",
                "",
                "USD",
                "",
                "",
                "100.0",
                "10.0",
                "1000.0",
                "",
                "",
                "",
                "",
                None,
            ),
            expected_contract_id: 0,
            expected_symbol: "SYM",
            expected_sec_type: super::super::super::contracts::SecurityType::Stock,
            expected_expiry: "251212",
            expected_strike: 0.0,
            expected_right: "P",
            expected_currency: "USD",
            expected_multiplier: "",
            expected_primary_exchange: "",
            expected_local_symbol: "",
            expected_trading_class: "",
            expected_position: 100.0,
            expected_market_price: 10.0,
            expected_market_value: 1000.0,
            expected_average_cost: None,
            expected_unrealized_pnl: None,
            expected_realized_pnl: None,
            expected_account_name: None,
        },
        TestCase {
            name: "mv2_sv_any_has_local_symbol",
            message_version: 2,
            server_version: server_versions::SIZE_RULES,
            message_string: construct_portfolio_message(
                2,
                server_versions::SIZE_RULES,
                "",
                "SYM",
                "STK",
                "251212",
                "0.0",
                "P",
                "",
                "",
                "USD",
                "LOCSYM",
                "",
                "100.0",
                "10.0",
                "1000.0",
                "",
                "",
                "",
                "",
                None,
            ),
            expected_contract_id: 0,
            expected_symbol: "SYM",
            expected_sec_type: super::super::super::contracts::SecurityType::Stock,
            expected_expiry: "251212",
            expected_strike: 0.0,
            expected_right: "P",
            expected_currency: "USD",
            expected_multiplier: "",
            expected_primary_exchange: "",
            expected_local_symbol: "LOCSYM",
            expected_trading_class: "",
            expected_position: 100.0,
            expected_market_price: 10.0,
            expected_market_value: 1000.0,
            expected_average_cost: None,
            expected_unrealized_pnl: None,
            expected_realized_pnl: None,
            expected_account_name: None,
        },
        TestCase {
            name: "mv5_sv_any_has_avgcost_pnl_accname",
            message_version: 5,
            server_version: server_versions::SIZE_RULES,
            message_string: construct_portfolio_message(
                5,
                server_versions::SIZE_RULES,
                "",
                "SYM",
                "STK",
                "251212",
                "0.0",
                "P",
                "",
                "",
                "USD",
                "LOCSYM",
                "",
                "100.0",
                "10.0",
                "1000.0",
                "9.0",
                "100.0",
                "50.0",
                "ACC1",
                None,
            ),
            expected_contract_id: 0,
            expected_symbol: "SYM",
            expected_sec_type: super::super::super::contracts::SecurityType::Stock,
            expected_expiry: "251212",
            expected_strike: 0.0,
            expected_right: "P",
            expected_currency: "USD",
            expected_multiplier: "",
            expected_primary_exchange: "",
            expected_local_symbol: "LOCSYM",
            expected_trading_class: "",
            expected_position: 100.0,
            expected_market_price: 10.0,
            expected_market_value: 1000.0,
            expected_average_cost: Some(9.0),
            expected_unrealized_pnl: Some(100.0),
            expected_realized_pnl: Some(50.0),
            expected_account_name: Some("ACC1"),
        },
        TestCase {
            name: "mv6_sv_not39_has_conid",
            message_version: 6,
            server_version: server_versions::SIZE_RULES,
            message_string: construct_portfolio_message(
                6,
                server_versions::SIZE_RULES,
                "123",
                "SYM",
                "STK",
                "251212",
                "0.0",
                "P",
                "",
                "",
                "USD",
                "LOCSYM",
                "",
                "100.0",
                "10.0",
                "1000.0",
                "9.0",
                "100.0",
                "50.0",
                "ACC1",
                None,
            ),
            expected_contract_id: 123,
            expected_symbol: "SYM",
            expected_sec_type: super::super::super::contracts::SecurityType::Stock,
            expected_expiry: "251212",
            expected_strike: 0.0,
            expected_right: "P",
            expected_currency: "USD",
            expected_multiplier: "",
            expected_primary_exchange: "",
            expected_local_symbol: "LOCSYM",
            expected_trading_class: "",
            expected_position: 100.0,
            expected_market_price: 10.0,
            expected_market_value: 1000.0,
            expected_average_cost: Some(9.0),
            expected_unrealized_pnl: Some(100.0),
            expected_realized_pnl: Some(50.0),
            expected_account_name: Some("ACC1"),
        },
        TestCase {
            name: "mv6_sv39_has_conid_prim_exch_override",
            message_version: 6,
            server_version: 39,
            message_string: construct_portfolio_message(
                6,
                39,
                "123",
                "SYM",
                "STK",
                "251212",
                "0.0",
                "P",
                "",
                "",
                "USD",
                "LOCSYM",
                "",
                "100.0",
                "10.0",
                "1000.0",
                "9.0",
                "100.0",
                "50.0",
                "ACC1",
                Some("OVERRIDE_EXCH"),
            ),
            expected_contract_id: 123,
            expected_symbol: "SYM",
            expected_sec_type: super::super::super::contracts::SecurityType::Stock,
            expected_expiry: "251212",
            expected_strike: 0.0,
            expected_right: "P",
            expected_currency: "USD",
            expected_multiplier: "",
            expected_primary_exchange: "OVERRIDE_EXCH",
            expected_local_symbol: "LOCSYM",
            expected_trading_class: "",
            expected_position: 100.0,
            expected_market_price: 10.0,
            expected_market_value: 1000.0,
            expected_average_cost: Some(9.0),
            expected_unrealized_pnl: Some(100.0),
            expected_realized_pnl: Some(50.0),
            expected_account_name: Some("ACC1"),
        },
        TestCase {
            name: "mv7_sv_any_has_mult_prim_exch",
            message_version: 7,
            server_version: server_versions::SIZE_RULES,
            message_string: construct_portfolio_message(
                7,
                server_versions::SIZE_RULES,
                "123",
                "SYM",
                "STK",
                "251212",
                "0.0",
                "P",
                "MULT1",
                "PRIMEXCH1",
                "USD",
                "LOCSYM",
                "",
                "100.0",
                "10.0",
                "1000.0",
                "9.0",
                "100.0",
                "50.0",
                "ACC1",
                None,
            ),
            expected_contract_id: 123,
            expected_symbol: "SYM",
            expected_sec_type: super::super::super::contracts::SecurityType::Stock,
            expected_expiry: "251212",
            expected_strike: 0.0,
            expected_right: "P",
            expected_currency: "USD",
            expected_multiplier: "MULT1",
            expected_primary_exchange: "PRIMEXCH1",
            expected_local_symbol: "LOCSYM",
            expected_trading_class: "",
            expected_position: 100.0,
            expected_market_price: 10.0,
            expected_market_value: 1000.0,
            expected_average_cost: Some(9.0),
            expected_unrealized_pnl: Some(100.0),
            expected_realized_pnl: Some(50.0),
            expected_account_name: Some("ACC1"),
        },
        TestCase {
            name: "mv7_sv39_has_mult_prim_exch_with_override_field",
            message_version: 7,
            server_version: 39,
            message_string: construct_portfolio_message(
                7,
                39,
                "123",
                "SYM",
                "STK",
                "251212",
                "0.0",
                "P",
                "MULT1",
                "PRIMEXCH1",
                "USD",
                "LOCSYM",
                "",
                "100.0",
                "10.0",
                "1000.0",
                "9.0",
                "100.0",
                "50.0",
                "ACC1",
                Some("OVERRIDE_EXCH_V7_SV39"),
            ),
            expected_contract_id: 123,
            expected_symbol: "SYM",
            expected_sec_type: super::super::super::contracts::SecurityType::Stock,
            expected_expiry: "251212",
            expected_strike: 0.0,
            expected_right: "P",
            expected_currency: "USD",
            expected_multiplier: "MULT1",
            expected_primary_exchange: "PRIMEXCH1",
            expected_local_symbol: "LOCSYM",
            expected_trading_class: "",
            expected_position: 100.0,
            expected_market_price: 10.0,
            expected_market_value: 1000.0,
            expected_average_cost: Some(9.0),
            expected_unrealized_pnl: Some(100.0),
            expected_realized_pnl: Some(50.0),
            expected_account_name: Some("ACC1"),
        },
        TestCase {
            name: "mv8_sv_any_has_trading_class",
            message_version: 8,
            server_version: server_versions::SIZE_RULES,
            message_string: construct_portfolio_message(
                8,
                server_versions::SIZE_RULES,
                "123",
                "SYM",
                "STK",
                "251212",
                "0.0",
                "P",
                "MULT1",
                "PRIMEXCH1",
                "USD",
                "LOCSYM",
                "TRDCLS1",
                "100.0",
                "10.0",
                "1000.0",
                "9.0",
                "100.0",
                "50.0",
                "ACC1",
                None,
            ),
            expected_contract_id: 123,
            expected_symbol: "SYM",
            expected_sec_type: super::super::super::contracts::SecurityType::Stock,
            expected_expiry: "251212",
            expected_strike: 0.0,
            expected_right: "P",
            expected_currency: "USD",
            expected_multiplier: "MULT1",
            expected_primary_exchange: "PRIMEXCH1",
            expected_local_symbol: "LOCSYM",
            expected_trading_class: "TRDCLS1",
            expected_position: 100.0,
            expected_market_price: 10.0,
            expected_market_value: 1000.0,
            expected_average_cost: Some(9.0),
            expected_unrealized_pnl: Some(100.0),
            expected_realized_pnl: Some(50.0),
            expected_account_name: Some("ACC1"),
        },
        TestCase {
            name: "mv8_sv39_has_trading_class_with_override_field",
            message_version: 8,
            server_version: 39,
            message_string: construct_portfolio_message(
                8,
                39,
                "123",
                "SYM",
                "STK",
                "251212",
                "0.0",
                "P",
                "MULT1",
                "PRIMEXCH1",
                "USD",
                "LOCSYM",
                "TRDCLS1",
                "100.0",
                "10.0",
                "1000.0",
                "9.0",
                "100.0",
                "50.0",
                "ACC1",
                Some("OVERRIDE_EXCH_V8_SV39"),
            ),
            expected_contract_id: 123,
            expected_symbol: "SYM",
            expected_sec_type: super::super::super::contracts::SecurityType::Stock,
            expected_expiry: "251212",
            expected_strike: 0.0,
            expected_right: "P",
            expected_currency: "USD",
            expected_multiplier: "MULT1",
            expected_primary_exchange: "PRIMEXCH1",
            expected_local_symbol: "LOCSYM",
            expected_trading_class: "TRDCLS1",
            expected_position: 100.0,
            expected_market_price: 10.0,
            expected_market_value: 1000.0,
            expected_average_cost: Some(9.0),
            expected_unrealized_pnl: Some(100.0),
            expected_realized_pnl: Some(50.0),
            expected_account_name: Some("ACC1"),
        },
    ];

    for tc in tests.iter() {
        let mut message = super::ResponseMessage::from(tc.message_string.as_str());
        let result = super::decode_account_portfolio_value(tc.server_version, &mut message)
            .unwrap_or_else(|e| panic!("Test case '{}' failed decoding: {:?}", tc.name, e));

        assert_eq!(result.contract.contract_id, tc.expected_contract_id, "Case: {} - contract_id", tc.name);
        assert_eq!(result.contract.symbol, tc.expected_symbol, "Case: {} - symbol", tc.name);
        assert_eq!(result.contract.security_type, tc.expected_sec_type, "Case: {} - sec_type", tc.name);
        assert_eq!(
            result.contract.last_trade_date_or_contract_month, tc.expected_expiry,
            "Case: {} - expiry",
            tc.name
        );
        assert_eq!(result.contract.strike, tc.expected_strike, "Case: {} - strike", tc.name);
        assert_eq!(result.contract.right, tc.expected_right, "Case: {} - right", tc.name);
        assert_eq!(result.contract.multiplier, tc.expected_multiplier, "Case: {} - multiplier", tc.name);

        if tc.message_version >= 6 && tc.server_version == 39 {
            assert_eq!(
                result.contract.primary_exchange, tc.expected_primary_exchange,
                "Case: {} - primary_exchange (sv39 override)",
                tc.name
            );
        } else {
            assert_eq!(
                result.contract.primary_exchange, tc.expected_primary_exchange,
                "Case: {} - primary_exchange",
                tc.name
            );
        }
        assert_eq!(result.contract.currency, tc.expected_currency, "Case: {} - currency", tc.name);
        assert_eq!(result.contract.local_symbol, tc.expected_local_symbol, "Case: {} - local_symbol", tc.name);
        assert_eq!(
            result.contract.trading_class, tc.expected_trading_class,
            "Case: {} - trading_class",
            tc.name
        );
        assert_eq!(result.position, tc.expected_position, "Case: {} - position", tc.name);
        assert_eq!(result.market_price, tc.expected_market_price, "Case: {} - market_price", tc.name);
        assert_eq!(result.market_value, tc.expected_market_value, "Case: {} - market_value", tc.name);
        assert_eq!(
            result.average_cost,
            tc.expected_average_cost.unwrap_or(0.0),
            "Case: {} - average_cost",
            tc.name
        );
        assert_eq!(
            result.unrealized_pnl,
            tc.expected_unrealized_pnl.unwrap_or(0.0),
            "Case: {} - unrealized_pnl",
            tc.name
        );
        assert_eq!(
            result.realized_pnl,
            tc.expected_realized_pnl.unwrap_or(0.0),
            "Case: {} - realized_pnl",
            tc.name
        );
    }
}

#[test]
fn test_decode_account_value_versions() {
    struct TestCase {
        name: &'static str,
        // message_version: i32,
        message_fields: Vec<&'static str>,
        expected_key: &'static str,
        expected_value: &'static str,
        expected_currency: &'static str,
        expected_account_name: Option<&'static str>,
    }

    let tests = [
        TestCase {
            name: "v1_no_account_name",
            message_fields: vec!["6", "1", "CashBalance", "1000.00", "USD"],
            expected_key: "CashBalance",
            expected_value: "1000.00",
            expected_currency: "USD",
            expected_account_name: None,
        },
        TestCase {
            name: "v2_with_account_name",
            message_fields: vec!["6", "2", "EquityWithLoanValue", "1200.00", "CAD", "ACC123"],
            expected_key: "EquityWithLoanValue",
            expected_value: "1200.00",
            expected_currency: "CAD",
            expected_account_name: Some("ACC123"),
        },
    ];

    for tc in tests.iter() {
        let message_string = tc.message_fields.join("\x00") + "\x00";
        let mut message = super::ResponseMessage::from(message_string.as_str());
        let result = super::decode_account_value(&mut message).unwrap_or_else(|e| panic!("Test case '{}' failed: {:?}", tc.name, e));
        assert_eq!(result.key, tc.expected_key, "Case: {} - key", tc.name);
        assert_eq!(result.value, tc.expected_value, "Case: {} - value", tc.name);
        assert_eq!(result.currency, tc.expected_currency, "Case: {} - currency", tc.name);
        assert_eq!(result.account.as_deref(), tc.expected_account_name, "Case: {} - account_name", tc.name);
    }
}

#[test]
fn test_decode_account_update_time_success() {
    // Assemble: type(8), version(1), time_stamp
    let mut message = super::ResponseMessage::from("8\x001\x0012:34:56\x00");

    // Act
    let result = super::decode_account_update_time(&mut message);

    // Assert
    assert!(result.is_ok(), "Decoding failed: {:?}", result.err());
    assert_eq!(result.unwrap().timestamp, "12:34:56", "Timestamp mismatch");
}
