use super::*;
use crate::orders::Action;

#[test]
fn test_valid_buy_bracket() {
    let result = validate_bracket_prices(
        Some(&Action::Buy),
        50.0, // entry
        55.0, // take profit (above entry)
        45.0, // stop loss (below entry)
    );
    assert!(result.is_ok());
}

#[test]
fn test_valid_sell_bracket() {
    let result = validate_bracket_prices(
        Some(&Action::Sell),
        50.0, // entry
        45.0, // take profit (below entry)
        55.0, // stop loss (above entry)
    );
    assert!(result.is_ok());
}

#[test]
fn test_invalid_buy_bracket_take_profit() {
    let result = validate_bracket_prices(
        Some(&Action::Buy),
        50.0, // entry
        45.0, // take profit (BELOW entry - invalid)
        45.0, // stop loss
    );
    assert!(result.is_err());
}

#[test]
fn test_invalid_buy_bracket_stop_loss() {
    let result = validate_bracket_prices(
        Some(&Action::Buy),
        50.0, // entry
        55.0, // take profit
        55.0, // stop loss (ABOVE entry - invalid)
    );
    assert!(result.is_err());
}

#[test]
fn test_invalid_sell_bracket() {
    let result = validate_bracket_prices(
        Some(&Action::Sell),
        50.0, // entry
        55.0, // take profit (ABOVE entry - invalid for sell)
        45.0, // stop loss (BELOW entry - invalid for sell)
    );
    assert!(result.is_err());
}

#[test]
fn test_missing_action_validation() {
    let result = validate_bracket_prices(None, 50.0, 55.0, 45.0);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ValidationError::MissingRequiredField("action")));
}

#[test]
fn test_sell_short_bracket() {
    let result = validate_bracket_prices(
        Some(&Action::SellShort),
        50.0, // entry
        45.0, // take profit (below entry)
        55.0, // stop loss (above entry)
    );
    assert!(result.is_ok());
}

#[test]
fn test_edge_case_equal_prices() {
    // Test when take profit equals entry (invalid)
    let result = validate_bracket_prices(Some(&Action::Buy), 50.0, 50.0, 45.0);
    assert!(result.is_err());

    // Test when stop loss equals entry (invalid)
    let result = validate_bracket_prices(Some(&Action::Buy), 50.0, 55.0, 50.0);
    assert!(result.is_err());
}

#[test]
fn test_bracket_validation_error_messages() {
    let result = validate_bracket_prices(Some(&Action::Buy), 50.0, 45.0, 45.0);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Take profit (45) must be above entry (50)"));

    let result = validate_bracket_prices(Some(&Action::Sell), 50.0, 55.0, 55.0);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Take profit (55) must be below entry (50)"));
}
