use super::*;

#[test]
fn missing_marker_is_copyable_and_debug() {
    let m = Missing;
    let copy = m;
    let _ = copy;
    assert_eq!(format!("{:?}", Missing), "Missing");
}

#[test]
fn validation_error_display() {
    let cases = [
        (ValidationError::InvalidQuantity(0.0), "Invalid quantity: 0"),
        (ValidationError::InvalidPrice(-10.0), "Invalid price: -10"),
        (ValidationError::InvalidCombination("test".to_string()), "Invalid combination: test"),
        (
            ValidationError::InvalidStopPrice { stop: 100.0, current: 95.0 },
            "Invalid stop price 100 for current price 95",
        ),
        (
            ValidationError::InvalidLimitPrice { limit: 90.0, current: 95.0 },
            "Invalid limit price 90 for current price 95",
        ),
        (ValidationError::InvalidBracketOrder("test".to_string()), "Invalid bracket order: test"),
        (
            ValidationError::InvalidPercentage {
                field: "max_pct_vol",
                value: 0.05,
                min: 0.1,
                max: 0.5,
            },
            "Invalid max_pct_vol: 0.05 (must be between 0.1 and 0.5)",
        ),
        (
            ValidationError::InvalidField {
                field: "Order::ext_operator",
                value: "probe-op".to_string(),
                reason: "TWS accepts 1-16 ASCII letters or digits".to_string(),
            },
            "Invalid Order::ext_operator \"probe-op\": TWS accepts 1-16 ASCII letters or digits",
        ),
        (
            ValidationError::InvalidDate {
                value: "20261340".to_string(),
                expected: "YYYYMMDD",
            },
            "Invalid date \"20261340\": expected YYYYMMDD",
        ),
    ];
    for (err, expected) in cases {
        assert_eq!(err.to_string(), expected);
    }
}
