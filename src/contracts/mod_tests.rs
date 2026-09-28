use super::*;
use crate::common::test_utils::wire_enum::{check_wire_enum_rejects_unknown, check_wire_enum_round_trip};

#[test]
fn test_v2_builders() {
    // Test stock builder
    let stock = Contract::stock("AAPL").build();
    assert_eq!(stock.symbol, Symbol::from("AAPL"), "stock.symbol");
    assert_eq!(stock.security_type, SecurityType::Stock, "stock.security_type");
    assert_eq!(stock.currency, Currency::from("USD"), "stock.currency");
    assert_eq!(stock.exchange, Exchange::from("SMART"), "stock.exchange");

    // Test stock with customization
    let toyota = Contract::stock("7203").on_exchange("TSEJ").in_currency("JPY").build();
    assert_eq!(toyota.symbol, Symbol::from("7203"));
    assert_eq!(toyota.exchange, Exchange::from("TSEJ"));
    assert_eq!(toyota.currency, Currency::from("JPY"));

    // Test call option builder
    let call = Contract::call("AAPL").strike(150.0).expires_on(2023, 12, 15).build();
    assert_eq!(call.symbol, Symbol::from("AAPL"));
    assert_eq!(call.security_type, SecurityType::Option);
    assert_eq!(call.strike, 150.0);
    assert_eq!(call.right, Some(OptionRight::Call));
    assert_eq!(call.last_trade_date_or_contract_month, "20231215");

    // Test put option builder
    let put = Contract::put("SPY").strike(450.0).expires_on(2024, 1, 19).build();
    assert_eq!(put.symbol, Symbol::from("SPY"));
    assert_eq!(put.right, Some(OptionRight::Put));
    assert_eq!(put.strike, 450.0);

    // Test crypto builder
    let btc = Contract::crypto("BTC").build();
    assert_eq!(btc.symbol, Symbol::from("BTC"));
    assert_eq!(btc.security_type, SecurityType::Crypto);
    assert_eq!(btc.currency, Currency::from("USD"));
    assert_eq!(btc.exchange, Exchange::from("PAXOS"));

    // Test index
    let spx = Contract::index("SPX");
    assert_eq!(spx.symbol, Symbol::from("SPX"));
    assert_eq!(spx.security_type, SecurityType::Index);
    assert_eq!(spx.exchange, Exchange::from("CBOE"));
    assert_eq!(spx.currency, Currency::from("USD"));

    // Test news constructor (unchanged)
    let news = Contract::news("BZ");
    assert_eq!(news.symbol, Symbol::from("BZ:BZ_ALL"));
    assert_eq!(news.security_type, SecurityType::News);
    assert_eq!(news.exchange, Exchange::from("BZ"));

    // Test backward compatibility with option constructor
    let option = Contract::option("AAPL", "20231215", 150.0, OptionRight::Call);
    assert_eq!(option.symbol, Symbol::from("AAPL"));
    assert_eq!(option.security_type, SecurityType::Option);
    assert_eq!(option.strike, 150.0);
    assert_eq!(option.right, Some(OptionRight::Call));
}

/// Every modeled `SecurityType` with its wire string. A new variant stops
/// `all_security_types_covers_every_variant` compiling until it gets an arm there; give it the
/// next index, grow `seen`, and add its row here.
const ALL_SECURITY_TYPES: [(SecurityType, &str); 15] = [
    (SecurityType::Stock, "STK"),
    (SecurityType::Option, "OPT"),
    (SecurityType::Future, "FUT"),
    (SecurityType::ContinuousFuture, "CONTFUT"),
    (SecurityType::Index, "IND"),
    (SecurityType::FuturesOption, "FOP"),
    (SecurityType::ForexPair, "CASH"),
    (SecurityType::Spread, "BAG"),
    (SecurityType::Warrant, "WAR"),
    (SecurityType::Bond, "BOND"),
    (SecurityType::Commodity, "CMDTY"),
    (SecurityType::News, "NEWS"),
    (SecurityType::MutualFund, "FUND"),
    (SecurityType::Crypto, "CRYPTO"),
    (SecurityType::CFD, "CFD"),
];

#[test]
fn security_type_round_trip() {
    check_wire_enum_round_trip(&ALL_SECURITY_TYPES);
    // Unrecognized values are preserved verbatim; case-sensitive, so
    // lowercase lands here rather than on the typed variant.
    check_wire_enum_round_trip(&[(SecurityType::Other("XYZ".into()), "XYZ"), (SecurityType::Other("stk".into()), "stk")]);
}

#[test]
fn security_type_from_str_rejects_empty() {
    // Open enum: empty is the only input FromStr rejects.
    check_wire_enum_rejects_unknown::<SecurityType>(&[""]);
}

