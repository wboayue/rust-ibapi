//! Strong types for contract building with validation.

use crate::builder::ValidationError;
use crate::{Error, ToField};
use std::fmt;
use std::str::FromStr;
use time::{Date, Duration, Month, OffsetDateTime, PrimitiveDateTime, Weekday};

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;

/// Contract identifier
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractId(pub i32);

impl ContractId {
    /// Get the inner value
    pub fn value(&self) -> i32 {
        self.0
    }
}

impl fmt::Display for ContractId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for ContractId {
    fn from(id: i32) -> Self {
        Self(id)
    }
}

impl From<&i32> for ContractId {
    fn from(id: &i32) -> Self {
        Self(*id)
    }
}

impl From<ContractId> for i32 {
    fn from(id: ContractId) -> Self {
        id.0
    }
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
///
/// The default is `SMART`, IB's smart router, which a `Contract` struct literal ending in
/// `..Default::default()` takes for `exchange` (its `primary_exchange` stays empty).
/// [`ContractBuilder::new`](crate::contracts::ContractBuilder::new) builds an empty exchange
/// unless [`exchange`](crate::contracts::ContractBuilder::exchange) sets one.
///
/// ```
/// use ibapi::contracts::{Contract, Exchange};
///
/// assert_eq!(Exchange::default().as_str(), "SMART");
/// assert_eq!(Contract::default().exchange, Exchange::default());
/// ```
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
///
/// The default is empty, which leaves the currency to TWS: it resolves a contract whose
/// other fields name one listing (an HSI future on HKFE resolves to HKD) and lists every
/// listing for contract details. Market data and orders on a contract that matches listings in
/// several currencies (AAPL on SMART also matches EBS in CHF and TSE in CAD) fail with error
/// 200, "The contract description specified for AAPL is ambiguous; you must specify the
/// currency". Set the currency for those.
///
/// ```
/// use ibapi::contracts::{Contract, Currency};
///
/// assert_eq!(Currency::default().as_str(), "");
/// assert_eq!(Contract::default().currency, Currency::default());
/// ```
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl_wire_enum!(OptionRight { Call, Put });

/// Security identifier scheme. Matches IBKR's `secIdType` wire vocabulary.
///
/// No `Default` — `Contract.security_id_type: Option<SecurityIdType>` carries
/// the no-identifier state via `None`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SecurityIdType {
    /// CUSIP — North American security identifier.
    Cusip,
    /// ISIN — international security identifier.
    Isin,
    /// SEDOL — UK/Irish security identifier.
    Sedol,
    /// RIC — Reuters Instrument Code.
    Ric,
    /// FIGI — Bloomberg Financial Instrument Global Identifier.
    Figi,
    /// Identifier scheme not modeled by this version of the API. Carries the
    /// raw wire value as received on an inbound contract; sent back unchanged.
    Unknown(String),
}

impl SecurityIdType {
    /// Return the canonical IBKR wire string — for
    /// [`Unknown`](Self::Unknown), the raw value as received.
    pub fn as_str(&self) -> &str {
        match self {
            SecurityIdType::Cusip => "CUSIP",
            SecurityIdType::Isin => "ISIN",
            SecurityIdType::Sedol => "SEDOL",
            SecurityIdType::Ric => "RIC",
            SecurityIdType::Figi => "FIGI",
            SecurityIdType::Unknown(raw) => raw,
        }
    }

    fn from_wire(s: &str) -> Option<Self> {
        match s {
            "CUSIP" => Some(Self::Cusip),
            "ISIN" => Some(Self::Isin),
            "SEDOL" => Some(Self::Sedol),
            "RIC" => Some(Self::Ric),
            "FIGI" => Some(Self::Figi),
            _ => None,
        }
    }
}

// Open: decoded on every inbound `Contract`, including the `OpenOrder` and
// `ExecutionData` frames of `order_update_stream`, and IBKR's catalogue grows.
// A new scheme must not end those streams (#840).
impl_wire_enum!(SecurityIdType, fallback Unknown);

/// Validated strike price (must be positive)
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strike(f64);

