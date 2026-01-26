use super::*;
use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};

#[test]
fn test_stock_builder_basic() {
    let stock = Contract::stock("AAPL").build();
    assert_eq!(stock.symbol, Symbol::from("AAPL"));
    assert_eq!(stock.security_type, SecurityType::Stock);
    assert_eq!(stock.exchange, Exchange::from("SMART"));
    assert_eq!(stock.currency, Currency::from("USD"));
}

#[test]
fn test_stock_builder_customization() {
    let stock = Contract::stock("7203")
        .on_exchange("TSEJ")
        .in_currency("JPY")
        .primary("TSEJ")
        .trading_class("TOPIX")
        .build();

    assert_eq!(stock.symbol, Symbol::from("7203"));
    assert_eq!(stock.exchange, Exchange::from("TSEJ"));
    assert_eq!(stock.currency, Currency::from("JPY"));
    assert_eq!(stock.primary_exchange, Exchange::from("TSEJ"));
    assert_eq!(stock.trading_class, "TOPIX");
}

#[test]
fn test_call_option_builder() {
    let call = Contract::call("AAPL").strike(150.0).expires_on(2024, 12, 20).build();

    assert_eq!(call.symbol, Symbol::from("AAPL"));
    assert_eq!(call.security_type, SecurityType::Option);
    assert_eq!(call.strike, 150.0);
    assert_eq!(call.right, "C");
    assert_eq!(call.last_trade_date_or_contract_month, "20241220");
    assert_eq!(call.multiplier, "100");
}

#[test]
fn test_put_option_builder() {
    let put = Contract::put("SPY")
        .strike(450.0)
        .expires(ExpirationDate::new(2024, 3, 15))
        .on_exchange("CBOE")
        .in_currency("USD")
        .primary("CBOE")
        .trading_class("SPXW")
        .multiplier(100)
        .build();

    assert_eq!(put.symbol, Symbol::from("SPY"));
    assert_eq!(put.security_type, SecurityType::Option);
    assert_eq!(put.strike, 450.0);
    assert_eq!(put.right, "P");
    assert_eq!(put.last_trade_date_or_contract_month, "20240315");
    assert_eq!(put.exchange, Exchange::from("CBOE"));
    assert_eq!(put.multiplier, "100");
    assert_eq!(put.primary_exchange, Exchange::from("CBOE"));
    assert_eq!(put.trading_class, "SPXW");
}

#[test]
fn test_invalid_strike_price() {
    let result = Strike::new(-10.0);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Strike price must be positive");

    let result = Strike::new(0.0);
    assert!(result.is_err());

    let result = Strike::new(100.0);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().value(), 100.0);
}

#[test]
fn test_futures_builder_with_manual_expiry() {
    let futures = Contract::futures("ES")
        .expires_in(ContractMonth::new(2024, 3))
        .on_exchange("GLOBEX")
        .in_currency("USD")
        .multiplier(50)
        .build();

    assert_eq!(futures.symbol, Symbol::from("ES"));
    assert_eq!(futures.security_type, SecurityType::Future);
    assert_eq!(futures.last_trade_date_or_contract_month, "202403");
    assert_eq!(futures.exchange, Exchange::from("GLOBEX"));
    assert_eq!(futures.currency, Currency::from("USD"));
    assert_eq!(futures.multiplier, "50");
}

#[test]
fn test_futures_multiplier() {
    // Default: no multiplier set (empty string)
    let es = Contract::futures("ES").expires_in(ContractMonth::new(2024, 3)).build();
    assert_eq!(es.multiplier, "");

    let nq = Contract::futures("NQ").expires_in(ContractMonth::new(2024, 3)).build();
    assert_eq!(nq.multiplier, "");

    // Explicit multiplier is used when specified
    let custom = Contract::futures("ES").expires_in(ContractMonth::new(2024, 3)).multiplier(50).build();
    assert_eq!(custom.multiplier, "50");

    let cl = Contract::futures("CL").expires_in(ContractMonth::new(2024, 3)).multiplier(1000).build();
    assert_eq!(cl.multiplier, "1000");
}

#[test]
fn test_forex_builder() {
    let forex = Contract::forex("EUR", "USD").on_exchange("IDEALPRO").build();

    assert_eq!(forex.symbol, Symbol::from("EUR"));
    assert_eq!(forex.security_type, SecurityType::ForexPair);
    assert_eq!(forex.exchange, Exchange::from("IDEALPRO"));
    assert_eq!(forex.currency, Currency::from("USD"));
}

#[test]
fn test_crypto_builder() {
    let btc = Contract::crypto("BTC").on_exchange("PAXOS").in_currency("USD").build();

    assert_eq!(btc.symbol, Symbol::from("BTC"));
    assert_eq!(btc.security_type, SecurityType::Crypto);
    assert_eq!(btc.exchange, Exchange::from("PAXOS"));
    assert_eq!(btc.currency, Currency::from("USD"));
}

