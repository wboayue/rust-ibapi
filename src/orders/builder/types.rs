use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a unique order identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub i32);

impl OrderId {
    /// Creates a new OrderId
    pub fn new(id: i32) -> Self {
        Self(id)
    }

    /// Returns the inner i32 value
    pub fn value(&self) -> i32 {
        self.0
    }
}

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for OrderId {
    fn from(id: i32) -> Self {
        Self(id)
    }
}

impl From<OrderId> for i32 {
    fn from(id: OrderId) -> i32 {
        id.0
    }
}

/// Represents the order IDs for a bracket order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BracketOrderIds {
    /// The parent order ID
    pub parent: OrderId,
    /// The take profit order ID
    pub take_profit: OrderId,
    /// The stop loss order ID
    pub stop_loss: OrderId,
}

impl BracketOrderIds {
    /// Creates a new BracketOrderIds
    pub fn new(parent: i32, take_profit: i32, stop_loss: i32) -> Self {
        Self {
            parent: OrderId(parent),
            take_profit: OrderId(take_profit),
            stop_loss: OrderId(stop_loss),
        }
    }

    /// Returns all order IDs as a vector
    pub fn as_vec(&self) -> Vec<OrderId> {
        vec![self.parent, self.take_profit, self.stop_loss]
    }

    /// Returns all order IDs as i32 values
    pub fn as_i32_vec(&self) -> Vec<i32> {
        vec![self.parent.0, self.take_profit.0, self.stop_loss.0]
    }
}

impl fmt::Display for BracketOrderIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BracketOrder(parent: {}, tp: {}, sl: {})",
            self.parent, self.take_profit, self.stop_loss
        )
    }
}

impl From<Vec<i32>> for BracketOrderIds {
    fn from(ids: Vec<i32>) -> Self {
        assert_eq!(ids.len(), 3, "BracketOrderIds requires exactly 3 order IDs");
        Self::new(ids[0], ids[1], ids[2])
    }
}

impl From<[i32; 3]> for BracketOrderIds {
    fn from(ids: [i32; 3]) -> Self {
        Self::new(ids[0], ids[1], ids[2])
    }
}

/// Represents a quantity of shares/contracts
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Quantity(f64);

impl Quantity {
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if value <= 0.0 {
            return Err(ValidationError::InvalidQuantity(value));
        }
        if value.is_nan() || value.is_infinite() {
            return Err(ValidationError::InvalidQuantity(value));
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Represents a price value
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Price(f64);

impl Price {
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if value < 0.0 {
            return Err(ValidationError::InvalidPrice(value));
        }
        if value.is_nan() || value.is_infinite() {
            return Err(ValidationError::InvalidPrice(value));
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Time in force options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    Day,
    GoodTillCancel,
    ImmediateOrCancel,
    GoodTillDate { date: String },
    FillOrKill,
    GoodTillCrossing,
    DayTillCanceled,
    Auction,
}

impl TimeInForce {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Day => "DAY",
            Self::GoodTillCancel => "GTC",
            Self::ImmediateOrCancel => "IOC",
            Self::GoodTillDate { .. } => "GTD",
            Self::FillOrKill => "FOK",
            Self::GoodTillCrossing => "GTX",
            Self::DayTillCanceled => "DTC",
            Self::Auction => "AUC",
        }
    }
}

/// Auction type for auction orders
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuctionType {
    Opening,
    Closing,
    Volatility,
}

impl AuctionType {
    pub fn to_strategy(&self) -> i32 {
        match self {
            Self::Opening => 1,
            Self::Closing => 2,
            Self::Volatility => 4,
        }
    }
}

/// Order types supported by Interactive Brokers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    // Basic Orders
    Market,
    Limit,
    Stop,
    StopLimit,

    // Trailing Orders
    TrailingStop,
    TrailingStopLimit,

    // Time-based Orders
    MarketOnClose,
    LimitOnClose,
    MarketOnOpen,
    LimitOnOpen,
    AtAuction,

    // Touched Orders
    MarketIfTouched,
    LimitIfTouched,

    // Protected Orders
    MarketWithProtection,
    StopWithProtection,

