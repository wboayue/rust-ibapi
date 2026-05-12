//! Strong types for contract building with validation.

use crate::ToField;
use std::fmt;
use time::{Date, Duration, Month, OffsetDateTime, Weekday};

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;

/// Mirrors std `String`'s `PartialEq` ergonomics on a string-newtype:
/// `wrapper == "literal"` and `"literal" == wrapper` both work.
macro_rules! impl_str_partial_eq {
    ($t:ty) => {
        impl PartialEq<str> for $t {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }
        impl PartialEq<&str> for $t {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }
        impl PartialEq<$t> for str {
            fn eq(&self, other: &$t) -> bool {
                self == other.0
            }
        }
        impl PartialEq<$t> for &str {
            fn eq(&self, other: &$t) -> bool {
                *self == other.0
            }
        }
    };
}

/// Generate `Display` / `FromStr<Err = Error>` / `ToField` impls from
/// hand-written `as_str(&self) -> &'static str` + `from_wire(&str) -> Option<Self>`
/// methods. The data tables stay in normal Rust (visible to goto-def); only
/// the boilerplate plumbing — `Display` via `as_str`, `FromStr` via `from_wire`
/// with canonical `Error::Parse`, `ToField` via `Display` — runs through the
/// macro. Orphan rule blocks a blanket `impl<T: WireEnum> Display`, so a
/// macro is the only viable shape.
macro_rules! impl_wire_enum {
    ($name:ident) => {
        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
        impl ::std::str::FromStr for $name {
            type Err = $crate::Error;
            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                Self::from_wire(s).ok_or_else(|| $crate::Error::Parse(0, s.to_string(), concat!("unknown ", stringify!($name)).into()))
            }
        }
        impl $crate::ToField for $name {
            fn to_field(&self) -> String {
                self.to_string()
            }
        }
    };
}

/// Strong type for trading symbols
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Symbol(pub String);

impl Symbol {
    /// Create a symbol from any string-like input.
    pub fn new(s: impl Into<String>) -> Self {
        Symbol(s.into())
    }