#[test]
fn test_index_contract() {
    // SPX should get CBOE and USD
    let spx = Contract::index("SPX");
    assert_eq!(spx.symbol, Symbol::from("SPX"));
    assert_eq!(spx.security_type, SecurityType::Index);
    assert_eq!(spx.exchange, Exchange::from("CBOE"));
    assert_eq!(spx.currency, Currency::from("USD"));

    // DAX should get EUREX and EUR
    let dax = Contract::index("DAX");
    assert_eq!(dax.symbol, Symbol::from("DAX"));
    assert_eq!(dax.exchange, Exchange::from("EUREX"));
    assert_eq!(dax.currency, Currency::from("EUR"));

    // FTSE should get LSE and GBP
    let ftse = Contract::index("FTSE");
    assert_eq!(ftse.symbol, Symbol::from("FTSE"));
    assert_eq!(ftse.exchange, Exchange::from("LSE"));
    assert_eq!(ftse.currency, Currency::from("GBP"));

    // Unknown should get SMART and USD
    let unknown = Contract::index("XYZ");
    assert_eq!(unknown.symbol, Symbol::from("XYZ"));
    assert_eq!(unknown.exchange, Exchange::from("SMART"));
    assert_eq!(unknown.currency, Currency::from("USD"));
}

#[test]
fn test_spread_builder_calendar() {
    let spread = Contract::spread()
        .calendar(12345, 67890)
        .in_currency("USD")
        .on_exchange("SMART")
        .build()
        .unwrap();

    assert_eq!(spread.security_type, SecurityType::Spread);
    assert_eq!(spread.combo_legs.len(), 2);
    assert_eq!(spread.combo_legs[0].contract_id, 12345);
    assert_eq!(spread.combo_legs[0].action, "BUY");
    assert_eq!(spread.combo_legs[0].ratio, 1);
    assert_eq!(spread.combo_legs[1].contract_id, 67890);
    assert_eq!(spread.combo_legs[1].action, "SELL");
    assert_eq!(spread.combo_legs[1].ratio, 1);
    assert_eq!(spread.currency, Currency::from("USD"));
    assert_eq!(spread.exchange, Exchange::from("SMART"));
}

#[test]
fn test_spread_builder_vertical() {
    let spread = Contract::spread().vertical(11111, 22222).build().unwrap();

    assert_eq!(spread.security_type, SecurityType::Spread);
    assert_eq!(spread.combo_legs.len(), 2);
    assert_eq!(spread.combo_legs[0].contract_id, 11111);
    assert_eq!(spread.combo_legs[0].action, "BUY");
    assert_eq!(spread.combo_legs[1].contract_id, 22222);
    assert_eq!(spread.combo_legs[1].action, "SELL");
}

#[test]
fn test_spread_builder_custom_legs() {
    let spread = Contract::spread()
        .add_leg(10001, LegAction::Buy)
        .ratio(2)
        .on_exchange("CBOE")
        .done()
        .add_leg(10002, LegAction::Sell)
        .ratio(3)
        .done()
        .add_leg(10003, LegAction::Buy)
        .ratio(1)
        .done()
        .build()
        .unwrap();

    assert_eq!(spread.security_type, SecurityType::Spread);
    assert_eq!(spread.combo_legs.len(), 3);

    assert_eq!(spread.combo_legs[0].contract_id, 10001);
    assert_eq!(spread.combo_legs[0].action, "BUY");
    assert_eq!(spread.combo_legs[0].ratio, 2);
    assert_eq!(spread.combo_legs[0].exchange, "CBOE");

    assert_eq!(spread.combo_legs[1].contract_id, 10002);
    assert_eq!(spread.combo_legs[1].action, "SELL");
    assert_eq!(spread.combo_legs[1].ratio, 3);

    assert_eq!(spread.combo_legs[2].contract_id, 10003);
    assert_eq!(spread.combo_legs[2].action, "BUY");
    assert_eq!(spread.combo_legs[2].ratio, 1);
}

#[test]
fn test_spread_builder_empty_fails() {
    let result = Contract::spread().build();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Spread must have at least one leg");
}

#[test]
fn test_exchange_display() {
    assert_eq!(Exchange("SMART".to_string()).to_string(), "SMART");
    assert_eq!(Exchange("NASDAQ".to_string()).to_string(), "NASDAQ");
    assert_eq!(Exchange("CBOE".to_string()).to_string(), "CBOE");
    assert_eq!(Exchange("TEST".to_string()).to_string(), "TEST");
}

