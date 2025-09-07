use crate::{
    contracts::tick_types::TickType,
    contracts::{Currency, Exchange, SecurityType, Symbol},
    messages::ResponseMessage,
    server_versions, Error,
};

use super::super::{Contract, ContractDescription, ContractDetails, MarketRule, OptionChain, OptionComputation, PriceIncrement, TagValue};

pub(in crate::contracts) fn decode_contract_details(server_version: i32, message: &mut ResponseMessage) -> Result<ContractDetails, Error> {
    message.skip(); // message type

    let mut message_version = 8;
    if server_version < server_versions::SIZE_RULES {
        message_version = message.next_int()?;
    }

    if message_version >= 3 {
        // request id
        message.skip();
    }

    let mut contract = ContractDetails::default();

    contract.contract.symbol = Symbol::from(message.next_string()?);
    contract.contract.security_type = SecurityType::from(&message.next_string()?);
    read_last_trade_date(&mut contract, &message.next_string()?, false)?;
    contract.contract.strike = message.next_double()?;
    contract.contract.right = message.next_string()?;
    contract.contract.exchange = Exchange::from(message.next_string()?);
    contract.contract.currency = Currency::from(message.next_string()?);
    contract.contract.local_symbol = message.next_string()?;
    contract.market_name = message.next_string()?;
    contract.contract.trading_class = message.next_string()?;
    contract.contract.contract_id = message.next_int()?;
    contract.min_tick = message.next_double()?;
    if (server_versions::MD_SIZE_MULTIPLIER..server_versions::SIZE_RULES).contains(&server_version) {
        message.next_int()?; // mdSizeMultiplier no longer used
    }
    contract.contract.multiplier = message.next_string()?;
    contract.order_types = split_to_vec(&message.next_string()?);
    contract.valid_exchanges = split_to_vec(&message.next_string()?);
    if message_version >= 2 {
        contract.price_magnifier = message.next_int()?;
    }
    if message_version >= 4 {
        contract.under_contract_id = message.next_int()?;
    }
    if message_version >= 5 {
        //        https://github.com/InteractiveBrokers/tws-api/blob/817a905d52299028ac5af08581c8ffde7644cea9/source/csharpclient/client/EDecoder.cs#L1626
        contract.long_name = message.next_string()?;
        contract.contract.primary_exchange = Exchange::from(message.next_string()?);
    }
    if message_version >= 6 {
        contract.contract_month = message.next_string()?;
        contract.industry = message.next_string()?;
        contract.category = message.next_string()?;
        contract.subcategory = message.next_string()?;
        contract.time_zone_id = message.next_string()?;
        contract.trading_hours = split_hours(&message.next_string()?);
        contract.liquid_hours = split_hours(&message.next_string()?);
    }
    if message_version >= 8 {
        contract.ev_rule = message.next_string()?;
        contract.ev_multiplier = message.next_double()?;
    }
    if message_version >= 7 {
        let sec_id_list_count = message.next_int()?;
        for _ in 0..sec_id_list_count {
            let tag = message.next_string()?;
            let value = message.next_string()?;
            contract.sec_id_list.push(TagValue { tag, value });
        }
    }
    if server_version > server_versions::AGG_GROUP {
        contract.agg_group = message.next_int()?;
    }
    if server_version > server_versions::UNDERLYING_INFO {
        contract.under_symbol = message.next_string()?;
        contract.under_security_type = message.next_string()?;
    }
    if server_version > server_versions::MARKET_RULES {
        contract.market_rule_ids = split_to_vec(&message.next_string()?);
    }
    if server_version > server_versions::REAL_EXPIRATION_DATE {
        contract.real_expiration_date = message.next_string()?;
    }
    if server_version > server_versions::STOCK_TYPE {
        contract.stock_type = message.next_string()?;
    }
    if (server_versions::FRACTIONAL_SIZE_SUPPORT..server_versions::SIZE_RULES).contains(&server_version) {
        message.next_double()?; // size min tick -- no longer used
    }
    if server_version >= server_versions::SIZE_RULES {
        contract.min_size = message.next_double()?;
        contract.size_increment = message.next_double()?;
        contract.suggested_size_increment = message.next_double()?;
    }

    Ok(contract)
}

fn split_hours(hours: &str) -> Vec<String> {
    hours.split(";").map(|s| s.to_string()).collect()
}

fn split_to_vec(s: &str) -> Vec<String> {
    s.split(",").map(|s| s.to_string()).collect()
}

fn read_last_trade_date(contract: &mut ContractDetails, last_trade_date_or_contract_month: &str, is_bond: bool) -> Result<(), Error> {
    if last_trade_date_or_contract_month.is_empty() {
        return Ok(());
    }

    let splitted: Vec<&str> = if last_trade_date_or_contract_month.contains('-') {
        last_trade_date_or_contract_month.split('-').collect()
    } else {
        // let re = Regex::new(r"\s+").unwrap();
        last_trade_date_or_contract_month.split(' ').collect()
    };

    if !splitted.is_empty() {
        if is_bond {
            contract.maturity = splitted[0].to_string();
        } else {
            contract.contract.last_trade_date_or_contract_month = splitted[0].to_string();
        }
    }
    if splitted.len() > 1 {
        contract.last_trade_time = splitted[1].to_string();
    }
    if is_bond && splitted.len() > 2 {
        contract.time_zone_id = splitted[2].to_string();
    }

    Ok(())
}

