use super::*;

#[test]
fn test_decode_market_rule() {
    let mut message = ResponseMessage::from_simple("93|26|1|0|0.01|");

    let market_rule = decode_market_rule(&mut message).expect("error decoding market rule");

    assert_eq!(market_rule.market_rule_id, 26, "market_rule.market_rule_id");

    assert_eq!(market_rule.price_increments.len(), 1, "market_rule.price_increments.len()");
    assert_eq!(market_rule.price_increments[0].low_edge, 0.0, "market_rule.price_increments[0].low_edge");
    assert_eq!(
        market_rule.price_increments[0].increment, 0.01,
        "market_rule.price_increments[0].increment"
    );
}

// Test for split_hours function
#[test]
fn test_split_hours() {
    let hours = "09:30:00-16:00:00;09:30:00-16:00:00";
    let result = split_hours(hours);

    assert_eq!(result.len(), 2, "Should split into 2 parts");
    assert_eq!(result[0], "09:30:00-16:00:00", "First part should match");
    assert_eq!(result[1], "09:30:00-16:00:00", "Second part should match");

    // Test with empty string
    let empty = "";
    let empty_result = split_hours(empty);
    assert_eq!(empty_result.len(), 1, "Empty string should produce one empty element");
    assert_eq!(empty_result[0], "", "Empty string result should be empty");
}

// Test for split_to_vec function
#[test]
fn test_split_to_vec() {
    let values = "SMART,ISLAND,NYSE,ARCA";
    let result = split_to_vec(values);

    assert_eq!(result.len(), 4, "Should split into 4 parts");
    assert_eq!(result[0], "SMART", "First part should be SMART");
    assert_eq!(result[1], "ISLAND", "Second part should be ISLAND");
    assert_eq!(result[2], "NYSE", "Third part should be NYSE");
    assert_eq!(result[3], "ARCA", "Fourth part should be ARCA");

    // Test with empty string
    let empty = "";
    let empty_result = split_to_vec(empty);
    assert_eq!(empty_result.len(), 1, "Empty string should produce one empty element");
    assert_eq!(empty_result[0], "", "Empty string result should be empty");
}

#[test]
fn test_decode_option_chain() {
    // Assemble
    // The message format should be: message_type, request_id, exchange, underlying_contract_id, trading_class, multiplier, expirations_count, expirations, strikes_count, strikes
    let mut message = ResponseMessage::from_simple("75|9000|NYSE|789456|GOOG|100|3|2023-06|2023-09|2023-12|3|100.5|110.0|120.5|");

    // Activate
    let option_chain = decode_option_chain(&mut message).expect("error decoding option chain");

    // Assert
    assert_eq!(option_chain.exchange, "NYSE", "option_chain.exchange");
    assert_eq!(option_chain.underlying_contract_id, 789456, "option_chain.underlying_contract_id");
    assert_eq!(option_chain.trading_class, "GOOG", "option_chain.trading_class");
    assert_eq!(option_chain.multiplier, "100", "option_chain.multiplier");
    assert_eq!(option_chain.expirations.len(), 3, "option_chain.expirations.len()");
    assert_eq!(option_chain.expirations[0], "2023-06", "option_chain.expirations[0]");
    assert_eq!(option_chain.expirations[1], "2023-09", "option_chain.expirations[1]");
    assert_eq!(option_chain.expirations[2], "2023-12", "option_chain.expirations[2]");
    assert_eq!(option_chain.strikes.len(), 3, "option_chain.strikes.len()");
    assert_eq!(option_chain.strikes[0], 100.5, "option_chain.strikes[0]");
    assert_eq!(option_chain.strikes[1], 110.0, "option_chain.strikes[1]");
    assert_eq!(option_chain.strikes[2], 120.5, "option_chain.strikes[2]");
}

