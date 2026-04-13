use super::*;

#[test]
fn test_contract_builder_new() {
    let builder = ContractBuilder::new();

    // All fields should be None initially
    assert_eq!(builder.symbol, None);
    assert_eq!(builder.security_type, None);
    assert_eq!(builder.exchange, None);
    assert_eq!(builder.currency, None);
}

#[test]
fn test_contract_builder_field_setters() {
    let builder = ContractBuilder::new()
        .contract_id(12345)
        .symbol("AAPL")
        .security_type(SecurityType::Stock)
        .exchange("NASDAQ")
        .currency("USD")
        .strike(150.0)
        .right("C")
        .last_trade_date_or_contract_month("20231215")
        .multiplier("100")
        .local_symbol("AAPL_123")
        .primary_exchange("NASDAQ")
        .trading_class("AAPL")
        .include_expired(true)
        .security_id_type("ISIN")
        .security_id("US0378331005")
        .combo_legs_description("Test combo")
        .issuer_id("ISSUER123")
        .description("Apple Inc.");

    assert_eq!(builder.contract_id, Some(12345));
    assert_eq!(builder.symbol, Some("AAPL".to_string()));
    assert_eq!(builder.security_type, Some(SecurityType::Stock));
    assert_eq!(builder.exchange, Some("NASDAQ".to_string()));
    assert_eq!(builder.currency, Some("USD".to_string()));
    assert_eq!(builder.strike, Some(150.0));
    assert_eq!(builder.right, Some("C".to_string()));
    assert_eq!(builder.last_trade_date_or_contract_month, Some("20231215".to_string()));
    assert_eq!(builder.multiplier, Some("100".to_string()));
    assert_eq!(builder.local_symbol, Some("AAPL_123".to_string()));
    assert_eq!(builder.primary_exchange, Some("NASDAQ".to_string()));
    assert_eq!(builder.trading_class, Some("AAPL".to_string()));
    assert_eq!(builder.include_expired, Some(true));
    assert_eq!(builder.security_id_type, Some("ISIN".to_string()));
    assert_eq!(builder.security_id, Some("US0378331005".to_string()));
    assert_eq!(builder.combo_legs_description, Some("Test combo".to_string()));
    assert_eq!(builder.issuer_id, Some("ISSUER123".to_string()));
    assert_eq!(builder.description, Some("Apple Inc.".to_string()));
}

#[test]
fn test_contract_builder_stock() {
    let builder = ContractBuilder::stock("AAPL", "NASDAQ", "USD");

    assert_eq!(builder.symbol, Some("AAPL".to_string()));
    assert_eq!(builder.security_type, Some(SecurityType::Stock));
    assert_eq!(builder.exchange, Some("NASDAQ".to_string()));
    assert_eq!(builder.currency, Some("USD".to_string()));
}

#[test]
fn test_contract_builder_futures() {
    let builder = ContractBuilder::futures("ES", "CME", "USD");

    assert_eq!(builder.symbol, Some("ES".to_string()));
    assert_eq!(builder.security_type, Some(SecurityType::Future));
    assert_eq!(builder.exchange, Some("CME".to_string()));
    assert_eq!(builder.currency, Some("USD".to_string()));
}

#[test]
fn test_contract_builder_crypto() {
    let builder = ContractBuilder::crypto("BTC", "PAXOS", "USD");

    assert_eq!(builder.symbol, Some("BTC".to_string()));
    assert_eq!(builder.security_type, Some(SecurityType::Crypto));
    assert_eq!(builder.exchange, Some("PAXOS".to_string()));
    assert_eq!(builder.currency, Some("USD".to_string()));
}

#[test]
fn test_contract_builder_option() {
    let builder = ContractBuilder::option("AAPL", "SMART", "USD");

    assert_eq!(builder.symbol, Some("AAPL".to_string()));
    assert_eq!(builder.security_type, Some(SecurityType::Option));
    assert_eq!(builder.exchange, Some("SMART".to_string()));
    assert_eq!(builder.currency, Some("USD".to_string()));
}