impl Strike {
    /// Construct a validated strike price ensuring it is positive and finite.
    ///
    /// # Errors
    ///
    /// [`ValidationError::InvalidPrice`] for zero, a negative, NaN or an infinite price.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::builder::ValidationError;
    /// use ibapi::contracts::Strike;
    ///
    /// assert_eq!(Strike::new(150.0)?.value(), 150.0);
    /// assert_eq!(Strike::new(0.0), Err(ValidationError::InvalidPrice(0.0)));
    /// # Ok::<(), ValidationError>(())
    /// ```
    pub fn new(price: f64) -> Result<Self, ValidationError> {
        if !price.is_finite() || price <= 0.0 {
            Err(ValidationError::InvalidPrice(price))
        } else {
            Ok(Strike(price))
        }
    }

    /// Access the numeric strike value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Date for option expiration
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
// Field order (year, month, day) makes the derived `Ord` chronological.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExpirationDate {
    year: u16,
    month: u8,
    day: u8,
}

impl ExpirationDate {
    /// Create an option expiration date from year/month/day components.
    ///
    /// # Errors
    ///
    /// [`ValidationError::InvalidDate`] unless the components form a real calendar
    /// date with a year of at most 9999 (the wire form is `YYYYMMDD`).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::ExpirationDate;
    ///
    /// let date = ExpirationDate::new(2026, 12, 18)?;
    /// assert_eq!(date.to_string(), "20261218");
    ///
    /// assert!(ExpirationDate::new(2026, 2, 30).is_err());
    /// assert!(ExpirationDate::new(2026, 13, 1).is_err());
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, ValidationError> {
        Self::unchecked(year, month, day).validated()
    }

    /// The year, 0–9999.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::ExpirationDate;
    ///
    /// let date: ExpirationDate = "20261218".parse()?;
    /// assert_eq!((date.year(), date.month(), date.day()), (2026, 12, 18));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn year(&self) -> u16 {
        self.year
    }

    /// The month, 1–12.
    pub fn month(&self) -> u8 {
        self.month
    }

    /// The day of the month, 1–31.
    pub fn day(&self) -> u8 {
        self.day
    }

    /// Store the components without checking them. Only for a builder that
    /// calls [`validated`](Self::validated) before the value leaves it.
    pub(crate) fn unchecked(year: u16, month: u8, day: u8) -> Self {
        ExpirationDate { year, month, day }
    }

    /// `Ok(self)` if the components form a real date with a 4-digit year.
    pub(crate) fn validated(self) -> Result<Self, ValidationError> {
        let invalid = || ValidationError::InvalidDate {
            value: format!("{:04}-{:02}-{:02}", self.year, self.month, self.day),
            expected: "a real calendar date, year 0-9999",
        };
        if self.year > 9999 {
            return Err(invalid());
        }
        let month = Month::try_from(self.month).map_err(|_| invalid())?;
        Date::from_calendar_date(i32::from(self.year), month, self.day).map_err(|_| invalid())?;
        Ok(self)
    }

    /// From a calendar date, valid by construction for years 0–9999.
    fn from_date(date: Date) -> Self {
        ExpirationDate {
            year: date.year() as u16,
            month: date.month().into(),
            day: date.day(),
        }
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
        Self::from_date(today + Duration::days(days_to_add))
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
            Self::from_date(next_month + Duration::days(days_to_first_friday_next + 14))
        } else {
            Self::from_date(third_friday)
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
// Field order (year, month) makes the derived `Ord` chronological.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractMonth {
    year: u16,
    month: u8,
}

impl ContractMonth {
    /// Construct a futures contract month given year and month.
    ///
    /// # Errors
    ///
    /// [`ValidationError::InvalidDate`] unless `month` is 1–12 and `year` is at most
    /// 9999 (the wire form is `YYYYMM`).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::ContractMonth;
    ///
    /// let month = ContractMonth::new(2026, 12)?;
    /// assert_eq!(month.to_string(), "202612");
    ///
    /// assert!(ContractMonth::new(2026, 13).is_err());
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn new(year: u16, month: u8) -> Result<Self, ValidationError> {
        if year > 9999 || !(1..=12).contains(&month) {
            return Err(ValidationError::InvalidDate {
                value: format!("{year:04}-{month:02}"),
                expected: "month 1-12, year 0-9999",
            });
        }
        Ok(ContractMonth { year, month })
    }

