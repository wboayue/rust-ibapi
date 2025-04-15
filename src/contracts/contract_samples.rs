use super::*;

// https://github.com/InteractiveBrokers/tws-api/blob/5cb24aea5cef9d315985a7b13dea7efbcfe2b16a/samples/CSharp/Testbed/ContractSamples.cs

// Future contracts also require an expiration date but are less complicated than options.
pub fn simple_future() -> Contract {
    Contract {
        symbol: "GBL".to_owned(),
        security_type: SecurityType::Future,
        exchange: "EUREX".to_owned(),
        currency: "EUR".to_owned(),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    }
}

// Rather than giving expiration dates we can also provide the local symbol
// attributes such as symbol, currency, strike, etc.
pub fn future_with_local_symbol() -> Contract {
    Contract {
        security_type: SecurityType::Future,
        exchange: "EUREX".to_owned(),
        currency: "EUR".to_owned(),
        local_symbol: "FGBL MAR 23".to_owned(),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    }
}

pub fn future_with_multiplier() -> Contract {
    Contract {
        symbol: "DAX".to_owned(),
        security_type: SecurityType::Future,
        exchange: "EUREX".to_owned(),
        currency: "EUR".to_owned(),
        last_trade_date_or_contract_month: "202303".to_owned(),
        multiplier: "1".to_owned(),
        ..Contract::default()
    }
}

pub fn smart_future_combo_contract() -> Contract {
    let leg_1 = ComboLeg {
        contract_id: 55928698, //WTI future June 2017
        ratio: 1,
        action: "BUY".to_owned(),
        exchange: "IPE".to_owned(),
        ..ComboLeg::default()
    };

    let leg_2 = ComboLeg {
        contract_id: 55850663, //COIL future June 2017
        ratio: 1,
        action: "SELL".to_owned(),
        exchange: "IPE".to_owned(),
        ..ComboLeg::default()
    };

    Contract {
        symbol: "WTI".to_owned(), // WTI,COIL spread. Symbol can be defined as first leg symbol ("WTI") or currency ("USD").
        security_type: SecurityType::Spread,
        currency: "USD".to_owned(),
        exchange: "SMART".to_owned(),
        combo_legs: vec![leg_1, leg_2],
        ..Contract::default()
    }
}

#[test]
fn test_contract_option_builder() {
    let contract = Contract::option("AAPL", "20231215", 150.0, "C");
    assert_eq!(contract.symbol, "AAPL");
    assert_eq!(contract.last_trade_date_or_contract_month, "20231215");
    assert_eq!(contract.strike, 150.0);
    assert_eq!(contract.right, "C");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.exchange, "SMART");
    assert_eq!(contract.security_type, SecurityType::Option);
}

#[test]
fn test_contract_futures_builder() {
    let contract = Contract::futures("ES");
    assert_eq!(contract.symbol, "ES");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.security_type, SecurityType::Future);
}

#[test]
fn test_contract_crypto_builder() {
    let contract = Contract::crypto("BTC");
    assert_eq!(contract.symbol, "BTC");
    assert_eq!(contract.security_type, SecurityType::Crypto);
    assert_eq!(contract.exchange, "PAXOS");
    assert_eq!(contract.currency, "USD");
}

#[test]
fn test_security_type_from_strings() {
    let cases = vec![
        ("STK", SecurityType::Stock),
        ("OPT", SecurityType::Option),
        ("FUT", SecurityType::Future),
        ("IND", SecurityType::Index),
        ("FOP", SecurityType::FuturesOption),
        ("CASH", SecurityType::ForexPair),
        ("BAG", SecurityType::Spread),
        ("WAR", SecurityType::Warrant),
        ("BOND", SecurityType::Bond),
        ("CMDTY", SecurityType::Commodity),
        ("NEWS", SecurityType::News),
        ("FUND", SecurityType::MutualFund),
        ("CRYPTO", SecurityType::Crypto),
    ];

    for (input, expected) in cases {
        assert_eq!(SecurityType::from(input), expected);
    }
}

#[test]
fn test_tag_value_to_field() {
    let tags = vec![
        TagValue {
            tag: "foo".into(),
            value: "bar".into(),
        },
        TagValue {
            tag: "baz".into(),
            value: "qux".into(),
        },
    ];

    assert_eq!(tags.to_field(), "foo=bar;baz=qux;");
}
