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
    /// Create a validated quantity ensuring it is positive and finite.
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if value <= 0.0 {
            return Err(ValidationError::InvalidQuantity(value));
        }
        if value.is_nan() || value.is_infinite() {
            return Err(ValidationError::InvalidQuantity(value));
        }
        Ok(Self(value))
    }

    /// Access the raw quantity value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Represents a price value
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Price(f64);

impl Price {
    /// Create a validated price ensuring it is non-negative and finite.
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if value < 0.0 {
            return Err(ValidationError::InvalidPrice(value));
        }
        if value.is_nan() || value.is_infinite() {
            return Err(ValidationError::InvalidPrice(value));
        }
        Ok(Self(value))
    }

    /// Access the raw price value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// Time in force options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Order is active only for the current trading day.
    Day,
    /// Order remains active until cancelled.
    GoodTillCancel,
    /// Order must be filled immediately or cancelled.
    ImmediateOrCancel,
    /// Order remains active until the specified date (`YYYYMMDD`).
    GoodTillDate {
        /// Date at which the order expires.
        date: String,
    },
    /// Order must be filled entirely or cancelled immediately.
    FillOrKill,
    /// Good-till-crossing (GTX) order type.
    GoodTillCrossing,
    /// Day-till-cancelled (DTC) order type.
    DayTillCanceled,
    /// Auction-only order.
    Auction,
    /// Opening auction order.
    OpeningAuction,
}

impl TimeInForce {
    /// Return the TWS API string identifier for the time-in-force.
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
            Self::OpeningAuction => "OPG",
        }
    }
}

/// Auction type for auction orders
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuctionType {
    /// Opening auction strategy.
    Opening,
    /// Closing auction strategy.
    Closing,
    /// Volatility auction strategy.
    Volatility,
}

impl AuctionType {
    /// Return the numeric strategy identifier used by TWS.
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
    /// Market order executed immediately at the best available price.
    Market,
    /// Limit order with a maximum/minimum execution price.
    Limit,
    /// Stop order that triggers a market order once the stop price is hit.
    Stop,
    /// Stop-limit order that triggers a limit order at the stop price.
    StopLimit,

    // Trailing Orders
    /// Trailing stop order with a moving stop offset.
    TrailingStop,
    /// Trailing stop-limit order with both stop and limit offsets.
    TrailingStopLimit,

    // Time-based Orders
    /// Market-on-close order.
    MarketOnClose,
    /// Limit-on-close order.
    LimitOnClose,
    /// Market-on-open order.
    MarketOnOpen,
    /// Limit-on-open order.
    LimitOnOpen,
    /// Auction order routed to an exchange auction.
    AtAuction,

    // Touched Orders
    /// Market-if-touched order.
    MarketIfTouched,
    /// Limit-if-touched order.
    LimitIfTouched,

    // Protected Orders
    /// Market order with price protection.
    MarketWithProtection,
    /// Stop order with price protection.
    StopWithProtection,

    // Market Variants
    /// Market-to-limit order that becomes a limit order if not filled.
    MarketToLimit,
    /// Midprice order targeting the NBBO midpoint.
    Midprice,

    // Pegged Orders
    /// Pegged-to-market order following the best quote.
    PeggedToMarket,
    /// Pegged-to-stock order for option hedging.
    PeggedToStock,
    /// Pegged-to-midpoint order tracking the midpoint.
    PeggedToMidpoint,
    /// Pegged-to-benchmark order using a benchmark price.
    PeggedToBenchmark,
    /// Peg to best order.
    PegBest,

    // Relative Orders
    /// Relative (pegged) order offset from the best price.
    Relative,
    /// Passive relative order posting liquidity.
    PassiveRelative,

    // Special Orders
    /// Volatility order for options.
    Volatility,
    /// Box-top order that converts to market at the best price.
    BoxTop,
    /// Auction limit order.
    AuctionLimit,
    /// Auction relative (pegged) order.
    AuctionRelative,

    // Combo Orders (special handling required)
    /// Limit order for combo legs.
    ComboLimit,
    /// Market order for combo legs.
    ComboMarket,
    /// Relative + limit order for combo legs.
    RelativeLimitCombo,
    /// Relative + market order for combo legs.
    RelativeMarketCombo,
}

impl OrderType {
    /// Return the TWS API string identifier for this order type.
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
    /// Quantity must be positive and finite.
    InvalidQuantity(f64),
    /// Price must be non-negative and finite.
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
    /// Initial margin requirement returned by TWS.
    pub initial_margin: Option<f64>,
    /// Maintenance margin requirement.
    pub maintenance_margin: Option<f64>,
    /// Estimated commission for the order.
    pub commission: Option<f64>,
    /// Currency for the commission figures.
    pub commission_currency: String,
    /// Free-form warnings provided by TWS.
    pub warning_text: String,
}

#[cfg(test)]
mod tests;