#[test]
fn test_contract_builder_build_stock_success() {
    let contract = ContractBuilder::stock("AAPL", "NASDAQ", "USD").contract_id(12345).build().unwrap();

    assert_eq!(contract.symbol, "AAPL");
    assert_eq!(contract.security_type, SecurityType::Stock);
    assert_eq!(contract.exchange, "NASDAQ");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.contract_id, 12345);
    assert_eq!(contract.strike, 0.0);
    assert_eq!(contract.right, "");
    assert_eq!(contract.last_trade_date_or_contract_month, "");
    assert!(!contract.include_expired);
}

#[test]
fn test_contract_builder_build_option_success() {
    let contract = ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(150.0)
        .right("C")
        .last_trade_date_or_contract_month("20231215")
        .build()
        .unwrap();

    assert_eq!(contract.symbol, "AAPL");
    assert_eq!(contract.security_type, SecurityType::Option);
    assert_eq!(contract.exchange, "SMART");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.strike, 150.0);
    assert_eq!(contract.right, "C");
    assert_eq!(contract.last_trade_date_or_contract_month, "20231215");
}

#[test]
fn test_contract_builder_build_futures_success() {
    let contract = ContractBuilder::futures("ES", "CME", "USD")
        .last_trade_date_or_contract_month("202312")
        .build()
        .unwrap();

    assert_eq!(contract.symbol, "ES");
    assert_eq!(contract.security_type, SecurityType::Future);
    assert_eq!(contract.exchange, "CME");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.last_trade_date_or_contract_month, "202312");
}

#[test]
fn test_contract_builder_build_missing_identifier() {
    let result = ContractBuilder::new().build();

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "error occurred: Symbol, local_symbol, or contract_id is required"
    );
}

#[test]
fn test_contract_builder_build_with_local_symbol_only() {
    let result = ContractBuilder::new()
        .local_symbol("FGBL MAR 23")
        .security_type(SecurityType::Future)
        .exchange("EUREX")
        .currency("EUR")
        .last_trade_date_or_contract_month("202303")
        .build();

    assert!(result.is_ok());
    let contract = result.unwrap();
    assert_eq!(contract.symbol, ""); // Empty symbol is OK when local_symbol is provided
    assert_eq!(contract.local_symbol, "FGBL MAR 23");
    assert_eq!(contract.security_type, SecurityType::Future);
    assert_eq!(contract.exchange, "EUREX");
    assert_eq!(contract.currency, "EUR");
}

#[test]
fn test_contract_builder_build_with_contract_id_only() {
    let result = ContractBuilder::new().contract_id(265598).exchange("SMART").build();

    assert!(result.is_ok());
    let contract = result.unwrap();
    assert_eq!(contract.symbol, ""); // Empty symbol is OK when contract_id is provided
    assert_eq!(contract.local_symbol, "");
    assert_eq!(contract.contract_id, 265598);
    assert_eq!(contract.exchange, "SMART");
}

#[test]
fn test_contract_builder_build_option_missing_strike() {
    let result = ContractBuilder::option("AAPL", "SMART", "USD")
        .right("C")
        .last_trade_date_or_contract_month("20231215")
        .build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Strike price is required for options");
}

#[test]
fn test_contract_builder_build_option_missing_right() {
    let result = ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(150.0)
        .last_trade_date_or_contract_month("20231215")
        .build();

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "error occurred: Right (P for PUT or C for CALL) is required for options"
    );
}

#[test]
fn test_contract_builder_build_option_missing_expiration() {
    let result = ContractBuilder::option("AAPL", "SMART", "USD").strike(150.0).right("C").build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Expiration date is required for options");
}

#[test]
fn test_contract_builder_build_futures_missing_contract_month() {
    let result = ContractBuilder::futures("ES", "CME", "USD").build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Contract month is required for futures");
}

#[test]
fn test_contract_builder_build_futures_option_missing_contract_month() {
    let result = ContractBuilder::new()
        .symbol("ES")
        .security_type(SecurityType::FuturesOption)
        .exchange("CME")
        .currency("USD")
        .build();

    assert!(result.is_err());
    // FuturesOption is checked as an option first, so it fails on missing strike price
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Strike price is required for options");
}

#[test]
fn test_contract_builder_build_invalid_option_right() {
    let result = ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(150.0)
        .right("INVALID")
        .last_trade_date_or_contract_month("20231215")
        .build();

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "error occurred: Option right must be P for PUT or C for CALL"
    );
}

