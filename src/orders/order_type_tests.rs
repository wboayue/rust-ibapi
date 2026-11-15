use super::*;

#[test]
fn order_type() {
    assert_eq!(OrderType::Market.as_str(), "MKT");
    assert_eq!(OrderType::Limit.as_str(), "LMT");
    assert_eq!(OrderType::Stop.as_str(), "STP");
    assert_eq!(OrderType::StopLimit.as_str(), "STP LMT");

    assert!(OrderType::Limit.requires_limit_price());
    assert!(!OrderType::Market.requires_limit_price());

    assert!(OrderType::Stop.requires_aux_price());
    assert!(!OrderType::Limit.requires_aux_price());
}
