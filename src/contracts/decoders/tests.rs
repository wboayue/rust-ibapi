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
#[ignore]
fn test_decode_option_chain() {
    // Assemble
    let mut message = ResponseMessage::from_simple("51|123|NYSE|789456|GOOG|100|2023-06,2023-09,2023-12|100.5,110.0,120.5|");
    
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
    assert_eq!(contract.contract.last_trade_date_or_contract_month, "2023", "contract.last_trade_date_or_contract_month");
    assert_eq!(contract.last_trade_time, "06", "contract.last_trade_time");
    
    // Test with space separator
    // Assemble
    let mut contract = ContractDetails::default();
    
    // Activate
    read_last_trade_date(&mut contract, "2023 06 15", false).expect("error reading last trade date");
    
    // Assert
    assert_eq!(contract.contract.last_trade_date_or_contract_month, "2023", "contract.last_trade_date_or_contract_month");
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
    assert_eq!(contract.contract.last_trade_date_or_contract_month, "", "contract.last_trade_date_or_contract_month should be empty");
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
    let mut message = ResponseMessage::from_simple("81|42|2|12345|AAPL|STK|NASDAQ|USD|2|OPT|WAR|Apple Inc.|AAPL123|67890|MSFT|STK|NASDAQ|USD|1|OPT|Microsoft Corp.|MSFT456|");
    
    // Activate
    let descriptions = decode_contract_descriptions(server_versions::BOND_ISSUERID, &mut message).expect("error decoding contract descriptions");
    
    // Assert
    assert_eq!(descriptions.len(), 2, "descriptions.len()");
    
    // First contract
    assert_eq!(descriptions[0].contract.contract_id, 12345, "descriptions[0].contract.contract_id");
    assert_eq!(descriptions[0].contract.symbol, "AAPL", "descriptions[0].contract.symbol");
    assert_eq!(descriptions[0].contract.security_type, SecurityType::Stock, "descriptions[0].contract.security_type");
    assert_eq!(descriptions[0].contract.primary_exchange, "NASDAQ", "descriptions[0].contract.primary_exchange");
    assert_eq!(descriptions[0].contract.currency, "USD", "descriptions[0].contract.currency");
    assert_eq!(descriptions[0].derivative_security_types.len(), 2, "descriptions[0].derivative_security_types.len()");
    assert_eq!(descriptions[0].derivative_security_types[0], "OPT", "descriptions[0].derivative_security_types[0]");
    assert_eq!(descriptions[0].derivative_security_types[1], "WAR", "descriptions[0].derivative_security_types[1]");
    assert_eq!(descriptions[0].contract.description, "Apple Inc.", "descriptions[0].contract.description");
    assert_eq!(descriptions[0].contract.issuer_id, "AAPL123", "descriptions[0].contract.issuer_id");
    
    // Second contract
    assert_eq!(descriptions[1].contract.contract_id, 67890, "descriptions[1].contract.contract_id");
    assert_eq!(descriptions[1].contract.symbol, "MSFT", "descriptions[1].contract.symbol");
    assert_eq!(descriptions[1].contract.security_type, SecurityType::Stock, "descriptions[1].contract.security_type");
    assert_eq!(descriptions[1].contract.primary_exchange, "NASDAQ", "descriptions[1].contract.primary_exchange");
    assert_eq!(descriptions[1].contract.currency, "USD", "descriptions[1].contract.currency");
    assert_eq!(descriptions[1].derivative_security_types.len(), 1, "descriptions[1].derivative_security_types.len()");
    assert_eq!(descriptions[1].derivative_security_types[0], "OPT", "descriptions[1].derivative_security_types[0]");
    assert_eq!(descriptions[1].contract.description, "Microsoft Corp.", "descriptions[1].contract.description");
    assert_eq!(descriptions[1].contract.issuer_id, "MSFT456", "descriptions[1].contract.issuer_id");
}