    /// The year, 0–9999.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::ContractMonth;
    ///
    /// let month: ContractMonth = "202612".parse()?;
    /// assert_eq!((month.year(), month.month()), (2026, 12));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn year(&self) -> u16 {
        self.year
    }

    /// The month, 1–12.
    pub fn month(&self) -> u8 {
        self.month
    }

    /// Get the front month contract (next expiring)
    pub fn front() -> Self {
        let now = OffsetDateTime::now_utc();
        Self::front_from(now.year() as u16, now.month() as u8, now.day())
    }

    // Builds the struct directly: `current_month` is 1–12 and every month
    // computed from it stays in range, so `new` has nothing to reject.
    fn front_from(current_year: u16, current_month: u8, current_day: u8) -> Self {
        // Futures typically expire around the third Friday of the month
        // If we're past the 15th, assume current month has expired
        if current_day > 15 {
            if current_month == 12 {
                ContractMonth {
                    year: current_year + 1,
                    month: 1,
                }
            } else {
                ContractMonth {
                    year: current_year,
                    month: current_month + 1,
                }
            }
        } else {
            ContractMonth {
                year: current_year,
                month: current_month,
            }
        }
    }

    /// Get the next quarterly contract month (Mar, Jun, Sep, Dec)
    pub fn next_quarter() -> Self {
        let now = OffsetDateTime::now_utc();
        Self::next_quarter_from(now.year() as u16, now.month() as u8, now.day())
    }

    // Builds the struct directly: the month is always 3, 6, 9 or 12.
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

        ContractMonth {
            year,
            month: next_quarter_month,
        }
    }
}

impl fmt::Display for ContractMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}{:02}", self.year, self.month)
    }
}

/// `YYYYMMDD` (8 digits, a real calendar date) → [`ExpirationDate`]; anything
/// else is a [`ValidationError::InvalidDate`].
///
/// # Examples
///
/// ```
/// use ibapi::contracts::ExpirationDate;
///
/// let date: ExpirationDate = "20261218".parse().unwrap();
/// assert_eq!(date, ExpirationDate::new(2026, 12, 18)?);
/// assert!("20261340".parse::<ExpirationDate>().is_err());
/// # Ok::<(), ibapi::Error>(())
/// ```
impl FromStr for ExpirationDate {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || ValidationError::InvalidDate {
            value: s.to_owned(),
            expected: "YYYYMMDD",
        };
        if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid());
        }
        let date = Date::parse(s, time::macros::format_description!("[year][month][day]")).map_err(|_| invalid())?;
        Ok(Self::from_date(date))
    }
}

impl TryFrom<&str> for ExpirationDate {
    type Error = ValidationError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// `YYYYMM` (6 digits, month 01–12) → [`ContractMonth`]; anything else is a
/// [`ValidationError::InvalidDate`].
///
/// # Examples
///
/// ```
/// use ibapi::contracts::ContractMonth;
///
/// let month: ContractMonth = "202612".parse().unwrap();
/// assert_eq!(month, ContractMonth::new(2026, 12)?);
/// assert!("202613".parse::<ContractMonth>().is_err());
/// # Ok::<(), ibapi::Error>(())
/// ```
impl FromStr for ContractMonth {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || ValidationError::InvalidDate {
            value: s.to_owned(),
            expected: "YYYYMM",
        };
        if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid());
        }
        let (year, month) = (s[..4].parse().map_err(|_| invalid())?, s[4..].parse().map_err(|_| invalid())?);
        ContractMonth::new(year, month).map_err(|_| invalid())
    }
}