#[test]
fn test_currency_display() {
    assert_eq!(Currency("USD".to_string()).to_string(), "USD");
    assert_eq!(Currency("EUR".to_string()).to_string(), "EUR");
    assert_eq!(Currency("JPY".to_string()).to_string(), "JPY");
    assert_eq!(Currency("XXX".to_string()).to_string(), "XXX");
}

#[test]
fn test_option_right_display() {
    assert_eq!(OptionRight::Call.to_string(), "C");
    assert_eq!(OptionRight::Put.to_string(), "P");
}

#[test]
fn test_action_display() {
    assert_eq!(LegAction::Buy.to_string(), "BUY");
    assert_eq!(LegAction::Sell.to_string(), "SELL");
}

#[test]
fn test_symbol_conversion() {
    let symbol = Symbol::new("AAPL");
    assert_eq!(symbol.as_str(), "AAPL");
    assert_eq!(symbol.to_string(), "AAPL");

    let symbol: Symbol = "MSFT".into();
    assert_eq!(symbol.as_str(), "MSFT");

    let symbol: Symbol = Symbol("TSLA".to_string());
    assert_eq!(symbol.as_str(), "TSLA");
}

#[test]
fn test_expiration_date_formatting() {
    let date = ExpirationDate::new(2024, 12, 20);
    assert_eq!(date.to_string(), "20241220");

    let date = ExpirationDate::new(2025, 1, 5);
    assert_eq!(date.to_string(), "20250105");
}

#[test]
fn test_contract_month_formatting() {
    let month = ContractMonth::new(2024, 3);
    assert_eq!(month.to_string(), "202403");

    let month = ContractMonth::new(2025, 12);
    assert_eq!(month.to_string(), "202512");
}

#[test]
fn test_default_implementations() {
    assert_eq!(Exchange::default().as_str(), "SMART");
    assert_eq!(Currency::default().as_str(), "USD");

    let spread = SpreadBuilder::default();
    assert!(spread.legs.is_empty());
    assert_eq!(spread.currency.as_str(), "USD");
    assert_eq!(spread.exchange.as_str(), "SMART");
}

#[test]
fn test_iron_condor_spread() {
    let spread = Contract::spread()
        .iron_condor(100, 105, 110, 115)
        .build()
        .expect("Failed to build iron condor");

    assert_eq!(spread.security_type, SecurityType::Spread);
    assert_eq!(spread.combo_legs.len(), 4);

    // Verify leg structure: Buy 100 Put, Sell 105 Put, Sell 110 Call, Buy 115 Call
    let legs = &spread.combo_legs;

    // First leg: Buy 100 Put
    assert_eq!(legs[0].contract_id, 100);
    assert_eq!(legs[0].action, "BUY");
    assert_eq!(legs[0].ratio, 1);

    // Second leg: Sell 105 Put
    assert_eq!(legs[1].contract_id, 105);
    assert_eq!(legs[1].action, "SELL");
    assert_eq!(legs[1].ratio, 1);

    // Third leg: Sell 110 Call
    assert_eq!(legs[2].contract_id, 110);
    assert_eq!(legs[2].action, "SELL");
    assert_eq!(legs[2].ratio, 1);

    // Fourth leg: Buy 115 Call
    assert_eq!(legs[3].contract_id, 115);
    assert_eq!(legs[3].action, "BUY");
    assert_eq!(legs[3].ratio, 1);
}

#[test]
fn test_option_convenience_date_methods() {
    // Test expires_weekly - should set next Friday
    let weekly_option = Contract::call("SPY").strike(450.0).expires_weekly().build();

    // Weekly expiration should be set
    assert!(!weekly_option.last_trade_date_or_contract_month.is_empty());
    // Should be a Friday date in format YYYYMMDD
    assert_eq!(weekly_option.last_trade_date_or_contract_month.len(), 8);

    // Test expires_monthly - should set third Friday of month
    let monthly_option = Contract::put("AAPL").strike(150.0).expires_monthly().build();

    // Monthly expiration should be set
    assert!(!monthly_option.last_trade_date_or_contract_month.is_empty());
    assert_eq!(monthly_option.last_trade_date_or_contract_month.len(), 8);

    // Test front_month for futures
    let front_month_future = Contract::futures("ES").front_month().build();

    // Front month should be set (actual value depends on current date)
    assert!(!front_month_future.last_trade_date_or_contract_month.is_empty());
    // Should be in YYYYMM format for futures
    assert_eq!(front_month_future.last_trade_date_or_contract_month.len(), 6);

    // Test next_quarter for futures
    let next_quarter_future = Contract::futures("ES").next_quarter().build();

    // Next quarter should be set
    assert!(!next_quarter_future.last_trade_date_or_contract_month.is_empty());
    assert_eq!(next_quarter_future.last_trade_date_or_contract_month.len(), 6);
}