#[test]
#[ignore]
fn test_decode_contract_details() {
    // Assemble
    // This is a complex test due to the many fields in ContractDetails
    // Creating a simplified test message with essential fields for different server versions
    let mut message = ResponseMessage::from_simple("10|8|42|AAPL|STK|2023-06-15|0.0||\
        SMART|USD|AAPL|Apple Inc.|AAPL|12345|0.01|100|SMART,NYSE|SMART,NYSE,NASDAQ|1000|\
        0|Apple Inc.|NASDAQ|JUN23|TECHNOLOGY|ELECTRONICS|COMPUTERS|US/Eastern|\
        09:30:00-16:00:00;09:30:00-16:00:00|09:30:00-16:00:00|VOL=P|1.0|2|TAG1|VALUE1|TAG2|VALUE2|\
        1|AAPL|STK|26|2023-06-15|COMMON|0.1|0.01|1|");
    
    // Activate
    let contract_details = decode_contract_details(server_versions::SIZE_RULES, &mut message).expect("error decoding contract details");
    
    // Assert
    assert_eq!(contract_details.contract.symbol, "AAPL", "contract.symbol");
    assert_eq!(contract_details.contract.security_type, SecurityType::Stock, "contract.security_type");
    assert_eq!(contract_details.contract.last_trade_date_or_contract_month, "2023-06-15", "contract.last_trade_date_or_contract_month");
    assert_eq!(contract_details.contract.strike, 0.0, "contract.strike");
    assert_eq!(contract_details.contract.right, "", "contract.right");
    assert_eq!(contract_details.contract.exchange, "SMART", "contract.exchange");
    assert_eq!(contract_details.contract.currency, "USD", "contract.currency");
    assert_eq!(contract_details.contract.local_symbol, "AAPL", "contract.local_symbol");
    assert_eq!(contract_details.market_name, "Apple Inc.", "market_name");
    assert_eq!(contract_details.contract.trading_class, "AAPL", "contract.trading_class");
    assert_eq!(contract_details.contract.contract_id, 12345, "contract.contract_id");
    assert_eq!(contract_details.min_tick, 0.01, "min_tick");
    assert_eq!(contract_details.contract.multiplier, "100", "contract.multiplier");
    assert_eq!(contract_details.order_types.len(), 2, "order_types.len()");
    assert_eq!(contract_details.valid_exchanges.len(), 3, "valid_exchanges.len()");
    assert_eq!(contract_details.price_magnifier, 1000, "price_magnifier");
    assert_eq!(contract_details.under_contract_id, 0, "under_contract_id");
    assert_eq!(contract_details.long_name, "Apple Inc.", "long_name");
    assert_eq!(contract_details.contract.primary_exchange, "NASDAQ", "contract.primary_exchange");
    assert_eq!(contract_details.contract_month, "JUN23", "contract_month");
    assert_eq!(contract_details.industry, "TECHNOLOGY", "industry");
    assert_eq!(contract_details.category, "ELECTRONICS", "category");
    assert_eq!(contract_details.subcategory, "COMPUTERS", "subcategory");
    assert_eq!(contract_details.time_zone_id, "US/Eastern", "time_zone_id");
    assert_eq!(contract_details.trading_hours.len(), 2, "trading_hours.len()");
    assert_eq!(contract_details.liquid_hours.len(), 1, "liquid_hours.len()");
    assert_eq!(contract_details.ev_rule, "VOL=P", "ev_rule");
    assert_eq!(contract_details.ev_multiplier, 1.0, "ev_multiplier");
    assert_eq!(contract_details.sec_id_list.len(), 2, "sec_id_list.len()");
    assert_eq!(contract_details.sec_id_list[0].tag, "TAG1", "sec_id_list[0].tag");
    assert_eq!(contract_details.sec_id_list[0].value, "VALUE1", "sec_id_list[0].value");
    assert_eq!(contract_details.agg_group, 1, "agg_group");
    assert_eq!(contract_details.under_symbol, "AAPL", "under_symbol");
    assert_eq!(contract_details.under_security_type, "STK", "under_security_type");
    assert_eq!(contract_details.market_rule_ids.len(), 1, "market_rule_ids.len()");
    assert_eq!(contract_details.market_rule_ids[0], "26", "market_rule_ids[0]");
    assert_eq!(contract_details.real_expiration_date, "2023-06-15", "real_expiration_date");
    assert_eq!(contract_details.stock_type, "COMMON", "stock_type");
    assert_eq!(contract_details.min_size, 0.1, "min_size");
    assert_eq!(contract_details.size_increment, 0.01, "size_increment");
    assert_eq!(contract_details.suggested_size_increment, 1.0, "suggested_size_increment");
}