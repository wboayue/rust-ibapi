use super::*;
use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
use crate::orders::{Action, Order, TimeInForce};

#[test]
fn test_encode_contract_basic() {
    let contract = Contract {
        contract_id: 265598,
        symbol: Symbol::from("AAPL"),
        security_type: SecurityType::Stock,
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
fn test_encode_contract_with_multiplier() {
    let contract = Contract {
        multiplier: "100".to_string(),
        ..Default::default()
    };

    let proto = encode_contract(&contract);
    assert_eq!(proto.multiplier, Some(100.0));
}

#[test]
fn test_encode_contract_empty_multiplier() {
    let contract = Contract::default();
    let proto = encode_contract(&contract);
    assert!(proto.multiplier.is_none());
}

#[test]
fn test_encode_delta_neutral() {
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
fn test_encode_order_basic() {
    let order = Order {
        action: Action::Buy,
        total_quantity: 100.0,
        order_type: "LMT".to_string(),
        limit_price: Some(150.0),
        tif: TimeInForce::Day,
        transmit: true,
        ..Default::default()
    };

    let proto = encode_order(&order);

    assert_eq!(proto.action.as_deref(), Some("BUY"));
    assert_eq!(proto.total_quantity.as_deref(), Some("100"));
    assert_eq!(proto.order_type.as_deref(), Some("LMT"));
    assert_eq!(proto.lmt_price, Some(150.0));
    assert_eq!(proto.tif.as_deref(), Some("DAY"));
    assert_eq!(proto.transmit, Some(true));
}

#[test]
fn test_encode_order_hedge_max_size() {
    let order = Order {
        hedge_max_size: Some(500),
        ..Default::default()
    };

    let proto = encode_order(&order);

    assert_eq!(proto.hedge_max_size, Some(500));
}

#[test]
fn test_encode_order_default_fields_omitted() {
    let order = Order::default();
    let proto = encode_order(&order);

    assert!(proto.client_id.is_none());
    assert!(proto.parent_id.is_none());
    assert!(proto.block_order.is_none());
    assert!(proto.hidden.is_none());
    assert!(proto.all_or_none.is_none());
    assert!(proto.hedge_max_size.is_none());
}

#[test]
fn test_encode_soft_dollar_tier_empty() {
    let tier = SoftDollarTier::default();
    assert!(encode_soft_dollar_tier(&tier).is_none());
}

#[test]
fn test_encode_soft_dollar_tier_filled() {
    let tier = SoftDollarTier {
        name: "Tier1".to_string(),
        value: "Val1".to_string(),
        display_name: "Display1".to_string(),
    };
    let proto = encode_soft_dollar_tier(&tier).unwrap();
    assert_eq!(proto.name.as_deref(), Some("Tier1"));
    assert_eq!(proto.value.as_deref(), Some("Val1"));
    assert_eq!(proto.display_name.as_deref(), Some("Display1"));
}

#[test]
fn test_encode_condition_price() {
    use crate::orders::conditions::{PriceCondition, TriggerMethod};
    let cond = OrderCondition::Price(PriceCondition {
        contract_id: 265598,
        exchange: "SMART".to_string(),
        price: 150.0,
        trigger_method: TriggerMethod::Last,
        is_more: true,
        is_conjunction: true,
    });

    let proto = encode_condition(&cond);
    assert_eq!(proto.r#type, Some(1));
    assert_eq!(proto.is_conjunction_connection, Some(true));
    assert_eq!(proto.is_more, Some(true));
    assert_eq!(proto.con_id, Some(265598));
    assert_eq!(proto.exchange.as_deref(), Some("SMART"));
    assert_eq!(proto.price, Some(150.0));
    assert_eq!(proto.trigger_method, Some(2)); // Last = 2
}

#[test]
fn test_encode_condition_price_default_trigger_method_is_emitted() {
    // TWS rejects ("Invalid value in field # 6127") if trigger_method is omitted.
    use crate::orders::conditions::{PriceCondition, TriggerMethod};
    let cond = OrderCondition::Price(PriceCondition {
        contract_id: 265598,
        exchange: "SMART".to_string(),
        price: 150.0,
        trigger_method: TriggerMethod::Default,
        is_more: true,
        is_conjunction: true,
    });

    let proto = encode_condition(&cond);
    assert_eq!(proto.trigger_method, Some(0), "Default trigger_method must be emitted, not omitted");
}

#[test]
fn test_encode_condition_time() {
    use crate::orders::conditions::TimeCondition;
    let cond = OrderCondition::Time(TimeCondition {
        time: "20251230 14:30:00 US/Eastern".to_string(),
        is_more: false,
        is_conjunction: false,
    });

    let proto = encode_condition(&cond);
    assert_eq!(proto.r#type, Some(3));
    assert_eq!(proto.is_conjunction_connection, Some(false));
    assert_eq!(proto.is_more, Some(false));
    assert_eq!(proto.time.as_deref(), Some("20251230 14:30:00 US/Eastern"));
}

#[test]
fn test_encode_execution_filter() {
    let filter = orders::ExecutionFilter {
        client_id: Some(1),
        account_code: "DU123".to_string(),
        time: "20240101 00:00:00".to_string(),
        symbol: "AAPL".to_string(),
        security_type: "STK".to_string(),
        exchange: "SMART".to_string(),
        side: Some(orders::ExecutionFilterSide::Buy),
        last_n_days: 5,
        specific_dates: vec!["20240101".to_string()],
    };

    let proto = encode_execution_filter(&filter);
    assert_eq!(proto.client_id, Some(1));
    assert_eq!(proto.acct_code.as_deref(), Some("DU123"));
    assert_eq!(proto.symbol.as_deref(), Some("AAPL"));
    assert_eq!(proto.side.as_deref(), Some("BUY"));
    assert_eq!(proto.last_n_days, Some(5));
    assert_eq!(proto.specific_dates, vec![20240101]);
}

#[test]
fn test_some_str_empty() {
    assert!(some_str("").is_none());
    assert_eq!(some_str("hello"), Some("hello".to_string()));
}

#[test]
fn test_some_display() {
    assert!(some_display::<i32>(None).is_none());
    assert_eq!(some_display(Some(&42_i32)), Some("42".to_string()));
    assert_eq!(some_display(Some(&"text".to_string())), Some("text".to_string()));
}

#[test]
fn test_tag_values_to_map() {
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