    // Market Variants
    MarketToLimit,
    Midprice,

    // Pegged Orders
    PeggedToMarket,
    PeggedToStock,
    PeggedToMidpoint,
    PeggedToBenchmark,
    PegBest,

    // Relative Orders
    Relative,
    PassiveRelative,

    // Special Orders
    Volatility,
    BoxTop,
    AuctionLimit,
    AuctionRelative,

    // Combo Orders (special handling required)
    ComboLimit,
    ComboMarket,
    RelativeLimitCombo,
    RelativeMarketCombo,
}

impl OrderType {
    pub fn as_str(&self) -> &str {
        match self {
            // Basic Orders
            Self::Market => "MKT",
            Self::Limit => "LMT",
            Self::Stop => "STP",
            Self::StopLimit => "STP LMT",

            // Trailing Orders
            Self::TrailingStop => "TRAIL",
            Self::TrailingStopLimit => "TRAIL LIMIT",

            // Time-based Orders
            Self::MarketOnClose => "MOC",
            Self::LimitOnClose => "LOC",
            Self::MarketOnOpen => "MKT",
            Self::LimitOnOpen => "LMT",
            Self::AtAuction => "MTL",

            // Touched Orders
            Self::MarketIfTouched => "MIT",
            Self::LimitIfTouched => "LIT",

            // Protected Orders
            Self::MarketWithProtection => "MKT PRT",
            Self::StopWithProtection => "STP PRT",

            // Market Variants
            Self::MarketToLimit => "MTL",
            Self::Midprice => "MIDPRICE",

            // Pegged Orders
            Self::PeggedToMarket => "PEG MKT",
            Self::PeggedToStock => "PEG STK",
            Self::PeggedToMidpoint => "PEG MID",
            Self::PeggedToBenchmark => "PEG BENCH",
            Self::PegBest => "PEG BEST",

            // Relative Orders
            Self::Relative => "REL",
            Self::PassiveRelative => "PASSV REL",

            // Special Orders
            Self::Volatility => "VOL",
            Self::BoxTop => "BOX TOP",
            Self::AuctionLimit => "LMT",
            Self::AuctionRelative => "REL",

            // Combo Orders
            Self::ComboLimit => "LMT",
            Self::ComboMarket => "MKT",
            Self::RelativeLimitCombo => "REL + LMT",
            Self::RelativeMarketCombo => "REL + MKT",
        }
    }

    /// Returns true if this order type requires a limit price
    pub fn requires_limit_price(&self) -> bool {
        matches!(
            self,
            Self::Limit
                | Self::StopLimit
                | Self::LimitOnClose
                | Self::LimitOnOpen
                | Self::LimitIfTouched
                | Self::AuctionLimit
                | Self::ComboLimit
                | Self::RelativeLimitCombo
                | Self::AtAuction
                | Self::Midprice // TrailingStopLimit uses limit_price_offset, not limit_price
        )
    }

    /// Returns true if this order type requires a stop/aux price
    pub fn requires_aux_price(&self) -> bool {
        matches!(
            self,
            Self::Stop
                | Self::StopLimit
                | Self::MarketIfTouched
                | Self::LimitIfTouched
                | Self::StopWithProtection
                | Self::TrailingStop
                | Self::TrailingStopLimit
                | Self::Relative
                | Self::PassiveRelative
                | Self::AuctionRelative
                | Self::PeggedToMarket
        )
    }
}

/// Validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    InvalidQuantity(f64),
    InvalidPrice(f64),
    MissingRequiredField(&'static str),
    InvalidCombination(String),
    InvalidStopPrice { stop: f64, current: f64 },
    InvalidLimitPrice { limit: f64, current: f64 },
    InvalidBracketOrder(String),
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
        }
    }
}

impl std::error::Error for ValidationError {}

/// Represents the outcome of analyzing an order for margin/commission
#[derive(Debug, Clone, PartialEq)]
pub struct OrderAnalysis {
    pub initial_margin: Option<f64>,
    pub maintenance_margin: Option<f64>,
    pub commission: Option<f64>,
    pub commission_currency: String,
    pub warning_text: String,
}

#[cfg(test)]
mod tests;
