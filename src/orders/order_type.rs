//! [`OrderType`]: the TWS order-type code on [`Order`](crate::orders::Order)'s
//! `order_type`, `adjusted_order_type` and `delta_neutral_order_type`, and the type
//! [`OrderBuilder::order_type`](crate::orders::OrderBuilder::order_type) takes.

use serde::Deserialize;

/// Order types supported by Interactive Brokers.
///
/// [`Order::order_type`](crate::orders::Order::order_type) holds one and
/// [`OrderBuilder::order_type`](crate::orders::OrderBuilder::order_type) takes one. `Display`
/// and [`as_str`](Self::as_str) give the TWS wire string (`"LMT"`, `"STP LMT"`, ...);
/// `s.parse::<OrderType>()` reads one. Serde uses the wire string in both directions.
///
/// A market-on-open, limit-on-open or auction order is a plain `Market`, `Limit` or
/// `MarketToLimit` with [`TimeInForce::OnOpen`](crate::orders::TimeInForce::OnOpen) or
/// [`TimeInForce::Auction`](crate::orders::TimeInForce::Auction); the builder's
/// [`market_on_open`](crate::orders::OrderBuilder::market_on_open),
/// [`limit_on_open`](crate::orders::OrderBuilder::limit_on_open) and
/// [`at_auction`](crate::orders::OrderBuilder::at_auction) set both.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    // Combo Orders (special handling required)
    /// Relative + limit order for combo legs.
    RelativeLimitCombo,
    /// Relative + market order for combo legs.
    RelativeMarketCombo,

    /// Order type string not modeled by this version of the API. Carries the raw
    /// wire value as received on an inbound order; sent back unchanged, so an order
    /// read from TWS round-trips. Matching is exact and case-sensitive, and only the
    /// canonical code maps to a named variant: aliases IB's Java client also accepts
    /// (`"STOP LIMIT"`, `"MKT TO LMT"`, `"TRAILLMT"`, ...) parse as `Unknown` and round-trip,
    /// but don't equal [`StopLimit`](Self::StopLimit) and the rest. The #1220 probe saw TWS echo
    /// only canonical codes (`docs/wire-facts/orders.md`). To send a code
    /// this version does not list, build one (`OrderType::Unknown("SNAP MID".into())`) or
    /// parse it (`"SNAP MID".parse::<OrderType>()`). Not for a listed code: `Unknown("LMT")`
    /// sends the same string as [`Limit`](Self::Limit) but is not equal to it, and parsing
    /// never produces it. An unset `Order` field is `None`, never `Unknown("")`. The builder
    /// sends every price set for an `Unknown` type, unchecked
    /// ([`OrderBuilder::order_type`](crate::orders::OrderBuilder::order_type)).
    Unknown(String),
}

impl OrderType {
    /// Return the TWS API string identifier for this order type — for
    /// [`Unknown`](Self::Unknown), the raw value as received.
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

            // Combo Orders
            Self::RelativeLimitCombo => "REL + LMT",
            Self::RelativeMarketCombo => "REL + MKT",