    /// Return the raw symbol text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Symbol {
    fn from(s: &str) -> Self {
        Symbol(s.to_string())
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Self {
        Symbol(s)
    }
}

impl From<&String> for Symbol {
    fn from(s: &String) -> Self {
        Symbol(s.clone())
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ToField for Symbol {
    fn to_field(&self) -> String {
        self.0.clone()
    }
}

impl_str_partial_eq!(Symbol);

/// Exchange identifier
///
/// IBKR supports 160+ exchanges worldwide. This type provides a lightweight wrapper
/// around exchange codes.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Exchange(pub String);

impl Exchange {
    /// Create a new exchange
    pub fn new(s: impl Into<String>) -> Self {
        Exchange(s.into())
    }

    /// Get the exchange code as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if the exchange string is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for Exchange {
    fn default() -> Self {
        Exchange("SMART".to_string())
    }
}

impl From<&str> for Exchange {
    fn from(s: &str) -> Self {
        Exchange(s.to_string())
    }
}

impl From<String> for Exchange {
    fn from(s: String) -> Self {
        Exchange(s)
    }
}

impl From<&String> for Exchange {
    fn from(s: &String) -> Self {
        Exchange(s.clone())
    }
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ToField for Exchange {
    fn to_field(&self) -> String {
        self.0.clone()
    }
}

impl_str_partial_eq!(Exchange);

/// Currency identifier
///
/// IBKR supports trading in many currencies worldwide. This type provides a lightweight
/// wrapper around currency codes.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Currency(pub String);

impl Currency {
    /// Create a new currency
    pub fn new(s: impl Into<String>) -> Self {
        Currency(s.into())
    }

    /// Get the currency code as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Currency {
    fn default() -> Self {
        Currency("USD".to_string())
    }
}

impl From<&str> for Currency {
    fn from(s: &str) -> Self {
        Currency(s.to_string())
    }
}

impl From<String> for Currency {
    fn from(s: String) -> Self {
        Currency(s)
    }
}

impl From<&String> for Currency {
    fn from(s: &String) -> Self {
        Currency(s.clone())
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ToField for Currency {
    fn to_field(&self) -> String {
        self.0.clone()
    }
}

impl_str_partial_eq!(Currency);

/// Option right (Call or Put). Matches IBKR's wire vocabulary `"C"` / `"P"`.
///
/// No `Default` — `Contract.right: Option<OptionRight>` carries the no-right
/// state via `None`.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum OptionRight {
    /// Call option right.
    Call,
    /// Put option right.
    Put,
}

impl OptionRight {
    /// Return the canonical single-character wire string (`"C"` or `"P"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            OptionRight::Call => "C",
            OptionRight::Put => "P",
        }
    }

    fn from_wire(s: &str) -> Option<Self> {
        match s {
            "C" => Some(Self::Call),
            "P" => Some(Self::Put),
            _ => None,
        }
    }
}

impl_wire_enum!(OptionRight);

/// Validated strike price (must be positive)
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strike(f64);

impl Strike {
    /// Construct a validated strike price ensuring it is positive.
    pub fn new(price: f64) -> Result<Self, String> {
        if price <= 0.0 {
            Err("Strike price must be positive".to_string())
        } else {
            Ok(Strike(price))
        }
    }

    /// Create a strike price, panicking if invalid (for internal use in builders)
    pub(crate) fn new_unchecked(price: f64) -> Self {
        Strike::new(price).expect("Strike price must be positive")
    }

    /// Access the numeric strike value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Date for option expiration
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpirationDate {
    year: u16,
    month: u8,
    day: u8,
}

impl ExpirationDate {
    /// Create an option expiration date from year/month/day components.
    pub fn new(year: u16, month: u8, day: u8) -> Self {
        ExpirationDate { year, month, day }
    }

    /// Helper to calculate days until next Friday from a given weekday
    fn days_until_friday(from_weekday: Weekday) -> i64 {
        match from_weekday {
            Weekday::Saturday => 6,
            Weekday::Sunday => 5,
            Weekday::Monday => 4,
            Weekday::Tuesday => 3,
            Weekday::Wednesday => 2,
            Weekday::Thursday => 1,
            Weekday::Friday => 0,
        }
    }

    /// Get the next Friday from today
    pub fn next_friday() -> Self {
        Self::next_friday_from(OffsetDateTime::now_utc().date())
    }

    fn next_friday_from(today: Date) -> Self {
        let days_to_add = match today.weekday() {
            Weekday::Friday => 7,
            other => Self::days_until_friday(other),
        };
        let next_friday = today + Duration::days(days_to_add);

        ExpirationDate {
            year: next_friday.year() as u16,
            month: next_friday.month() as u8,
            day: next_friday.day(),
        }
    }

    /// Get the third Friday of the current month (standard monthly options expiration)
    pub fn third_friday_of_month() -> Self {
        Self::third_friday_from(OffsetDateTime::now_utc().date())
    }

    fn third_friday_from(today: Date) -> Self {
        let year = today.year();
        let month = today.month();

        // Find the first day of the month
        let first_of_month = Date::from_calendar_date(year, month, 1).expect("Valid date");

        // Find the first Friday, then add 14 days to get third Friday
        let days_to_first_friday = Self::days_until_friday(first_of_month.weekday());
        let third_friday = first_of_month + Duration::days(days_to_first_friday + 14);

        // If we've passed this month's third Friday, get next month's
        if today > third_friday {
            let next_month = if month == Month::December {
                Date::from_calendar_date(year + 1, Month::January, 1)
            } else {
                Date::from_calendar_date(year, month.next(), 1)
            }
            .expect("Valid date");

            let days_to_first_friday_next = Self::days_until_friday(next_month.weekday());
            let third_friday_next = next_month + Duration::days(days_to_first_friday_next + 14);

            ExpirationDate {
                year: third_friday_next.year() as u16,
                month: third_friday_next.month() as u8,
                day: third_friday_next.day(),
            }
        } else {
            ExpirationDate {
                year: third_friday.year() as u16,
                month: third_friday.month() as u8,
                day: third_friday.day(),
            }
        }
    }
}

impl fmt::Display for ExpirationDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}{:02}{:02}", self.year, self.month, self.day)
    }
}

/// Contract month for futures
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractMonth {
    year: u16,
    month: u8,
}

impl ContractMonth {
    /// Construct a futures contract month given year and month.
    pub fn new(year: u16, month: u8) -> Self {
        ContractMonth { year, month }
    }