#[test]
fn test_read_last_trade_date() {
    // Assemble
    let mut contract = ContractDetails::default();

    // Test with dash separator
    // Activate
    read_last_trade_date(&mut contract, "2023-06-15", false).expect("error reading last trade date");

    // Assert
    assert_eq!(
        contract.contract.last_trade_date_or_contract_month, "2023",
        "contract.last_trade_date_or_contract_month"
    );
    assert_eq!(contract.last_trade_time, "06", "contract.last_trade_time");

    // Test with space separator
    // Assemble
    let mut contract = ContractDetails::default();

    // Activate
    read_last_trade_date(&mut contract, "2023 06 15", false).expect("error reading last trade date");

    // Assert
    assert_eq!(
        contract.contract.last_trade_date_or_contract_month, "2023",
        "contract.last_trade_date_or_contract_month"
    );
    assert_eq!(contract.last_trade_time, "06", "contract.last_trade_time");

    // Test for bond
    // Assemble
    let mut contract = ContractDetails::default();

    // Activate
    read_last_trade_date(&mut contract, "2030 01 15 GMT", true).expect("error reading last trade date");

    // Assert
    assert_eq!(contract.maturity, "2030", "contract.maturity");
    assert_eq!(contract.last_trade_time, "01", "contract.last_trade_time");
    assert_eq!(contract.time_zone_id, "15", "contract.time_zone_id");

    // Test with empty string
    // Assemble
    let mut contract = ContractDetails::default();

    // Activate
    read_last_trade_date(&mut contract, "", false).expect("error reading last trade date");

    // Assert
    assert_eq!(
        contract.contract.last_trade_date_or_contract_month, "",
        "contract.last_trade_date_or_contract_month should be empty"
    );
    assert_eq!(contract.last_trade_time, "", "contract.last_trade_time should be empty");
}

#[test]
fn test_next_optional_double() {
    // Assemble
    let mut message = ResponseMessage::from_simple("1.25|2.50|-1.0|-2.0|");

    // Activate
    let result1 = next_optional_double(&mut message, -1.0).expect("error decoding optional double");
    let result2 = next_optional_double(&mut message, -1.0).expect("error decoding optional double");
    let result3 = next_optional_double(&mut message, -1.0).expect("error decoding optional double");
    let result4 = next_optional_double(&mut message, -2.0).expect("error decoding optional double");

    // Assert
    assert_eq!(result1, Some(1.25), "result1 should be Some(1.25)");
    assert_eq!(result2, Some(2.50), "result2 should be Some(2.50)");
    assert_eq!(result3, None, "result3 should be None because it matches none_value (-1.0)");
    assert_eq!(result4, None, "result4 should be None because it matches none_value (-2.0)");
}

#[test]
fn test_decode_option_computation() {
    // Assemble
    // Message format: message_type, request_id, tick_type, tick_attribute, implied_vol, delta, option_price, dividend, gamma, vega, theta, underlying_price
    let mut message = ResponseMessage::from_simple("10|123|13|1|0.25|0.45|155.25|0.75|0.05|0.15|0.10|150.0|");

    // Activate
    let computation = decode_option_computation(server_versions::PRICE_BASED_VOLATILITY, &mut message).expect("error decoding option computation");

    // Assert
    assert_eq!(computation.field, TickType::ModelOption, "computation.field");
    assert_eq!(computation.tick_attribute, Some(1), "computation.tick_attribute");
    assert_eq!(computation.implied_volatility, Some(0.25), "computation.implied_volatility");
    assert_eq!(computation.delta, Some(0.45), "computation.delta");
    assert_eq!(computation.option_price, Some(155.25), "computation.option_price");
    assert_eq!(computation.present_value_dividend, Some(0.75), "computation.present_value_dividend");
    assert_eq!(computation.gamma, Some(0.05), "computation.gamma");
    assert_eq!(computation.vega, Some(0.15), "computation.vega");
    assert_eq!(computation.theta, Some(0.10), "computation.theta");
    assert_eq!(computation.underlying_price, Some(150.0), "computation.underlying_price");
}

