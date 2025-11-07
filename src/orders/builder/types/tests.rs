use super::*;

#[test]
fn test_order_id() {
    let id = OrderId::new(100);
    assert_eq!(id.value(), 100);
    assert_eq!(format!("{}", id), "100");

    let id2: OrderId = 200.into();
    assert_eq!(id2.value(), 200);

    let val: i32 = id.into();
    assert_eq!(val, 100);
}

#[test]
fn test_bracket_order_ids() {
    let ids = BracketOrderIds::new(100, 101, 102);
    assert_eq!(ids.parent.value(), 100);
    assert_eq!(ids.take_profit.value(), 101);
    assert_eq!(ids.stop_loss.value(), 102);

    let vec = ids.as_vec();
    assert_eq!(vec.len(), 3);
    assert_eq!(vec[0].value(), 100);

    let i32_vec = ids.as_i32_vec();
    assert_eq!(i32_vec, vec![100, 101, 102]);

    let ids2 = BracketOrderIds::from(vec![200, 201, 202]);
    assert_eq!(ids2.parent.value(), 200);

    let ids3 = BracketOrderIds::from([300, 301, 302]);
    assert_eq!(ids3.parent.value(), 300);
}

#[test]
fn test_quantity_validation() {
    assert!(Quantity::new(100.0).is_ok());
    assert!(Quantity::new(0.0).is_err());
    assert!(Quantity::new(-10.0).is_err());
    assert!(Quantity::new(f64::NAN).is_err());
    assert!(Quantity::new(f64::INFINITY).is_err());
}

#[test]
fn test_price_validation() {
    assert!(Price::new(50.0).is_ok());
    assert!(Price::new(0.0).is_ok());
    assert!(Price::new(-10.0).is_ok());
    assert!(Price::new(f64::NAN).is_err());
    assert!(Price::new(f64::INFINITY).is_err());
}

#[test]
fn test_time_in_force() {
    assert_eq!(TimeInForce::Day.as_str(), "DAY");
    assert_eq!(TimeInForce::GoodTillCancel.as_str(), "GTC");
    assert_eq!(TimeInForce::ImmediateOrCancel.as_str(), "IOC");
    assert_eq!(
        TimeInForce::GoodTillDate {
            date: "20240101".to_string()
        }
        .as_str(),
        "GTD"
    );
}

#[test]
fn test_order_type() {
    assert_eq!(OrderType::Market.as_str(), "MKT");
    assert_eq!(OrderType::Limit.as_str(), "LMT");
    assert_eq!(OrderType::Stop.as_str(), "STP");
    assert_eq!(OrderType::StopLimit.as_str(), "STP LMT");

    assert!(OrderType::Limit.requires_limit_price());
    assert!(!OrderType::Market.requires_limit_price());

    assert!(OrderType::Stop.requires_aux_price());
    assert!(!OrderType::Limit.requires_aux_price());
}

#[test]
fn test_auction_type() {
    assert_eq!(AuctionType::Opening.to_strategy(), 1);
    assert_eq!(AuctionType::Closing.to_strategy(), 2);
    assert_eq!(AuctionType::Volatility.to_strategy(), 4);
}

#[test]
fn test_validation_error_display() {
    let err = ValidationError::InvalidQuantity(0.0);
    assert_eq!(err.to_string(), "Invalid quantity: 0");

    let err = ValidationError::InvalidPrice(-10.0);
    assert_eq!(err.to_string(), "Invalid price: -10");

    let err = ValidationError::MissingRequiredField("order_type");
    assert_eq!(err.to_string(), "Missing required field: order_type");

    let err = ValidationError::InvalidCombination("test".to_string());
    assert_eq!(err.to_string(), "Invalid combination: test");

    let err = ValidationError::InvalidStopPrice { stop: 100.0, current: 95.0 };
    assert_eq!(err.to_string(), "Invalid stop price 100 for current price 95");

    let err = ValidationError::InvalidLimitPrice { limit: 90.0, current: 95.0 };
    assert_eq!(err.to_string(), "Invalid limit price 90 for current price 95");

    let err = ValidationError::InvalidBracketOrder("test".to_string());
    assert_eq!(err.to_string(), "Invalid bracket order: test");
}

#[test]
#[should_panic]
fn test_bracket_order_ids_wrong_length() {
    let _ = BracketOrderIds::from(vec![100, 101]); // Should panic with wrong length
}

#[test]
fn test_order_analysis_default() {
    let analysis = OrderAnalysis {
        initial_margin: Some(1000.0),
        maintenance_margin: Some(800.0),
        commission: Some(1.5),
        commission_currency: "USD".to_string(),
        warning_text: String::new(),
    };

    assert_eq!(analysis.initial_margin, Some(1000.0));
    assert_eq!(analysis.maintenance_margin, Some(800.0));
    assert_eq!(analysis.commission, Some(1.5));
    assert_eq!(analysis.commission_currency, "USD");
    assert!(analysis.warning_text.is_empty());
}