pub(in crate::contracts) fn decode_contract_descriptions(
    server_version: i32,
    message: &mut ResponseMessage,
) -> Result<Vec<ContractDescription>, Error> {
    message.skip(); // message type

    let _request_id = message.next_int()?;
    let contract_descriptions_count = message.next_int()?;

    if contract_descriptions_count < 1 {
        return Ok(Vec::default());
    }

    let mut contract_descriptions: Vec<ContractDescription> = Vec::with_capacity(contract_descriptions_count as usize);

    for _ in 0..contract_descriptions_count {
        let mut contract = Contract {
            contract_id: message.next_int()?,
            symbol: Symbol::from(message.next_string()?),
            security_type: SecurityType::from(&message.next_string()?),
            primary_exchange: Exchange::from(message.next_string()?),
            currency: Currency::from(message.next_string()?),
            ..Default::default()
        };

        let derivative_security_types_count = message.next_int()?;
        let mut derivative_security_types: Vec<String> = Vec::with_capacity(derivative_security_types_count as usize);
        for _ in 0..derivative_security_types_count {
            derivative_security_types.push(message.next_string()?);
        }

        if server_version >= server_versions::BOND_ISSUERID {
            contract.description = message.next_string()?;
            contract.issuer_id = message.next_string()?;
        }

        contract_descriptions.push(ContractDescription {
            contract,
            derivative_security_types,
        });
    }

    Ok(contract_descriptions)
}

pub(in crate::contracts) fn decode_market_rule(message: &mut ResponseMessage) -> Result<MarketRule, Error> {
    message.skip(); // message type

    let mut market_rule = MarketRule {
        market_rule_id: message.next_int()?,
        ..Default::default()
    };

    let price_increments_count = message.next_int()?;
    for _ in 0..price_increments_count {
        market_rule.price_increments.push(PriceIncrement {
            low_edge: message.next_double()?,
            increment: message.next_double()?,
        });
    }

    Ok(market_rule)
}

pub(crate) fn decode_option_computation(server_version: i32, message: &mut ResponseMessage) -> Result<OptionComputation, Error> {
    message.skip(); // message type

    let message_version = if server_version >= server_versions::PRICE_BASED_VOLATILITY {
        i32::MAX
    } else {
        message.next_int()?
    };

    message.skip(); // request id

    let mut computation = OptionComputation {
        field: TickType::from(message.next_int()?),
        ..Default::default()
    };

    if server_version >= server_versions::PRICE_BASED_VOLATILITY {
        computation.tick_attribute = Some(message.next_int()?);
    }

    computation.implied_volatility = next_optional_double(message, -1.0)?;
    computation.delta = next_optional_double(message, -2.0)?;

    if message_version >= 6 || computation.field == TickType::ModelOption || computation.field == TickType::DelayedModelOption {
        computation.option_price = next_optional_double(message, -1.0)?;
        computation.present_value_dividend = next_optional_double(message, -1.0)?;
    }

    if message_version >= 6 {
        computation.gamma = next_optional_double(message, -2.0)?;
        computation.vega = next_optional_double(message, -2.0)?;
        computation.theta = next_optional_double(message, -2.0)?;
        computation.underlying_price = next_optional_double(message, -1.0)?;
    }

    Ok(computation)
}

fn next_optional_double(message: &mut ResponseMessage, none_value: f64) -> Result<Option<f64>, Error> {
    let value = message.next_double()?;
    if value == none_value {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

pub(in crate::contracts) fn decode_option_chain(message: &mut ResponseMessage) -> Result<OptionChain, Error> {
    message.skip(); // message type
    message.skip(); // request id

    let mut option_chain = OptionChain {
        exchange: message.next_string()?,
        underlying_contract_id: message.next_int()?,
        trading_class: message.next_string()?,
        multiplier: message.next_string()?,
        ..Default::default()
    };

    let expirations_count = message.next_int()?;
    option_chain.expirations.reserve(expirations_count as usize);
    for _ in 0..expirations_count {
        option_chain.expirations.push(message.next_string()?);
    }

    let strikes_count = message.next_int()?;
    option_chain.strikes.reserve(strikes_count as usize);
    for _ in 0..strikes_count {
        option_chain.strikes.push(message.next_double()?);
    }

    Ok(option_chain)
}

#[cfg(test)]
mod tests {
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
        let computation =
            decode_option_computation(server_versions::PRICE_BASED_VOLATILITY, &mut message).expect("error decoding option computation");

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
}
