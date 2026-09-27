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

#[cfg(test)]
mod tests;