#[test]
fn test_contract_builder_build_valid_option_rights() {
    let valid_rights = ["P", "C", "p", "c"];

    for right in &valid_rights {
        let result = ContractBuilder::option("AAPL", "SMART", "USD")
            .strike(150.0)
            .right(*right)
            .last_trade_date_or_contract_month("20231215")
            .build();

        assert!(result.is_ok(), "Right '{}' should be valid", right);
    }
}

#[test]
fn test_contract_builder_build_negative_strike() {
    let result = ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(-10.0)
        .right("C")
        .last_trade_date_or_contract_month("20231215")
        .build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Strike price cannot be negative");
}

#[test]
fn test_contract_builder_combo_legs() {
    let combo_legs = vec![
        ComboLeg {
            contract_id: 12345,
            ratio: 1,
            action: "BUY".to_string(),
            exchange: Exchange::from("SMART"),
            ..Default::default()
        },
        ComboLeg {
            contract_id: 67890,
            ratio: 1,
            action: "SELL".to_string(),
            exchange: Exchange::from("SMART"),
            ..Default::default()
        },
    ];

    let contract = ContractBuilder::new()
        .symbol("SPREAD")
        .security_type(SecurityType::Spread)
        .combo_legs(combo_legs.clone())
        .build()
        .unwrap();

    assert_eq!(contract.combo_legs.len(), 2);
    assert_eq!(contract.combo_legs[0].contract_id, 12345);
    assert_eq!(contract.combo_legs[0].action, "BUY");
    assert_eq!(contract.combo_legs[1].contract_id, 67890);
    assert_eq!(contract.combo_legs[1].action, "SELL");
}

#[test]
fn test_contract_builder_delta_neutral_contract() {
    let delta_neutral = DeltaNeutralContract {
        contract_id: 12345,
        delta: 0.5,
        price: 100.0,
    };

    let contract = ContractBuilder::stock("AAPL", "NASDAQ", "USD")
        .delta_neutral_contract(delta_neutral.clone())
        .build()
        .unwrap();

    assert!(contract.delta_neutral_contract.is_some());
    let delta_neutral_result = contract.delta_neutral_contract.unwrap();
    assert_eq!(delta_neutral_result.contract_id, 12345);
    assert_eq!(delta_neutral_result.delta, 0.5);
    assert_eq!(delta_neutral_result.price, 100.0);
}

#[test]
fn test_contract_builder_chaining() {
    // Test that builder methods can be chained fluently
    let contract = ContractBuilder::new()
        .symbol("TSLA")
        .security_type(SecurityType::Stock)
        .exchange("NASDAQ")
        .currency("USD")
        .contract_id(76792991)
        .primary_exchange("NASDAQ")
        .local_symbol("TSLA")
        .trading_class("TSLA")
        .description("Tesla Inc.")
        .build()
        .unwrap();

    assert_eq!(contract.symbol, "TSLA");
    assert_eq!(contract.security_type, SecurityType::Stock);
    assert_eq!(contract.exchange, "NASDAQ");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.contract_id, 76792991);
    assert_eq!(contract.primary_exchange, "NASDAQ");
    assert_eq!(contract.local_symbol, "TSLA");
    assert_eq!(contract.trading_class, "TSLA");
    assert_eq!(contract.description, "Tesla Inc.");
}

#[test]
fn test_contract_builder_defaults() {
    // Test that unset fields get proper defaults
    let contract = ContractBuilder::new().symbol("TEST").build().unwrap();

    assert_eq!(contract.contract_id, 0);
    assert_eq!(contract.symbol, "TEST");
    assert_eq!(contract.security_type, SecurityType::Stock); // Default
    assert_eq!(contract.last_trade_date_or_contract_month, "");
    assert_eq!(contract.strike, 0.0);
    assert_eq!(contract.right, "");
    assert_eq!(contract.multiplier, "");
    assert_eq!(contract.exchange, "");
    assert_eq!(contract.currency, "");
    assert_eq!(contract.local_symbol, "");
    assert_eq!(contract.primary_exchange, "");
    assert_eq!(contract.trading_class, "");
    assert!(!contract.include_expired);
    assert_eq!(contract.security_id_type, "");
    assert_eq!(contract.security_id, "");
    assert_eq!(contract.combo_legs_description, "");
    assert!(contract.combo_legs.is_empty());
    assert!(contract.delta_neutral_contract.is_none());
    assert_eq!(contract.issuer_id, "");
    assert_eq!(contract.description, "");
}
