// TODO: Implement async version of realtime market data
#![cfg(feature = "sync")]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::client::{DataStream, ResponseContext};
use crate::contracts::OptionComputation;
use crate::messages::{self, IncomingMessages, Notice, RequestMessage, ResponseMessage};
use crate::ToField;
use crate::{Client, Error};

pub(crate) mod common;
pub mod sync;

// Re-export tick types
pub use crate::contracts::tick_types::TickType;

// === Models ===

/// Bar size for real-time bars.
///
/// Note: Currently only 5-second bars are supported for real-time data.
#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq)]
pub enum BarSize {
    // Sec,
    /// 5-second bars.
    Sec5,
    // Sec15,
    // Sec30,
    // Min,
    // Min2,
    // Min3,
    // Min5,
    // Min15,
    // Min30,
    // Hour,
    // Day,
}

/// Represents `BidAsk` tick by tick realtime tick.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct BidAsk {
    /// The spread's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// tick-by-tick real-time tick bid price
    pub bid_price: f64,
    /// tick-by-tick real-time tick ask price
    pub ask_price: f64,
    /// tick-by-tick real-time tick bid size
    pub bid_size: f64,
    /// tick-by-tick real-time tick ask size
    pub ask_size: f64,
    /// tick-by-tick real-time bid/ask tick attribs (bit 0 - bid past low, bit 1 - ask past high)
    pub bid_ask_attribute: BidAskAttribute,
}

impl DataStream<BidAsk> for BidAsk {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::TickByTick];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickByTick => common::decoders::decode_bid_ask_tick(message),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel realtime bars");
        common::encoders::encode_cancel_tick_by_tick(request_id)
    }
}

/// Attributes for bid/ask tick data.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct BidAskAttribute {
    /// Indicates if the bid price is past the daily low.
    pub bid_past_low: bool,
    /// Indicates if the ask price is past the daily high.
    pub ask_past_high: bool,
}

/// Represents `MidPoint` tick by tick realtime tick.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MidPoint {
    /// The trade's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// mid point
    pub mid_point: f64,
}

impl DataStream<MidPoint> for MidPoint {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::TickByTick];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickByTick => common::decoders::decode_mid_point_tick(message),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel mid point ticks");
        common::encoders::encode_cancel_tick_by_tick(request_id)
    }
}

/// Represents a real-time bar with OHLCV data
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Bar {
    /// The timestamp of the bar in market timezone
    pub date: OffsetDateTime,
    /// Opening price during the bar period
    pub open: f64,
    /// Highest price during the bar period
    pub high: f64,
    /// Lowest price during the bar period
    pub low: f64,
    /// Closing price of the bar period
    pub close: f64,
    /// Total volume traded during the bar period
    pub volume: f64,
    /// Volume weighted average price
    pub wap: f64,
    /// Number of trades during the bar period
    pub count: i32,
}

impl DataStream<Bar> for Bar {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::RealTimeBars];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        common::decoders::decode_realtime_bar(message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel realtime bars");
        common::encoders::encode_cancel_realtime_bars(request_id)
    }
}

/// Represents `Last` or `AllLast` tick-by-tick real-time tick.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Trade {
    /// Tick type: `Last` or `AllLast`
    pub tick_type: String,
    /// The trade's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// Tick last price
    pub price: f64,
    /// Tick last size
    pub size: f64,
    /// Tick attributes (bit 0 - past limit, bit 1 - unreported)
    pub trade_attribute: TradeAttribute,
    /// Tick exchange
    pub exchange: String,
    /// Tick special conditions
    pub special_conditions: String,
}

impl DataStream<Trade> for Trade {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::TickByTick];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickByTick => common::decoders::decode_trade_tick(message),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel realtime bars");
        common::encoders::encode_cancel_tick_by_tick(request_id)
    }
}

/// Attributes for trade tick data.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TradeAttribute {
    /// Indicates if the trade occurred past the limit price.
    pub past_limit: bool,
    /// Indicates if the trade was unreported.
    pub unreported: bool,
}

/// Specifies the type of data to show for real-time bars.
#[derive(Clone, Debug, Copy)]
pub enum WhatToShow {
    /// Trade data.
    Trades,
    /// Midpoint between bid and ask.
    MidPoint,
    /// Bid prices.
    Bid,
    /// Ask prices.
    Ask,
}

