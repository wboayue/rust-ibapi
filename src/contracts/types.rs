//! Strong types for contract building with validation.

use std::fmt;
use time::{Date, Duration, Month, OffsetDateTime, Weekday};

/// Strong type for trading symbols
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol(pub String);

impl Symbol {
    pub fn new(s: impl Into<String>) -> Self {
        Symbol(s.into())
    }

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

/// Exchange identifier
///
/// IBKR supports 160+ exchanges worldwide. This type provides a lightweight wrapper
/// around exchange codes with constants for commonly used exchanges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exchange(pub &'static str);

impl Exchange {
    /// Create a new exchange from a static string
    pub const fn new(s: &'static str) -> Self {
        Exchange(s)
    }

    /// Get the exchange code as a string slice
    pub fn as_str(&self) -> &str {
        self.0
    }

    // Common exchange constants
    pub const SMART: Exchange = Exchange("SMART");
    pub const NASDAQ: Exchange = Exchange("NASDAQ");
    pub const NYSE: Exchange = Exchange("NYSE");
    pub const CBOE: Exchange = Exchange("CBOE");
    pub const GLOBEX: Exchange = Exchange("GLOBEX");
    pub const IDEALPRO: Exchange = Exchange("IDEALPRO");
    pub const PAXOS: Exchange = Exchange("PAXOS");
    pub const EUREX: Exchange = Exchange("EUREX");
    pub const LSE: Exchange = Exchange("LSE");
    pub const TSEJ: Exchange = Exchange("TSEJ");
    pub const ARCA: Exchange = Exchange("ARCA");
    pub const ISLAND: Exchange = Exchange("ISLAND");
    pub const CME: Exchange = Exchange("CME");
    pub const ICE: Exchange = Exchange("ICE");
    pub const BATS: Exchange = Exchange("BATS");
}

impl Default for Exchange {
    fn default() -> Self {
        Exchange::SMART
    }
}

impl From<&'static str> for Exchange {
    fn from(s: &'static str) -> Self {
        Exchange(s)
    }
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Currency identifier
///
/// IBKR supports trading in many currencies worldwide. This type provides a lightweight
/// wrapper around currency codes with constants for commonly used currencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Currency(pub &'static str);

impl Currency {
    /// Create a new currency from a static string
    pub const fn new(s: &'static str) -> Self {
        Currency(s)
    }

    /// Get the currency code as a string slice
    pub fn as_str(&self) -> &str {
        self.0
    }

    // Common currency constants
    pub const USD: Currency = Currency("USD");
    pub const EUR: Currency = Currency("EUR");
    pub const GBP: Currency = Currency("GBP");
    pub const JPY: Currency = Currency("JPY");
    pub const CHF: Currency = Currency("CHF");
    pub const CAD: Currency = Currency("CAD");
    pub const AUD: Currency = Currency("AUD");
    pub const NZD: Currency = Currency("NZD");
    pub const HKD: Currency = Currency("HKD");
    pub const SGD: Currency = Currency("SGD");
    pub const CNH: Currency = Currency("CNH"); // Offshore RMB
    pub const KRW: Currency = Currency("KRW");
    pub const SEK: Currency = Currency("SEK");
    pub const NOK: Currency = Currency("NOK");
    pub const DKK: Currency = Currency("DKK");
    pub const MXN: Currency = Currency("MXN");
    pub const INR: Currency = Currency("INR");
    pub const ZAR: Currency = Currency("ZAR");
}

impl Default for Currency {
    fn default() -> Self {
        Currency::USD
    }
}

impl From<&'static str> for Currency {
    fn from(s: &'static str) -> Self {
        Currency(s)
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Option right (Call or Put)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionRight {
    Call,
    Put,
}

impl OptionRight {
    pub fn as_str(&self) -> &str {
        match self {
            OptionRight::Call => "C",
            OptionRight::Put => "P",
        }
    }
}

impl fmt::Display for OptionRight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Validated strike price (must be positive)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strike(f64);

impl Strike {
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

    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Date for option expiration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpirationDate {
    year: u16,
    month: u8,
    day: u8,
}

impl ExpirationDate {
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
        let today = OffsetDateTime::now_utc().date();
        let days_to_add = match today.weekday() {
            Weekday::Friday => 7, // If today is Friday, get next Friday
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
        let now = OffsetDateTime::now_utc();
        let year = now.year();
        let month = now.month();

        // Find the first day of the month
        let first_of_month = Date::from_calendar_date(year, month, 1).expect("Valid date");

        // Find the first Friday, then add 14 days to get third Friday
        let days_to_first_friday = Self::days_until_friday(first_of_month.weekday());
        let third_friday = first_of_month + Duration::days(days_to_first_friday + 14);

        // If we've passed this month's third Friday, get next month's
        if now.date() > third_friday {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractMonth {
    year: u16,
    month: u8,
}

impl ContractMonth {
    pub fn new(year: u16, month: u8) -> Self {
        ContractMonth { year, month }
    }

    /// Get the front month contract (next expiring)
    pub fn front() -> Self {
        let now = OffsetDateTime::now_utc();
        let current_year = now.year() as u16;
        let current_month = now.month() as u8;
        let current_day = now.day();

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
        let current_year = now.year() as u16;
        let current_month = now.month() as u8;
        let current_day = now.day();

        // Find next quarterly month
        let next_quarter_month = match current_month {
            1 | 2 => 3,
            3 => {
                if current_day > 15 {
                    6
                } else {
                    3
                }
            }
            4 | 5 => 6,
            6 => {
                if current_day > 15 {
                    9
                } else {
                    6
                }
            }
            7 | 8 => 9,
            9 => {
                if current_day > 15 {
                    12
                } else {
                    9
                }
            }
            10 | 11 => 12,
            12 => {
                if current_day > 15 {
                    3
                } else {
                    12
                }
            }
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cusip(pub String);

impl Cusip {
    pub fn new(s: impl Into<String>) -> Self {
        Cusip(s.into())
    }

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Isin(pub String);

impl Isin {
    pub fn new(s: impl Into<String>) -> Self {
        Isin(s.into())
    }

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BondIdentifier {
    Cusip(Cusip),
    Isin(Isin),
}

/// Trading action for spread/combo legs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegAction {
    Buy,
    Sell,
}

impl LegAction {
    pub fn as_str(&self) -> &str {
        match self {
            LegAction::Buy => "BUY",
            LegAction::Sell => "SELL",
        }
    }
}

impl fmt::Display for LegAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Marker type for missing required fields in type-state builders
#[derive(Debug, Clone, Copy)]
pub struct Missing;
