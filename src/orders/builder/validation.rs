use super::types::ValidationError;
use crate::orders::Action;

/// Validates bracket order prices
pub fn validate_bracket_prices(action: Option<&Action>, entry: f64, take_profit: f64, stop_loss: f64) -> Result<(), ValidationError> {
    let action = action.ok_or(ValidationError::MissingRequiredField("action"))?;

    match action {
        Action::Buy => {
            if take_profit <= entry {
                return Err(ValidationError::InvalidBracketOrder(format!(
                    "Take profit ({}) must be above entry ({}) for buy orders",
                    take_profit, entry
                )));
            }
            if stop_loss >= entry {
                return Err(ValidationError::InvalidBracketOrder(format!(
                    "Stop loss ({}) must be below entry ({}) for buy orders",
                    stop_loss, entry
                )));
            }
        }
        Action::Sell | Action::SellShort => {
            if take_profit >= entry {
                return Err(ValidationError::InvalidBracketOrder(format!(
                    "Take profit ({}) must be below entry ({}) for sell orders",
                    take_profit, entry
                )));
            }
            if stop_loss <= entry {
                return Err(ValidationError::InvalidBracketOrder(format!(
                    "Stop loss ({}) must be above entry ({}) for sell orders",
                    stop_loss, entry
                )));
            }
        }
        _ => {}
    }

    Ok(())
}

/// Validates stop price relative to current market price
pub fn validate_stop_price(action: &Action, stop_price: f64, current_price: Option<f64>) -> Result<(), ValidationError> {
    if let Some(current) = current_price {
        match action {
            Action::Buy => {
                if stop_price <= current {
                    return Err(ValidationError::InvalidStopPrice { stop: stop_price, current });
                }
            }
            Action::Sell | Action::SellShort => {
                if stop_price >= current {
                    return Err(ValidationError::InvalidStopPrice { stop: stop_price, current });
                }
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_stop_price_validation_buy() {
        // Buy stop must be above current price
        assert!(validate_stop_price(&Action::Buy, 55.0, Some(50.0)).is_ok());
        assert!(validate_stop_price(&Action::Buy, 45.0, Some(50.0)).is_err());
    }

    #[test]
    fn test_stop_price_validation_sell() {
        // Sell stop must be below current price
        assert!(validate_stop_price(&Action::Sell, 45.0, Some(50.0)).is_ok());
        assert!(validate_stop_price(&Action::Sell, 55.0, Some(50.0)).is_err());
    }

    #[test]
    fn test_stop_price_no_current_price() {
        // Should pass if no current price provided
        assert!(validate_stop_price(&Action::Buy, 55.0, None).is_ok());
        assert!(validate_stop_price(&Action::Sell, 45.0, None).is_ok());
    }
}
