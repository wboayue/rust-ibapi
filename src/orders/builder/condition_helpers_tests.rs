use super::*;

#[test]
fn test_price_helper() {
    let condition = price(12345, "NASDAQ").greater_than(150.0).build();
    assert_eq!(condition.contract_id, 12345);
    assert_eq!(condition.exchange, "NASDAQ");
    assert_eq!(condition.price, 150.0);
    assert!(condition.is_more);
}

#[test]
fn test_time_helper() {
    let condition = time().greater_than("20251230 14:30:00 UTC").build();
    assert_eq!(condition.time, "20251230 14:30:00 UTC");
    assert!(condition.is_more);
}

#[test]
fn test_margin_helper() {
    let condition = margin().less_than(30).build();
    assert_eq!(condition.percent, 30);
    assert!(!condition.is_more);
}

#[test]
fn test_volume_helper() {
    let condition = volume(12345, "SMART").greater_than(1000000).build();
    assert_eq!(condition.contract_id, 12345);
    assert_eq!(condition.exchange, "SMART");
    assert_eq!(condition.volume, 1000000);
    assert!(condition.is_more);
}

#[test]
fn test_execution_helper() {
    let condition = execution("AAPL", "STK", "NASDAQ");
    match condition {
        OrderCondition::Execution(exec) => {
            assert_eq!(exec.symbol, "AAPL");
            assert_eq!(exec.security_type, "STK");
            assert_eq!(exec.exchange, "NASDAQ");
            assert!(exec.is_conjunction);
        }
        _ => panic!("Expected Execution condition"),
    }
}

#[test]
fn test_percent_change_helper() {
    let condition = percent_change(12345, "SMART").greater_than(5.0).build();
    assert_eq!(condition.contract_id, 12345);
    assert_eq!(condition.exchange, "SMART");
    assert_eq!(condition.percent, 5.0);
    assert!(condition.is_more);
}