#[test]
fn test_decode_contract_descriptions() {
    // Assemble
    // Format: message_type, request_id, count, contract_id, symbol, security_type, primary_exchange, currency, derivative_sec_types_count, deriv_types, description, issuer_id
    let mut message = ResponseMessage::from_simple(
        "81|42|2|12345|AAPL|STK|NASDAQ|USD|2|OPT|WAR|Apple Inc.|AAPL123|67890|MSFT|STK|NASDAQ|USD|1|OPT|Microsoft Corp.|MSFT456|",
    );

    // Activate
    let descriptions = decode_contract_descriptions(server_versions::BOND_ISSUERID, &mut message).expect("error decoding contract descriptions");

    // Assert
    assert_eq!(descriptions.len(), 2, "descriptions.len()");

    // First contract
    assert_eq!(descriptions[0].contract.contract_id, 12345, "descriptions[0].contract.contract_id");
    assert_eq!(descriptions[0].contract.symbol, Symbol::from("AAPL"), "descriptions[0].contract.symbol");
    assert_eq!(
        descriptions[0].contract.security_type,
        SecurityType::Stock,
        "descriptions[0].contract.security_type"
    );
    assert_eq!(
        descriptions[0].contract.primary_exchange,
        Exchange::from("NASDAQ"),
        "descriptions[0].contract.primary_exchange"
    );
    assert_eq!(
        descriptions[0].contract.currency,
        Currency::from("USD"),
        "descriptions[0].contract.currency"
    );
    assert_eq!(
        descriptions[0].derivative_security_types.len(),
        2,
        "descriptions[0].derivative_security_types.len()"
    );
    assert_eq!(
        descriptions[0].derivative_security_types[0], "OPT",
        "descriptions[0].derivative_security_types[0]"
    );
    assert_eq!(
        descriptions[0].derivative_security_types[1], "WAR",
        "descriptions[0].derivative_security_types[1]"
    );
    assert_eq!(descriptions[0].contract.description, "Apple Inc.", "descriptions[0].contract.description");
    assert_eq!(descriptions[0].contract.issuer_id, "AAPL123", "descriptions[0].contract.issuer_id");

    // Second contract
    assert_eq!(descriptions[1].contract.contract_id, 67890, "descriptions[1].contract.contract_id");
    assert_eq!(descriptions[1].contract.symbol, Symbol::from("MSFT"), "descriptions[1].contract.symbol");
    assert_eq!(
        descriptions[1].contract.security_type,
        SecurityType::Stock,
        "descriptions[1].contract.security_type"
    );
    assert_eq!(
        descriptions[1].contract.primary_exchange,
        Exchange::from("NASDAQ"),
        "descriptions[1].contract.primary_exchange"
    );
    assert_eq!(
        descriptions[1].contract.currency,
        Currency::from("USD"),
        "descriptions[1].contract.currency"
    );
    assert_eq!(
        descriptions[1].derivative_security_types.len(),
        1,
        "descriptions[1].derivative_security_types.len()"
    );
    assert_eq!(
        descriptions[1].derivative_security_types[0], "OPT",
        "descriptions[1].derivative_security_types[0]"
    );
    assert_eq!(
        descriptions[1].contract.description, "Microsoft Corp.",
        "descriptions[1].contract.description"
    );
    assert_eq!(descriptions[1].contract.issuer_id, "MSFT456", "descriptions[1].contract.issuer_id");
}