    /// Get the front month contract (next expiring)
    pub fn front() -> Self {
        let now = OffsetDateTime::now_utc();
        Self::front_from(now.year() as u16, now.month() as u8, now.day())
    }

    fn front_from(current_year: u16, current_month: u8, current_day: u8) -> Self {
        // Futures typically expire around the third Friday of the month
        // If we're past the 15th, assume current month has expired
        if current_day > 15 {
            if current_month == 12 {
                ContractMonth::new(current_year + 1, 1)
            } else {
                ContractMonth::new(current_year, current_month + 1)
            }
        } else {
            ContractMonth::new(current_year, current_month)
        }
    }

    /// Get the next quarterly contract month (Mar, Jun, Sep, Dec)
    pub fn next_quarter() -> Self {
        let now = OffsetDateTime::now_utc();
        Self::next_quarter_from(now.year() as u16, now.month() as u8, now.day())
    }

    fn next_quarter_from(current_year: u16, current_month: u8, current_day: u8) -> Self {
        // Find next quarterly month
        let next_quarter_month = match current_month {
            1 | 2 => 3,
            3 if current_day > 15 => 6,
            3 => 3,
            4 | 5 => 6,
            6 if current_day > 15 => 9,
            6 => 6,
            7 | 8 => 9,
            9 if current_day > 15 => 12,
            9 => 9,
            10 | 11 => 12,
            12 if current_day > 15 => 3,
            12 => 12,
            _ => 3,
        };

        // Adjust year if we wrapped around
        let year = if current_month == 12 && next_quarter_month == 3 {
            current_year + 1
        } else {
            current_year
        };

        ContractMonth::new(year, next_quarter_month)
    }
}

impl fmt::Display for ContractMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}{:02}", self.year, self.month)
    }
}

/// CUSIP identifier
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cusip(pub String);

impl Cusip {
    /// Create a CUSIP identifier from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Cusip(s.into())
    }

    /// Return the underlying CUSIP text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Cusip {
    fn from(s: &str) -> Self {
        Cusip(s.to_string())
    }
}

impl From<String> for Cusip {
    fn from(s: String) -> Self {
        Cusip(s)
    }
}

impl From<&String> for Cusip {
    fn from(s: &String) -> Self {
        Cusip(s.clone())
    }
}

impl fmt::Display for Cusip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ISIN identifier
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Isin(pub String);

impl Isin {
    /// Create an ISIN identifier from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Isin(s.into())
    }

    /// Return the underlying ISIN text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Isin {
    fn from(s: &str) -> Self {
        Isin(s.to_string())
    }
}

impl From<String> for Isin {
    fn from(s: String) -> Self {
        Isin(s)
    }
}

impl From<&String> for Isin {
    fn from(s: &String) -> Self {
        Isin(s.clone())
    }
}

impl fmt::Display for Isin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Bond identifier type
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BondIdentifier {
    /// A bond identified by a CUSIP code.
    Cusip(Cusip),
    /// A bond identified by an ISIN code.
    Isin(Isin),
}

/// Trading action for spread/combo legs. Mirrors the IBKR wire vocabulary
/// `BUY` / `SELL` / `SSHORT`. `SLONG` is not accepted on combo legs — only the
/// SSHORT short-sale form is gated (`SSHORT_COMBO_LEGS = 35`, well below our
/// floor of 210), so all three variants are unconditionally valid.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum LegAction {
    /// Buy the leg.
    #[default]
    Buy,
    /// Sell the leg.
    Sell,
    /// Short-sell the leg.
    SellShort,
}

impl LegAction {
    /// Return the canonical IB wire string (`"BUY"` / `"SELL"` / `"SSHORT"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            LegAction::Buy => "BUY",
            LegAction::Sell => "SELL",
            LegAction::SellShort => "SSHORT",
        }
    }

    fn from_wire(s: &str) -> Option<Self> {
        match s {
            "BUY" => Some(Self::Buy),
            "SELL" => Some(Self::Sell),
            "SSHORT" => Some(Self::SellShort),
            _ => None,
        }
    }
}

impl_wire_enum!(LegAction);

/// Marker type for missing required fields in type-state builders
#[derive(Debug, Clone, Copy)]
pub struct Missing;
