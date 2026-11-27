use super::*;
use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
use crate::orders::{Order, ShortSaleSlot};

#[test]
fn encode_contract_sends_expiry_as_wire_text() {
    use crate::contracts::Expiry;
    // Unknown (a bond's zoned value) goes back byte for byte.
    for raw in ["20261218", "202612", "20430504 16:00 Europe/London"] {
        let contract = Contract {
            last_trade_date_or_contract_month: Expiry::from_wire(raw),
            ..Default::default()
        };
        assert_eq!(encode_contract(&contract).last_trade_date_or_contract_month.as_deref(), Some(raw));
    }
    // A hand-built empty Unknown is absent on the wire, as an unset field is.
    for expiry in [None, Some(Expiry::Unknown(String::new()))] {
        let contract = Contract {
            last_trade_date_or_contract_month: expiry,
            ..Default::default()
        };
        assert_eq!(encode_contract(&contract).last_trade_date_or_contract_month, None);
    }
}

#[test]
fn encode_contract_omits_unset_or_empty_sec_type() {
    for security_type in [None, Some(SecurityType::Other(String::new()))] {
        let contract = Contract {
            security_type,
            ..Default::default()
        };
        assert_eq!(encode_contract(&contract).sec_type, None);
    }
}

#[test]
fn encode_contract_basic() {
    let contract = Contract {
        contract_id: 265598,
        symbol: Symbol::from("AAPL"),
        security_type: Some(SecurityType::Stock),
        exchange: Exchange::from("SMART"),
        primary_exchange: Exchange::from("NASDAQ"),
        currency: Currency::from("USD"),
        ..Default::default()
    };

    let proto = encode_contract(&contract);

    assert_eq!(proto.con_id, Some(265598));
    assert_eq!(proto.symbol.as_deref(), Some("AAPL"));
    assert_eq!(proto.sec_type.as_deref(), Some("STK"));
    assert_eq!(proto.exchange.as_deref(), Some("SMART"));
    assert_eq!(proto.primary_exch.as_deref(), Some("NASDAQ"));
    assert_eq!(proto.currency.as_deref(), Some("USD"));
    assert!(proto.last_trade_date_or_contract_month.is_none());
    assert!(proto.strike.is_none());
    assert!(proto.multiplier.is_none());
}

#[test]
fn encode_contract_with_multiplier() {
    let contract = Contract {
        multiplier: "100".to_string(),
        ..Default::default()
    };

    let proto = encode_contract(&contract);
    assert_eq!(proto.multiplier, Some(100.0));
}

#[test]
fn encode_contract_empty_multiplier() {
    let contract = Contract::default();
    let proto = encode_contract(&contract);
    assert!(proto.multiplier.is_none());
}

#[test]
fn encode_delta_neutral() {
    let dnc = contracts::DeltaNeutralContract {
        contract_id: 123,
        delta: 0.5,
        price: 45.0,
    };
    let contract = Contract {
        delta_neutral_contract: Some(dnc),
        ..Default::default()
    };

    let proto = encode_contract(&contract);
    let dnc_proto = proto.delta_neutral_contract.unwrap();
    assert_eq!(dnc_proto.con_id, Some(123));
    assert_eq!(dnc_proto.delta, Some(0.5));
    assert_eq!(dnc_proto.price, Some(45.0));
}

#[test]
fn some_str_empty() {
    assert!(some_str("").is_none());
    assert_eq!(some_str("hello"), Some("hello".to_string()));
}

#[test]
fn some_display_works() {
    assert!(some_display::<i32>(None).is_none());
    assert_eq!(some_display(Some(&42_i32)), Some("42".to_string()));
    assert_eq!(some_display(Some(&"text".to_string())), Some("text".to_string()));
}

#[test]
fn tag_values_to_map_works() {
    let tags = vec![
        crate::contracts::TagValue {
            tag: "k1".to_string(),
            value: "v1".to_string(),
        },
        crate::contracts::TagValue {
            tag: "k2".to_string(),
            value: "v2".to_string(),
        },
    ];
    let map = tag_values_to_map(&tags);
    assert_eq!(map.get("k1").unwrap(), "v1");
    assert_eq!(map.get("k2").unwrap(), "v2");
}

#[test]
fn encode_combo_legs_pairs_each_leg_with_its_order_price() {
    use crate::contracts::{ComboLeg, ComboLegOpenClose, LegAction};
    use crate::orders::OrderComboLeg;

    let contract = Contract {
        combo_legs: vec![
            ComboLeg {
                contract_id: 1001,
                ratio: 1,
                action: LegAction::Buy,
                exchange: "SMART".into(),
                open_close: ComboLegOpenClose::Open,
                short_sale_slot: ShortSaleSlot::ThirdParty,
                designated_location: "LOC".into(),
                exempt_code: 7,
            },
            ComboLeg {
                contract_id: 1002,
                ratio: 2,
                action: LegAction::SellShort,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    // Prices cover only the first leg: the second must go out without one.
    let order = Order {
        order_combo_legs: vec![OrderComboLeg { price: Some(1.25) }],
        ..Default::default()
    };

    let legs = encode_combo_legs(&contract, Some(&order));

    assert_eq!(
        legs,
        vec![
            proto::ComboLeg {
                con_id: Some(1001),
                ratio: Some(1),
                action: Some("BUY".into()),
                exchange: Some("SMART".into()),
                open_close: Some(1),
                short_sales_slot: Some(2),
                designated_location: Some("LOC".into()),
                exempt_code: Some(7),
                per_leg_price: Some(1.25),
            },
            proto::ComboLeg {
                con_id: Some(1002),
                ratio: Some(2),
                action: Some("SSHORT".into()),
                exchange: None,
                open_close: None,
                short_sales_slot: None,
                designated_location: None,
                exempt_code: None,
                per_leg_price: None,
            },
        ]
    );

    let without_order = encode_combo_legs(&contract, None);
    assert!(without_order.iter().all(|leg| leg.per_leg_price.is_none()), "no order, no per-leg price");
}

#[test]
fn encode_combo_legs_sends_unknown_open_close_code_unchanged() {
    use crate::contracts::{ComboLeg, ComboLegOpenClose};

    let contract = Contract {
        combo_legs: vec![ComboLeg {
            open_close: ComboLegOpenClose::Unknown(7),
            ..Default::default()
        }],
        ..Default::default()
    };

    let legs = encode_combo_legs(&contract, None);

    assert_eq!(legs[0].open_close, Some(7));
}

#[test]
fn encode_combo_legs_short_sale_slot() {
    use crate::contracts::ComboLeg;

    let cases = [
        (ShortSaleSlot::None, None),
        (ShortSaleSlot::Broker, Some(1)),
        (ShortSaleSlot::ThirdParty, Some(2)),
        (ShortSaleSlot::Unknown(7), Some(7)),
    ];
    for (short_sale_slot, expected) in cases {
        let contract = Contract {
            combo_legs: vec![ComboLeg {
                short_sale_slot,
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(encode_combo_legs(&contract, None)[0].short_sales_slot, expected, "{short_sale_slot:?}");
    }
}

#[test]
fn encode_combo_legs_is_empty_for_a_plain_contract() {
    assert!(encode_combo_legs(&Contract::default(), Some(&Order::default())).is_empty());
}