#[test]
fn test_decode_contract_details() {
    // Assemble
    // This is a complex test due to the many fields in ContractDetails
    // Creating a more accurate test message with correct field structure
    let mut message = ResponseMessage::from_simple(
        "10|9001|AAPL|STK||0||SMART|USD|AAPL|Apple Inc.|AAPL|12345|0.01|100|\
    ACTIVETIM,AD,ADJUST,ALERT,ALLOC|SMART,AMEX,NYSE,NASDAQ|1000|\
    0|Apple Inc.|NASDAQ|JUN23|TECHNOLOGY|ELECTRONICS|COMPUTERS|US/Eastern|\
    20230630:0930-20230630:1600;20230701:CLOSED|20230630:0930-20230630:1600|VOL=P|1.0|1|ISIN|US0378331005|\
    1|AAPL|STK|26|20230630|COMMON|0.1|0.01|1|",
    );

    // Activate
    let contract_details = decode_contract_details(server_versions::SIZE_RULES, &mut message).expect("error decoding contract details");

    // Assert
    assert_eq!(contract_details.contract.symbol, Symbol::from("AAPL"), "contract.symbol");
    assert_eq!(contract_details.contract.security_type, SecurityType::Stock, "contract.security_type");
    assert_eq!(contract_details.contract.strike, 0.0, "contract.strike");
    assert_eq!(contract_details.contract.right, "", "contract.right");
    assert_eq!(contract_details.contract.exchange, Exchange::from("SMART"), "contract.exchange");
    assert_eq!(contract_details.contract.currency, Currency::from("USD"), "contract.currency");
    assert_eq!(contract_details.contract.local_symbol, "AAPL", "contract.local_symbol");
    assert_eq!(contract_details.market_name, "Apple Inc.", "market_name");
    assert_eq!(contract_details.contract.trading_class, "AAPL", "contract.trading_class");
    assert_eq!(contract_details.contract.contract_id, 12345, "contract.contract_id");
    assert_eq!(contract_details.min_tick, 0.01, "min_tick");
    assert_eq!(contract_details.contract.multiplier, "100", "contract.multiplier");

    // Check order types parsing
    assert_eq!(contract_details.order_types.len(), 5, "order_types.len()");
    assert_eq!(contract_details.order_types[0], "ACTIVETIM", "order_types[0]");
    assert_eq!(contract_details.order_types[1], "AD", "order_types[1]");
    assert_eq!(contract_details.order_types[2], "ADJUST", "order_types[2]");
    assert_eq!(contract_details.order_types[3], "ALERT", "order_types[3]");
    assert_eq!(contract_details.order_types[4], "ALLOC", "order_types[4]");

    // Check valid exchanges parsing
    assert_eq!(contract_details.valid_exchanges.len(), 4, "valid_exchanges.len()");
    assert_eq!(contract_details.valid_exchanges[0], "SMART", "valid_exchanges[0]");
    assert_eq!(contract_details.valid_exchanges[1], "AMEX", "valid_exchanges[1]");
    assert_eq!(contract_details.valid_exchanges[2], "NYSE", "valid_exchanges[2]");
    assert_eq!(contract_details.valid_exchanges[3], "NASDAQ", "valid_exchanges[3]");

    assert_eq!(contract_details.price_magnifier, 1000, "price_magnifier");
    assert_eq!(contract_details.under_contract_id, 0, "under_contract_id");
    assert_eq!(contract_details.long_name, "Apple Inc.", "long_name");
    assert_eq!(
        contract_details.contract.primary_exchange,
        Exchange::from("NASDAQ"),
        "contract.primary_exchange"
    );
    assert_eq!(contract_details.contract_month, "JUN23", "contract_month");
    assert_eq!(contract_details.industry, "TECHNOLOGY", "industry");
    assert_eq!(contract_details.category, "ELECTRONICS", "category");
    assert_eq!(contract_details.subcategory, "COMPUTERS", "subcategory");
    assert_eq!(contract_details.time_zone_id, "US/Eastern", "time_zone_id");

    // Check trading hours parsing
    assert_eq!(contract_details.trading_hours.len(), 2, "trading_hours.len()");
    assert_eq!(contract_details.trading_hours[0], "20230630:0930-20230630:1600", "trading_hours[0]");
    assert_eq!(contract_details.trading_hours[1], "20230701:CLOSED", "trading_hours[1]");

    // Check liquid hours parsing
    assert_eq!(contract_details.liquid_hours.len(), 1, "liquid_hours.len()");
    assert_eq!(contract_details.liquid_hours[0], "20230630:0930-20230630:1600", "liquid_hours[0]");

    assert_eq!(contract_details.ev_rule, "VOL=P", "ev_rule");
    assert_eq!(contract_details.ev_multiplier, 1.0, "ev_multiplier");

    // Check sec_id_list parsing
    assert_eq!(contract_details.sec_id_list.len(), 1, "sec_id_list.len()");
    assert_eq!(contract_details.sec_id_list[0].tag, "ISIN", "sec_id_list[0].tag");
    assert_eq!(contract_details.sec_id_list[0].value, "US0378331005", "sec_id_list[0].value");

    assert_eq!(contract_details.agg_group, 1, "agg_group");
    assert_eq!(contract_details.under_symbol, "AAPL", "under_symbol");
    assert_eq!(contract_details.under_security_type, "STK", "under_security_type");
    assert_eq!(contract_details.market_rule_ids.len(), 1, "market_rule_ids.len()");
    assert_eq!(contract_details.market_rule_ids[0], "26", "market_rule_ids[0]");
    assert_eq!(contract_details.real_expiration_date, "20230630", "real_expiration_date");
    assert_eq!(contract_details.stock_type, "COMMON", "stock_type");
    assert_eq!(contract_details.min_size, 0.1, "min_size");
    assert_eq!(contract_details.size_increment, 0.01, "size_increment");
    assert_eq!(contract_details.suggested_size_increment, 1.0, "suggested_size_increment");
}

