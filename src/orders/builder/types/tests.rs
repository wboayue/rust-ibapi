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
fn test_order_type() {
    assert_eq!(OrderType::Market.as_str(), "MKT");
    assert_eq!(OrderType::Limit.as_str(), "LMT");
    assert_eq!(OrderType::Stop.as_str(), "STP");
    assert_eq!(OrderType::StopLimit.as_str(), "STP LMT");

    assert!(OrderType::Limit.requires_limit_price());
    assert!(!OrderType::Market.requires_limit_price());
}

#[test]
fn test_validation_error_display() {
    let err = ValidationError::InvalidQuantity(0.0);
    assert_eq!(err.to_string(), "Invalid quantity: 0");

    let err = ValidationError::InvalidPrice(-10.0);
    assert_eq!(err.to_string(), "Invalid price: -10");

    let err = ValidationError::MissingRequiredField("order_type");
    assert_eq!(err.to_string(), "Missing required field: order_type");

    let err = ValidationError::InvalidBracketOrder("test".to_string());
    assert_eq!(err.to_string(), "Invalid bracket order: test");
}