impl TryFrom<&str> for ContractMonth {
    type Error = ValidationError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// A contract's last trading day or contract month: the typed form of
/// [`Contract::last_trade_date_or_contract_month`](crate::contracts::Contract::last_trade_date_or_contract_month)
/// (TWS tag 4).
///
/// TWS reads `YYYYMMDD` as the last trading day and `YYYYMM` as the contract
/// month. A contract sent as a month comes back from TWS as its last trading
/// day (an order placed on ES `202612` is echoed as `20261218`), so don't
/// expect the variant to round-trip.
///
/// # Examples
///
/// ```
/// use ibapi::contracts::{ContractMonth, ExpirationDate, Expiry};
///
/// let month: Expiry = "202612".parse().unwrap();
/// assert_eq!(month, Expiry::Month(ContractMonth::new(2026, 12)?));
///
/// let date = Expiry::from(ExpirationDate::new(2026, 12, 18)?);
/// assert_eq!(date.to_string(), "20261218");
/// # Ok::<(), ibapi::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expiry {
    /// Last trading day, `YYYYMMDD`.
    Date(ExpirationDate),
    /// Contract month, `YYYYMM`.
    Month(ContractMonth),
    /// Any other text TWS sent, kept verbatim and sent back as is: a bond's
    /// `"20430504 16:00 Europe/London"`, or a shape not seen yet. Parsing a
    /// string never produces it; decoding and deserializing do. It is the
    /// unchecked escape hatch: a hand-built `Unknown` is sent as is, so holding
    /// an `Expiry` does not mean the value was validated.
    Unknown(String),
}

impl Expiry {
    /// Lenient decode of the wire text: empty is `None`, a date or month
    /// types, anything else is [`Expiry::Unknown`]. Contracts arrive on
    /// streaming paths (positions, open orders, executions), where a parse
    /// error would end the subscription (docs/rules/wire/enum-typing.md).
    pub(crate) fn from_wire(raw: &str) -> Option<Expiry> {
        if raw.is_empty() {
            return None;
        }
        Some(raw.parse().unwrap_or_else(|_| Expiry::Unknown(raw.to_owned())))
    }

    /// The last trading day, if this is an [`Expiry::Date`].
    ///
    /// `Expiry` has no order of its own; take the dates to sort or find the
    /// nearest one.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{ExpirationDate, Expiry};
    ///
    /// let expirations: Vec<Expiry> = vec!["20261218".parse()?, "202612".parse()?, "20260918".parse()?];
    /// let nearest = expirations.iter().filter_map(Expiry::date).min();
    /// assert_eq!(nearest, Some(ExpirationDate::new(2026, 9, 18)?));
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn date(&self) -> Option<ExpirationDate> {
        match self {
            Expiry::Date(date) => Some(*date),
            _ => None,
        }
    }

    /// The contract month, if this is an [`Expiry::Month`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{ContractMonth, Expiry};
    ///
    /// let month: Expiry = "202612".parse()?;
    /// assert_eq!(month.month(), Some(ContractMonth::new(2026, 12)?));
    /// assert_eq!(month.date(), None);
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn month(&self) -> Option<ContractMonth> {
        match self {
            Expiry::Month(month) => Some(*month),
            _ => None,
        }
    }
}

impl fmt::Display for Expiry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expiry::Date(date) => date.fmt(f),
            Expiry::Month(month) => month.fmt(f),
            Expiry::Unknown(raw) => f.write_str(raw),
        }
    }
}

/// Strict: `YYYYMMDD` → [`Expiry::Date`], `YYYYMM` → [`Expiry::Month`], anything
/// else is a [`ValidationError::InvalidDate`].
impl FromStr for Expiry {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.len() {
            8 => s.parse().map(Expiry::Date),
            6 => s.parse().map(Expiry::Month),
            _ => Err(ValidationError::InvalidDate {
                value: s.to_owned(),
                expected: "YYYYMMDD (last trading day) or YYYYMM (contract month)",
            }),
        }
    }
}

impl TryFrom<&str> for Expiry {
    type Error = ValidationError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<ExpirationDate> for Expiry {
    fn from(date: ExpirationDate) -> Self {
        Expiry::Date(date)
    }
}

impl From<ContractMonth> for Expiry {
    fn from(month: ContractMonth) -> Self {
        Expiry::Month(month)
    }
}

// Serde as the wire string, so a contract serializes `"20261218"` as before
// rather than `{"Date":{"year":..}}`. Deserialize is lenient like the
// decoder, so `Unknown` round-trips, and unchecked text loads as `Unknown`.
// The utoipa schema is hand-written to match: exactly a string.
#[cfg(feature = "utoipa")]
impl utoipa::PartialSchema for Expiry {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        <String as utoipa::PartialSchema>::schema()
    }
}