impl std::fmt::Display for WhatToShow {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Trades => write!(f, "TRADES"),
            Self::MidPoint => write!(f, "MIDPOINT"),
            Self::Bid => write!(f, "BID"),
            Self::Ask => write!(f, "ASK"),
        }
    }
}

impl ToField for WhatToShow {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

/// Market depth data types.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum MarketDepths {
    MarketDepth(MarketDepth),
    MarketDepthL2(MarketDepthL2),
    Notice(Notice),
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
/// Returns the order book.
pub struct MarketDepth {
    /// The order book's row being updated
    pub position: i32,
    /// How to refresh the row: 0 - insert (insert this new order into the row identified by 'position')· 1 - update (update the existing order in the row identified by 'position')· 2 - delete (delete the existing order at the row identified by 'position').
    pub operation: i32,
    /// 0 for ask, 1 for bid
    pub side: i32,
    // The order's price
    pub price: f64,
    // The order's size
    pub size: f64,
}

/// Returns the order book.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MarketDepthL2 {
    /// The order book's row being updated
    pub position: i32,
    /// The exchange holding the order if isSmartDepth is True, otherwise the MPID of the market maker
    pub market_maker: String,
    /// How to refresh the row: 0 - insert (insert this new order into the row identified by 'position')· 1 - update (update the existing order in the row identified by 'position')· 2 - delete (delete the existing order at the row identified by 'position').
    pub operation: i32,
    /// 0 for ask, 1 for bid
    pub side: i32,
    // The order's price
    pub price: f64,
    // The order's size
    pub size: f64,
    /// Flag indicating if this is smart depth response (aggregate data from multiple exchanges, v974+)
    pub smart_depth: bool,
}

impl DataStream<MarketDepths> for MarketDepths {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::MarketDepth, IncomingMessages::MarketDepthL2, IncomingMessages::Error];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::MarketDepth => Ok(MarketDepths::MarketDepth(common::decoders::decode_market_depth(message)?)),
            IncomingMessages::MarketDepthL2 => Ok(MarketDepths::MarketDepthL2(common::decoders::decode_market_depth_l2(
                client.server_version,
                message,
            )?)),
            IncomingMessages::Error => {
                let code = message.peek_int(messages::CODE_INDEX).unwrap();
                if (2100..2200).contains(&code) {
                    Ok(MarketDepths::Notice(Notice::from(message)))
                } else {
                    Err(Error::from(message.clone()))
                }
            }
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(server_version: i32, request_id: Option<i32>, context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel realtime bars");
        common::encoders::encode_cancel_market_depth(server_version, request_id, context.is_smart_depth)
    }
}

/// Stores depth market data description.
#[derive(Debug, Default)]
pub struct DepthMarketDataDescription {
    /// The exchange name
    pub exchange_name: String,
    /// The security type
    pub security_type: String,
    /// The listing exchange name
    pub listing_exchange: String,
    /// The service data type
    pub service_data_type: String,
    /// The aggregated group
    pub aggregated_group: Option<String>,
}

/// Various types of market data ticks.
#[derive(Debug)]
pub enum TickTypes {
    Price(TickPrice),
    Size(TickSize),
    String(TickString),
    EFP(TickEFP),
    Generic(TickGeneric),
    OptionComputation(OptionComputation),
    SnapshotEnd,
    Notice(Notice),
    RequestParameters(TickRequestParameters),
    PriceSize(TickPriceSize),
}