/// Guard that `ALL_SECURITY_TYPES` lists every modeled variant. A new variant
/// has no arm in `modeled_index` and fails to compile. At runtime it fails if
/// any index below `seen.len()` has no row; a new variant whose row is missing
/// still passes if `seen` was not grown, so growing `seen` and adding the row
/// stay manual steps. Same shape as `all_tifs_covers_every_variant` in
/// `src/orders/tests.rs`.
#[test]
fn all_security_types_covers_every_variant() {
    fn modeled_index(security_type: &SecurityType) -> Option<usize> {
        match security_type {
            SecurityType::Stock => Some(0),
            SecurityType::Option => Some(1),
            SecurityType::Future => Some(2),
            SecurityType::ContinuousFuture => Some(3),
            SecurityType::Index => Some(4),
            SecurityType::FuturesOption => Some(5),
            SecurityType::ForexPair => Some(6),
            SecurityType::Spread => Some(7),
            SecurityType::Warrant => Some(8),
            SecurityType::Bond => Some(9),
            SecurityType::Commodity => Some(10),
            SecurityType::News => Some(11),
            SecurityType::MutualFund => Some(12),
            SecurityType::Crypto => Some(13),
            SecurityType::CFD => Some(14),
            // Other carries a raw string, so it has no place in a table of
            // modeled wire values; `security_type_round_trip` covers it.
            SecurityType::Other(_) => None,
        }
    }

    let mut seen = [false; 15];
    for (variant, _) in &ALL_SECURITY_TYPES {
        if let Some(index) = modeled_index(variant) {
            seen[index] = true;
        }
    }
    assert!(seen.iter().all(|&s| s), "ALL_SECURITY_TYPES is missing a SecurityType variant");
}

#[test]
fn test_security_type_display_all_variants() {
    let cases = [
        (SecurityType::Stock, "STK"),
        (SecurityType::Option, "OPT"),
        (SecurityType::Future, "FUT"),
        (SecurityType::ContinuousFuture, "CONTFUT"),
        (SecurityType::Index, "IND"),
        (SecurityType::FuturesOption, "FOP"),
        (SecurityType::ForexPair, "CASH"),
        (SecurityType::Spread, "BAG"),
        (SecurityType::Warrant, "WAR"),
        (SecurityType::Bond, "BOND"),
        (SecurityType::Commodity, "CMDTY"),
        (SecurityType::News, "NEWS"),
        (SecurityType::MutualFund, "FUND"),
        (SecurityType::Crypto, "CRYPTO"),
        (SecurityType::CFD, "CFD"),
        (SecurityType::Other("XYZ".to_string()), "XYZ"),
    ];
    for (variant, expected) in &cases {
        assert_eq!(format!("{variant}"), *expected, "Display mismatch for {variant:?}");
        assert_eq!(variant.to_field(), *expected, "ToField mismatch for {variant:?}");
    }
}

#[test]
fn test_option_security_type_to_field() {
    let some: Option<SecurityType> = Some(SecurityType::Option);
    assert_eq!(some.to_field(), "OPT");
    let none: Option<SecurityType> = None;
    assert_eq!(none.to_field(), "");
}

fn assert_cusip_bond(bond: Contract, cusip: &str) {
    assert_eq!(bond.symbol, Symbol::from(cusip));
    assert_eq!(bond.security_type, SecurityType::Bond);
    assert_eq!(bond.security_id_type, Some(SecurityIdType::Cusip));
    assert_eq!(bond.security_id, cusip);
    assert_eq!(bond.exchange, Exchange::from("SMART"));
    assert_eq!(bond.currency, Currency::from("USD"));
}

fn assert_isin_currency_mapping(make_bond: impl Fn(&str) -> Contract) {
    let cases = [
        ("US0378331005", "USD"),
        ("CA1234567890", "USD"),
        ("GB0002374006", "GBP"),
        ("JP3902900004", "JPY"),
        ("CH0012032048", "CHF"),
        ("AU000000BHP4", "AUD"),
        ("DE0001102309", "EUR"),
        ("FR0000131104", "EUR"),
        ("IT0001234567", "EUR"),
        ("ES0123456789", "EUR"),
        ("NL0011794037", "EUR"),
        ("BE0974293251", "EUR"),
        ("XX0000000000", "USD"),
        ("X", "USD"),
    ];
    for (isin, expected_currency) in cases {
        let bond = make_bond(isin);
        assert_eq!(bond.symbol, Symbol::from(isin), "symbol for {isin}");
        assert_eq!(bond.security_type, SecurityType::Bond);
        assert_eq!(bond.security_id_type, Some(SecurityIdType::Isin));
        assert_eq!(bond.security_id, isin);
        assert_eq!(bond.exchange, Exchange::from("SMART"));
        assert_eq!(bond.currency, Currency::from(expected_currency), "currency for {isin}");
    }
}

#[test]
fn test_bond_cusip_constructor() {
    assert_cusip_bond(Contract::bond_cusip("912810RN0"), "912810RN0");
}

#[test]
fn test_bond_constructor_cusip_variant() {
    assert_cusip_bond(Contract::bond(BondIdentifier::Cusip(Cusip::new("912810RN0"))), "912810RN0");
}

