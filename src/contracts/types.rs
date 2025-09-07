//! Strong types for contract building with validation.

use std::fmt;

/// Strong type for trading symbols with validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol(String);

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

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Strongly typed exchange enum
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Exchange {
    #[default]
    Smart,
    Nasdaq,
    Nyse,
    Cboe,
    Globex,
    Idealpro,
    Paxos,
    Eurex,
    Lse,
    Tsej,
    Custom(String),
}

impl Exchange {
    pub fn as_str(&self) -> &str {
        match self {
            Exchange::Smart => "SMART",
            Exchange::Nasdaq => "NASDAQ",
            Exchange::Nyse => "NYSE",
            Exchange::Cboe => "CBOE",
            Exchange::Globex => "GLOBEX",
            Exchange::Idealpro => "IDEALPRO",
            Exchange::Paxos => "PAXOS",
            Exchange::Eurex => "EUREX",
            Exchange::Lse => "LSE",
            Exchange::Tsej => "TSEJ",
            Exchange::Custom(s) => s,
        }
    }
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Strongly typed currency enum
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Currency {
    #[default]
    USD,
    EUR,
    GBP,
    JPY,
    CHF,
    CAD,
    AUD,
    Custom(String),
}

impl Currency {
    pub fn as_str(&self) -> &str {
        match self {
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
            Currency::JPY => "JPY",
            Currency::CHF => "CHF",
            Currency::CAD => "CAD",
            Currency::AUD => "AUD",
            Currency::Custom(s) => s,
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
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

    pub fn to_string(&self) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day)
    }
}

impl fmt::Display for ExpirationDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
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

    pub fn to_string(&self) -> String {
        format!("{:04}{:02}", self.year, self.month)
    }
}

impl fmt::Display for ContractMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// CUSIP identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cusip(String);

impl Cusip {
    pub fn new(s: impl Into<String>) -> Self {
        Cusip(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Cusip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ISIN identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Isin(String);

impl Isin {
    pub fn new(s: impl Into<String>) -> Self {
        Isin(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
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

/// Trading action for spreads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Buy,
    Sell,
}

impl Action {
    pub fn as_str(&self) -> &str {
        match self {
            Action::Buy => "BUY",
            Action::Sell => "SELL",
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Marker type for missing required fields in type-state builders
#[derive(Debug, Clone, Copy)]
pub struct Missing;