            Self::Unknown(raw) => raw,
        }
    }

    fn from_wire(s: &str) -> Option<Self> {
        match s {
            "MKT" => Some(Self::Market),
            "LMT" => Some(Self::Limit),
            "STP" => Some(Self::Stop),
            "STP LMT" => Some(Self::StopLimit),
            "TRAIL" => Some(Self::TrailingStop),
            "TRAIL LIMIT" => Some(Self::TrailingStopLimit),
            "MOC" => Some(Self::MarketOnClose),
            "LOC" => Some(Self::LimitOnClose),
            "MIT" => Some(Self::MarketIfTouched),
            "LIT" => Some(Self::LimitIfTouched),
            "MKT PRT" => Some(Self::MarketWithProtection),
            "STP PRT" => Some(Self::StopWithProtection),
            "MTL" => Some(Self::MarketToLimit),
            "MIDPRICE" => Some(Self::Midprice),
            "PEG MKT" => Some(Self::PeggedToMarket),
            "PEG STK" => Some(Self::PeggedToStock),
            "PEG MID" => Some(Self::PeggedToMidpoint),
            "PEG BENCH" => Some(Self::PeggedToBenchmark),
            "PEG BEST" => Some(Self::PegBest),
            "REL" => Some(Self::Relative),
            "PASSV REL" => Some(Self::PassiveRelative),
            "VOL" => Some(Self::Volatility),
            "BOX TOP" => Some(Self::BoxTop),
            "REL + LMT" => Some(Self::RelativeLimitCombo),
            "REL + MKT" => Some(Self::RelativeMarketCombo),
            _ => None,
        }
    }

    /// Returns true if this order type requires a limit price
    pub fn requires_limit_price(&self) -> bool {
        matches!(
            self,
            // TrailingStopLimit uses limit_price_offset, not limit_price.
            Self::Limit | Self::StopLimit | Self::LimitOnClose | Self::LimitIfTouched | Self::RelativeLimitCombo
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
                | Self::PeggedToMarket
        )
    }

    // `build()` sends a price field only for the types below, so one left by an earlier
    // order-type setter is dropped. Sets follow C# `OrderSamples.cs`, plus `limit_price` for
    // PEG BEST and an `aux_price` offset on the REL combos, which only `.order_type(..)` can carry.

    /// Types whose stop price the builder sends as `aux_price`.
    pub(crate) fn uses_stop_price(&self) -> bool {
        matches!(self, Self::Stop | Self::StopLimit | Self::StopWithProtection)
    }

    /// Types that send `limit_price`, required or as an optional cap. `MarketToLimit` carries
    /// one as an auction order (`at_auction`); `market_to_limit()` clears an earlier one.
    pub(crate) fn uses_limit_price(&self) -> bool {
        self.requires_limit_price()
            || matches!(
                self,
                Self::Midprice | Self::Relative | Self::PeggedToMidpoint | Self::PegBest | Self::MarketToLimit
            )
    }

    /// Types that send `aux_price` as a trigger, offset or trail amount.
    pub(crate) fn uses_aux_price(&self) -> bool {
        self.requires_aux_price() || matches!(self, Self::PeggedToMidpoint | Self::RelativeLimitCombo | Self::RelativeMarketCombo)
    }

    /// Types that send `trailing_percent` and `trail_stop_price`.
    pub(crate) fn uses_trail(&self) -> bool {
        matches!(self, Self::TrailingStop | Self::TrailingStopLimit)
    }
}

// Open: an inbound OpenOrder / CompletedOrder field whose codes TWS assigns, so one
// this crate does not list must not end the order stream.
// Serde and utoipa use the wire string (see `impl_wire_enum!`), so `Order` JSON keeps
// `"order_type":"LMT"`. The `Order` fields go through `deserialize_optional_order_type`,
// which reads `null`, `""` and `"None"` as `None`.
impl_wire_enum!(OrderType, fallback Unknown, serde);

/// Whether an order-type string means "no order type": empty, or `None` in any case. The
/// Gateway echoes an unset `adjusted_order_type` / `delta_neutral_order_type` as `"None"`
/// (`docs/wire-facts/orders.md`); IB's docs and legacy C# decoder spell it `"NONE"`, and TWS
/// (as opposed to the Gateway) was not probed, so the match ignores case.
pub(crate) fn is_unset_order_type(code: &str) -> bool {
    code.is_empty() || code.eq_ignore_ascii_case("none")
}

/// Deserializer for `Order`'s `Option<OrderType>` fields: `null` and any code
/// [`is_unset_order_type`] accepts are `None`. 5.x serialized an unset `order_type` as `""`
/// and an unset `adjusted_order_type` or `delta_neutral_order_type` as TWS echoed it,
/// `"None"`, so that JSON still loads.
pub(crate) fn deserialize_optional_order_type<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<OrderType>, D::Error> {
    match Option::<String>::deserialize(deserializer)?.as_deref() {
        None => Ok(None),
        Some(code) if is_unset_order_type(code) => Ok(None),
        Some(code) => code.parse().map(Some).map_err(serde::de::Error::custom),
    }
}

#[cfg(test)]
#[path = "order_type_tests.rs"]
mod tests;