#[test]
fn test_bond_isin_constructor_currency_mapping() {
    assert_isin_currency_mapping(|isin| Contract::bond_isin(isin));
}

#[test]
fn test_bond_constructor_isin_currency_mapping() {
    assert_isin_currency_mapping(|isin| Contract::bond(BondIdentifier::Isin(Isin::new(isin))));
}

#[test]
fn test_combo_leg_open_close() {
    // Test From<i32> implementation
    assert_eq!(ComboLegOpenClose::from(0), ComboLegOpenClose::Same, "0 should be Same");
    assert_eq!(ComboLegOpenClose::from(1), ComboLegOpenClose::Open, "1 should be Open");
    assert_eq!(ComboLegOpenClose::from(2), ComboLegOpenClose::Close, "2 should be Close");
    assert_eq!(ComboLegOpenClose::from(3), ComboLegOpenClose::Unknown, "3 should be Unknown");

    // Test ToField implementation
    assert_eq!(ComboLegOpenClose::Same.to_field(), "0", "Same should be 0");
    assert_eq!(ComboLegOpenClose::Open.to_field(), "1", "Open should be 1");
    assert_eq!(ComboLegOpenClose::Close.to_field(), "2", "Close should be 2");
    assert_eq!(ComboLegOpenClose::Unknown.to_field(), "3", "Unknown should be 3");

    // Test Default implementation
    assert_eq!(ComboLegOpenClose::default(), ComboLegOpenClose::Same, "Default should be Same");
}

#[test]
fn test_combo_leg_open_close_unknown_value() {
    // Values outside the documented 0..=3 range decode to Unknown rather than panicking
    assert_eq!(ComboLegOpenClose::from(4), ComboLegOpenClose::Unknown);
    assert_eq!(ComboLegOpenClose::from(-1), ComboLegOpenClose::Unknown);
}

#[test]
fn test_tag_value_to_field() {
    // Test with multiple TagValue items
    let tag_values = vec![
        TagValue {
            tag: "TAG1".to_string(),
            value: "VALUE1".to_string(),
        },
        TagValue {
            tag: "TAG2".to_string(),
            value: "VALUE2".to_string(),
        },
        TagValue {
            tag: "TAG3".to_string(),
            value: "VALUE3".to_string(),
        },
    ];

    assert_eq!(
        tag_values.to_field(),
        "TAG1=VALUE1;TAG2=VALUE2;TAG3=VALUE3;",
        "Tag values should be formatted as TAG=VALUE; pairs"
    );

    // Test with a single TagValue
    let single_tag_value = vec![TagValue {
        tag: "SINGLE_TAG".to_string(),
        value: "SINGLE_VALUE".to_string(),
    }];

    assert_eq!(
        single_tag_value.to_field(),
        "SINGLE_TAG=SINGLE_VALUE;",
        "Single tag value should be formatted as TAG=VALUE;"
    );

    // Test with empty vec
    let empty: Vec<TagValue> = vec![];
    assert_eq!(empty.to_field(), "", "Empty vec should result in empty string");

    // Test with empty tag/value
    let empty_fields = vec![TagValue {
        tag: "".to_string(),
        value: "".to_string(),
    }];

    assert_eq!(empty_fields.to_field(), "=;", "Empty tag/value should be formatted as =;");
}

#[test]
fn test_is_bag() {
    // Test with a regular stock contract (not a bag/spread)
    let stock_contract = Contract::stock("AAPL").build();
    assert!(!stock_contract.is_bag(), "Stock contract should not be a bag");

    // Test with a regular option contract (not a bag/spread)
    let option_contract = Contract::option("AAPL", "20231215", 150.0, OptionRight::Call);
    assert!(!option_contract.is_bag(), "Option contract should not be a bag");

    // Test with a futures contract (not a bag/spread)
    // Using the simple factory method for futures that requires adding expiry
    let futures_contract = Contract {
        symbol: Symbol::from("ES"),
        security_type: SecurityType::Future,
        ..Default::default()
    };
    assert!(!futures_contract.is_bag(), "Futures contract should not be a bag");

    // Test with a contract that is a bag/spread
    let spread_contract = Contract {
        security_type: SecurityType::Spread,
        ..Default::default()
    };
    assert!(spread_contract.is_bag(), "Spread contract should be a bag");

    // Test with an explicitly set BAG security type
    let bag_contract = Contract {
        security_type: "BAG".parse().unwrap(),
        ..Default::default()
    };
    assert!(bag_contract.is_bag(), "BAG contract should be a bag");

    // Test with combo legs
    let combo_contract = Contract {
        security_type: SecurityType::Spread,
        combo_legs: vec![
            ComboLeg {
                contract_id: 12345,
                ratio: 1,
                action: LegAction::Buy,
                exchange: "SMART".to_string(),
                ..Default::default()
            },
            ComboLeg {
                contract_id: 67890,
                ratio: 1,
                action: LegAction::Sell,
                exchange: "SMART".to_string(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert!(combo_contract.is_bag(), "Contract with combo legs should be a bag");
}