impl DataStream<TickTypes> for TickTypes {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[
        IncomingMessages::TickPrice,
        IncomingMessages::TickSize,
        IncomingMessages::TickString,
        IncomingMessages::TickEFP,
        IncomingMessages::TickGeneric,
        IncomingMessages::TickOptionComputation,
        IncomingMessages::TickSnapshotEnd,
        IncomingMessages::Error,
        IncomingMessages::TickReqParams,
    ];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickPrice => Ok(common::decoders::decode_tick_price(client.server_version, message)?),
            IncomingMessages::TickSize => Ok(TickTypes::Size(common::decoders::decode_tick_size(message)?)),
            IncomingMessages::TickString => Ok(TickTypes::String(common::decoders::decode_tick_string(message)?)),
            IncomingMessages::TickEFP => Ok(TickTypes::EFP(common::decoders::decode_tick_efp(message)?)),
            IncomingMessages::TickGeneric => Ok(TickTypes::Generic(common::decoders::decode_tick_generic(message)?)),
            IncomingMessages::TickOptionComputation => Ok(TickTypes::OptionComputation(common::decoders::decode_tick_option_computation(
                client.server_version,
                message,
            )?)),
            IncomingMessages::TickReqParams => Ok(TickTypes::RequestParameters(common::decoders::decode_tick_request_parameters(message)?)),
            IncomingMessages::TickSnapshotEnd => Ok(TickTypes::SnapshotEnd),
            IncomingMessages::Error => Ok(TickTypes::Notice(Notice::from(message))),
            _ => Err(Error::NotImplemented),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel realtime bars");
        common::encoders::encode_cancel_market_data(request_id)
    }

    fn is_snapshot_end(&self) -> bool {
        matches!(self, TickTypes::SnapshotEnd)
    }
}

/// Price tick data.
#[derive(Debug, Default)]
pub struct TickPrice {
    /// Type of price tick (bid, ask, last, etc.).
    pub tick_type: TickType,
    /// The price value.
    pub price: f64,
    /// Additional attributes for the price tick.
    pub attributes: TickAttribute,
}

/// Attributes associated with price ticks.
#[derive(Debug, PartialEq, Default)]
pub struct TickAttribute {
    /// Indicates if the order can be automatically executed.
    pub can_auto_execute: bool,
    /// Indicates if the price is past the limit.
    pub past_limit: bool,
    /// Indicates if this is a pre-market opening price.
    pub pre_open: bool,
}

/// Size tick data.
#[derive(Debug, Default)]
pub struct TickSize {
    /// Type of size tick (bid size, ask size, etc.).
    pub tick_type: TickType,
    /// The size value.
    pub size: f64,
}

/// Combined price and size tick data.
#[derive(Debug, Default)]
pub struct TickPriceSize {
    /// Type of price tick.
    pub price_tick_type: TickType,
    /// The price value.
    pub price: f64,
    /// Price tick attributes.
    pub attributes: TickAttribute,
    /// Type of size tick.
    pub size_tick_type: TickType,
    /// The size value.
    pub size: f64,
}

/// String-based tick data.
#[derive(Debug, Default)]
pub struct TickString {
    /// Type of string tick.
    pub tick_type: TickType,
    /// The string value.
    pub value: String,
}

/// Exchange for Physical (EFP) tick data.
#[derive(Debug, Default)]
pub struct TickEFP {
    /// Type of EFP tick.
    pub tick_type: TickType,
    /// EFP basis points.
    pub basis_points: f64,
    /// Formatted basis points string.
    pub formatted_basis_points: String,
    /// Implied futures price.
    pub implied_futures_price: f64,
    /// Number of hold days.
    pub hold_days: i32,
    /// Future's last trade date.
    pub future_last_trade_date: String,
    /// Dividend impact on the EFP.
    pub dividend_impact: f64,
    /// Dividends to last trade date.
    pub dividends_to_last_trade_date: f64,
}

/// Generic tick data.
#[derive(Debug, Default)]
pub struct TickGeneric {
    /// Type of generic tick.
    pub tick_type: TickType,
    /// The numeric value.
    pub value: f64,
}

/// Parameters related to tick data requests.
#[derive(Debug, Default)]
pub struct TickRequestParameters {
    /// Minimum tick increment.
    pub min_tick: f64,
    /// Best Bid/Offer exchange.
    pub bbo_exchange: String,
    /// Snapshot permissions code.
    pub snapshot_permissions: i32,
}

// === Implementation ===

// Re-export sync functions when sync feature is enabled
pub(crate) use sync::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_what_to_show_display() {
        assert_eq!(WhatToShow::Trades.to_string(), "TRADES");
        assert_eq!(WhatToShow::MidPoint.to_string(), "MIDPOINT");
        assert_eq!(WhatToShow::Bid.to_string(), "BID");
        assert_eq!(WhatToShow::Ask.to_string(), "ASK");
    }
}