#[test]
fn test_decode_contract_details_with_ineligibility_reasons() {
    // STK contract at v200 — no fund fields, but has ineligibility reasons
    // At v200: no message_version read (defaults to 8), LAST_TRADE_DATE field present
    let mut message = ResponseMessage::from_simple(
        "10|9001|AAPL|STK||20230630|0||SMART|USD|AAPL|Apple Inc.|AAPL|12345|0.01|100|\
        ACTIVETIM,AD|SMART,NYSE|1000|\
        0|Apple Inc.|NASDAQ|JUN23|TECHNOLOGY|ELECTRONICS|COMPUTERS|US/Eastern|\
        09:30-16:00|09:30-16:00|VOL=P|1.0|1|ISIN|US0378331005|\
        1|AAPL|STK|26|20230630|COMMON|0.1|0.01|1|\
        2|REASON1|Not eligible for margin|REASON2|Account restriction|",
    );

    let cd = decode_contract_details(200, &mut message).expect("error decoding");

    assert_eq!(cd.ineligibility_reasons.len(), 2);
    assert_eq!(cd.ineligibility_reasons[0].id, "REASON1");
    assert_eq!(cd.ineligibility_reasons[0].description, "Not eligible for margin");
    assert_eq!(cd.ineligibility_reasons[1].id, "REASON2");
    assert_eq!(cd.ineligibility_reasons[1].description, "Account restriction");
}

#[test]
fn test_decode_contract_details_fund_with_all_fields() {
    // FUND contract at v200 — has fund data fields + ineligibility reasons
    let mut message = ResponseMessage::from_simple(
        "10|9001|VFINX|FUND||20990101|0||SMART|USD|VFINX|Vanguard|VFINX|99999|0.01|1|\
        LMT,MKT|SMART|0|\
        0|Vanguard 500 Index Fund|FUNDSERV|JAN00|FUNDS|INDEX|LARGE_CAP|US/Eastern|\
        09:30-16:00|09:30-16:00||0|0|\
        1|VFINX|FUND|26|20990101|COMMON|0.01|0.001|1|\
        Vanguard 500 Index|Vanguard|Index Fund|0.5%|||\
        0.14%|0|0|0|10000|3000|1000|CA,NY|US,PR|N|004|\
        1|INEL1|Fund restriction|",
    );

    let cd = decode_contract_details(200, &mut message).expect("error decoding fund");

    assert_eq!(cd.contract.symbol, Symbol::from("VFINX"));
    assert_eq!(cd.contract.security_type, SecurityType::MutualFund);

    // Fund fields
    assert_eq!(cd.fund_name, "Vanguard 500 Index");
    assert_eq!(cd.fund_family, "Vanguard");
    assert_eq!(cd.fund_type, "Index Fund");
    assert_eq!(cd.fund_front_load, "0.5%");
    assert_eq!(cd.fund_back_load, "");
    assert_eq!(cd.fund_back_load_time_interval, "");
    assert_eq!(cd.fund_management_fee, "0.14%");
    assert!(!cd.fund_closed);
    assert!(!cd.fund_closed_for_new_investors);
    assert!(!cd.fund_closed_for_new_money);
    assert_eq!(cd.fund_notify_amount, "10000");
    assert_eq!(cd.fund_minimum_initial_purchase, "3000");
    assert_eq!(cd.fund_subsequent_minimum_purchase, "1000");
    assert_eq!(cd.fund_blue_sky_states, "CA,NY");
    assert_eq!(cd.fund_blue_sky_territories, "US,PR");
    assert_eq!(cd.fund_distribution_policy_indicator, FundDistributionPolicyIndicator::AccumulationFund);
    assert_eq!(cd.fund_asset_type, FundAssetType::Equity);

    // Ineligibility reasons
    assert_eq!(cd.ineligibility_reasons.len(), 1);
    assert_eq!(cd.ineligibility_reasons[0].id, "INEL1");
    assert_eq!(cd.ineligibility_reasons[0].description, "Fund restriction");
}

#[test]
fn test_decode_contract_details_stock_skips_fund_fields() {
    // STK at v200 — no fund fields sent, just ineligibility count = 0
    let mut message = ResponseMessage::from_simple(
        "10|9001|AAPL|STK||20230630|0||SMART|USD|AAPL|Apple Inc.|AAPL|12345|0.01|100|\
        LMT|SMART|1000|\
        0|Apple Inc.|NASDAQ|JUN23|TECH|ELEC|COMP|US/Eastern|\
        09:30-16:00|09:30-16:00|VOL=P|1.0|1|ISIN|US0378331005|\
        1|AAPL|STK|26|20230630|COMMON|0.1|0.01|1|\
        0|",
    );

    let cd = decode_contract_details(200, &mut message).expect("error decoding");

    assert_eq!(cd.contract.security_type, SecurityType::Stock);
    assert_eq!(cd.fund_name, "");
    assert!(cd.ineligibility_reasons.is_empty());
}

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
