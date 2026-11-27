//! Types shared by the crate's builders and input constructors: the typestate
//! [`Missing`] marker and [`ValidationError`].
//!
//! A typestate builder carries a required field as a type parameter that starts
//! as [`Missing`] and becomes the field's value once it is set. The terminal
//! (`build()`) exists only on the complete state, so a builder missing a required
//! field fails to compile instead of failing at runtime.
//!
//! A value that can only be checked at runtime fails with a [`ValidationError`]
//! when it is data a caller may correct: a quantity, a price or strike, a date
//! parsed from user or file input, a field on a caller-built `Order`.

use std::fmt;

/// Marker for a required field a typestate builder has not been given yet.
#[derive(Debug, Clone, Copy)]
pub struct Missing;

/// A runtime input the client rejected before sending anything.
///
/// Returned on its own by input constructors (`Quantity::new`, `ExpirationDate::new`,
/// `"20261218".parse::<ExpirationDate>()`, `OrderBuilder::build`) and inside
/// [`Error::Validation`](crate::Error::Validation) by client methods. `?` converts
/// one into the other.
///
/// # Examples
///
/// ```
/// use ibapi::builder::ValidationError;
/// use ibapi::contracts::ExpirationDate;
///
/// match "20261340".parse::<ExpirationDate>() {
///     Err(ValidationError::InvalidDate { value, .. }) => assert_eq!(value, "20261340"),
///     other => panic!("unexpected {other:?}"),
/// }
/// ```
///
/// The enum is `#[non_exhaustive]`: the crate adds a variant for each new kind of check, so a
/// `match` needs a wildcard arm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ValidationError {
    /// Quantity must be positive and finite.
    InvalidQuantity(f64),
    /// Price must be finite (not NaN or infinity); a strike must also be positive.
    InvalidPrice(f64),
    /// Required builder field was not supplied.
    MissingRequiredField(&'static str),
    /// Combination of inputs violates broker rules.
    InvalidCombination(String),
    /// Stop price conflicts with current market context.
    InvalidStopPrice {
        /// Stop trigger price supplied by caller.
        stop: f64,
        /// Reference market price used for validation.
        current: f64,
    },
    /// Limit price conflicts with current market context.
    InvalidLimitPrice {
        /// Limit price supplied by caller.
        limit: f64,
        /// Reference market price used for validation.
        current: f64,
    },
    /// Bracket order configuration is invalid.
    InvalidBracketOrder(String),
    /// Percentage value outside allowed range (10-50%).
    InvalidPercentage {
        /// The field name (e.g., "max_pct_vol", "pct_vol").
        field: &'static str,
        /// The invalid value provided.
        value: f64,
        /// Minimum allowed value.
        min: f64,
        /// Maximum allowed value.
        max: f64,
    },
    /// A field on a caller-built value holds something TWS rejects, such as
    /// `Order::ext_operator`.
    InvalidField {
        /// The field, as `Type::field`, or the type name when a whole value was parsed from text
        /// (`"Duration"`).
        field: &'static str,
        /// The rejected value.
        value: String,
        /// What TWS accepts.
        reason: String,
    },
    /// Not a real date or contract month in the expected form.
    InvalidDate {
        /// The rejected input: the text parsed, or the components as `YYYY-MM-DD` / `YYYY-MM`.
        value: String,
        /// The form or constraint expected, such as `"YYYYMMDD"`.
        expected: &'static str,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuantity(q) => write!(f, "Invalid quantity: {}", q),
            Self::InvalidPrice(p) => write!(f, "Invalid price: {}", p),
            Self::MissingRequiredField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidCombination(msg) => write!(f, "Invalid combination: {}", msg),
            Self::InvalidStopPrice { stop, current } => {
                write!(f, "Invalid stop price {} for current price {}", stop, current)
            }
            Self::InvalidLimitPrice { limit, current } => {
                write!(f, "Invalid limit price {} for current price {}", limit, current)
            }
            Self::InvalidBracketOrder(msg) => write!(f, "Invalid bracket order: {}", msg),
            Self::InvalidPercentage { field, value, min, max } => {
                write!(f, "Invalid {}: {} (must be between {} and {})", field, value, min, max)
            }
            Self::InvalidField { field, value, reason } => write!(f, "Invalid {field} {value:?}: {reason}"),
            Self::InvalidDate { value, expected } => write!(f, "Invalid date {value:?}: expected {expected}"),
        }
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