#[cfg(feature = "utoipa")]
impl utoipa::ToSchema for Expiry {}

impl serde::Serialize for Expiry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Expiry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Expiry::from_wire(&raw).ok_or_else(|| serde::de::Error::custom("empty expiry; use null"))
    }
}

/// `Contract::last_trade_date_or_contract_month` deserializer: `null` and `""`
/// are `None`. 5.x serialized an unset field as `""`, so its JSON still loads.
pub(crate) fn deserialize_optional_expiry<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Expiry>, D::Error> {
    use serde::Deserialize;
    Ok(Option::<String>::deserialize(deserializer)?.as_deref().and_then(Expiry::from_wire))
}

/// One trading session from [`ContractDetails::trading_sessions`](crate::contracts::ContractDetails::trading_sessions)
/// or [`liquid_sessions`](crate::contracts::ContractDetails::liquid_sessions).
///
/// Times are wall clock in the product's
/// [`time_zone_id`](crate::contracts::ContractDetails::time_zone_id), as TWS sends them: no zone
/// is applied, so turning one into an instant is the caller's step.
///
/// Parses from one `;`-separated entry with [`FromStr`]:
///
/// ```
/// use ibapi::contracts::TradingSession;
/// use time::macros::date;
///
/// let session: TradingSession = "20261114:CLOSED".parse()?;
/// assert_eq!(session, TradingSession::Closed(date!(2026 - 11 - 14)));
/// # Ok::<(), ibapi::Error>(())
/// ```
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TradingSession {
    /// The product trades from `start` to `end`. A session can span midnight (17:00 to 16:00
    /// the next day), so the end carries its own date.
    Open {
        /// Session open.
        start: PrimitiveDateTime,
        /// Session close.
        end: PrimitiveDateTime,
    },
    /// The product does not trade on this date.
    Closed(Date),
}

const EXPECTED_SESSION_FORMAT: &str = "expected YYYYMMDD:HHMM-YYYYMMDD:HHMM or YYYYMMDD:CLOSED";

impl FromStr for TradingSession {
    type Err = Error;

    /// Parses one entry in the form TWS 970 and later sends: `YYYYMMDD:HHMM-YYYYMMDD:HHMM`
    /// or `YYYYMMDD:CLOSED`. The pre-970 form (`20090507:0700-1830`) is an [`Error::Parse`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || Error::invalid_value(s, EXPECTED_SESSION_FORMAT);

        if let Some(date) = s.strip_suffix(":CLOSED") {
            let date = Date::parse(date, time::macros::format_description!("[year][month][day]")).map_err(|_| invalid())?;
            return Ok(TradingSession::Closed(date));
        }

        let (start, end) = s.split_once('-').ok_or_else(invalid)?;
        let parse = |value: &str| {
            PrimitiveDateTime::parse(value, time::macros::format_description!("[year][month][day]:[hour][minute]")).map_err(|_| invalid())
        };
        Ok(TradingSession::Open {
            start: parse(start)?,
            end: parse(end)?,
        })
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

    /// Currency implied by the country code (first two characters), USD when unknown.
    pub(crate) fn implied_currency(&self) -> &'static str {
        match self.0.get(0..2) {
            Some("US") | Some("CA") => "USD",
            Some("GB") => "GBP",
            Some("JP") => "JPY",
            Some("CH") => "CHF",
            Some("AU") => "AUD",
            Some("DE") | Some("FR") | Some("IT") | Some("ES") | Some("NL") | Some("BE") => "EUR",
            _ => "USD",
        }
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
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

impl_wire_enum!(LegAction { Buy, Sell, SellShort });

/// Marker type for a futures builder with an unspecified contract month (open query).
///
/// Reached via [`FuturesBuilder::any_month`](crate::contracts::FuturesBuilder::any_month) to build
/// a month-less futures contract for an enumerate-all `contract_details` lookup.
#[derive(Debug, Clone, Copy)]
pub struct AnyMonth;
