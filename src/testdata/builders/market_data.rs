//! Builders for market-data domain request and response messages.
//!
//! Covers both `historical` and `realtime` submodules. Response builders are
//! gated on the family's protobuf wire being available; realtime
//! TickByTick / RealTimeBars / MarketData / MarketDepth shipped proto in
//! [#543](https://github.com/wboayue/rust-ibapi/pull/543).

use super::{RequestEncoder, ResponseProtoEncoder};
use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::contracts::{Contract, TagValue};
use crate::market_data::historical::{BarSize, Duration, WhatToShow as HistoricalWhatToShow};
use crate::market_data::realtime::WhatToShow as RealtimeWhatToShow;
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::proto::encoders::{encode_contract, some_bool, some_str, tag_values_to_map};
use crate::ToField;
use time::OffsetDateTime;

const DATE_FORMAT: i32 = 2;

// =============================================================================
// Historical request builders
// =============================================================================

#[derive(Clone, Debug)]
pub struct HeadTimestampRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub what_to_show: HistoricalWhatToShow,
    pub use_rth: bool,
}

impl Default for HeadTimestampRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            what_to_show: HistoricalWhatToShow::Trades,
            use_rth: false,
        }
    }
}

impl HeadTimestampRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, v: &Contract) -> Self {
        self.contract = v.clone();
        self
    }
    pub fn what_to_show(mut self, v: HistoricalWhatToShow) -> Self {
        self.what_to_show = v;
        self
    }
    pub fn use_rth(mut self, v: bool) -> Self {
        self.use_rth = v;
        self
    }
}

impl RequestEncoder for HeadTimestampRequestBuilder {
    type Proto = proto::HeadTimestampRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestHeadTimestamp;

    fn to_proto(&self) -> Self::Proto {
        proto::HeadTimestampRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            use_rth: some_bool(self.use_rth),
            what_to_show: Some(self.what_to_show.to_field()),
            format_date: Some(DATE_FORMAT),
        }
    }
}

#[derive(Clone, Debug)]
pub struct HistoricalDataRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub end_date: Option<OffsetDateTime>,
    pub duration: Duration,
    pub bar_size: BarSize,
    pub what_to_show: Option<HistoricalWhatToShow>,
    pub use_rth: bool,
    pub keep_up_to_date: bool,
}

impl Default for HistoricalDataRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            end_date: None,
            duration: Duration::days(1),
            bar_size: BarSize::Day,
            what_to_show: None,
            use_rth: false,
            keep_up_to_date: false,
        }
    }
}

impl HistoricalDataRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, v: &Contract) -> Self {
        self.contract = v.clone();
        self
    }
    pub fn end_date(mut self, v: Option<OffsetDateTime>) -> Self {
        self.end_date = v;
        self
    }
    pub fn duration(mut self, v: Duration) -> Self {
        self.duration = v;
        self
    }
    pub fn bar_size(mut self, v: BarSize) -> Self {
        self.bar_size = v;
        self
    }
    pub fn what_to_show(mut self, v: Option<HistoricalWhatToShow>) -> Self {
        self.what_to_show = v;
        self
    }
    pub fn use_rth(mut self, v: bool) -> Self {
        self.use_rth = v;
        self
    }
    pub fn keep_up_to_date(mut self, v: bool) -> Self {
        self.keep_up_to_date = v;
        self
    }
}

impl RequestEncoder for HistoricalDataRequestBuilder {
    type Proto = proto::HistoricalDataRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestHistoricalData;

    fn to_proto(&self) -> Self::Proto {
        let end_str = self.end_date.to_field();
        let wts_str = self.what_to_show.to_field();
        proto::HistoricalDataRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            end_date_time: some_str(&end_str),
            duration: Some(self.duration.to_field()),
            bar_size_setting: Some(self.bar_size.to_field()),
            what_to_show: some_str(&wts_str),
            use_rth: some_bool(self.use_rth),
            format_date: Some(DATE_FORMAT),
            keep_up_to_date: some_bool(self.keep_up_to_date),
            chart_options: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct HistoricalTicksRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub start: Option<OffsetDateTime>,
    pub end: Option<OffsetDateTime>,
    pub number_of_ticks: i32,
    pub what_to_show: HistoricalWhatToShow,
    pub use_rth: bool,
    pub ignore_size: bool,
}

impl Default for HistoricalTicksRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            start: None,
            end: None,
            number_of_ticks: 0,
            what_to_show: HistoricalWhatToShow::Trades,
            use_rth: false,
            ignore_size: false,
        }
    }
}

impl HistoricalTicksRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, v: &Contract) -> Self {
        self.contract = v.clone();
        self
    }
    pub fn start(mut self, v: Option<OffsetDateTime>) -> Self {
        self.start = v;
        self
    }
    pub fn end(mut self, v: Option<OffsetDateTime>) -> Self {
        self.end = v;
        self
    }
    pub fn number_of_ticks(mut self, v: i32) -> Self {
        self.number_of_ticks = v;
        self
    }
    pub fn what_to_show(mut self, v: HistoricalWhatToShow) -> Self {
        self.what_to_show = v;
        self
    }
    pub fn use_rth(mut self, v: bool) -> Self {
        self.use_rth = v;
        self
    }
    pub fn ignore_size(mut self, v: bool) -> Self {
        self.ignore_size = v;
        self
    }
}

impl RequestEncoder for HistoricalTicksRequestBuilder {
    type Proto = proto::HistoricalTicksRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestHistoricalTicks;

    fn to_proto(&self) -> Self::Proto {
        let start_str = self.start.to_field();
        let end_str = self.end.to_field();
        proto::HistoricalTicksRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            start_date_time: some_str(&start_str),
            end_date_time: some_str(&end_str),
            number_of_ticks: Some(self.number_of_ticks),
            what_to_show: Some(self.what_to_show.to_field()),
            use_rth: some_bool(self.use_rth),
            ignore_size: some_bool(self.ignore_size),
            misc_options: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct HistogramDataRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub use_rth: bool,
    pub period: BarSize,
}

impl Default for HistogramDataRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            use_rth: false,
            period: BarSize::Day,
        }
    }
}

impl HistogramDataRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, v: &Contract) -> Self {
        self.contract = v.clone();
        self
    }
    pub fn use_rth(mut self, v: bool) -> Self {
        self.use_rth = v;
        self
    }
    pub fn period(mut self, v: BarSize) -> Self {
        self.period = v;
        self
    }
}

impl RequestEncoder for HistogramDataRequestBuilder {
    type Proto = proto::HistogramDataRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestHistogramData;

    fn to_proto(&self) -> Self::Proto {
        proto::HistogramDataRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            use_rth: Some(self.use_rth),
            time_period: Some(self.period.to_field()),
        }
    }
}

// Cancel builders (cancel_historical_data, cancel_historical_ticks,
// cancel_histogram_data, cancel_head_timestamp) intentionally omitted: the
// production cancel paths fire from subscription drop handlers and have no
// per-test consumer that needs body verification.

// =============================================================================
// Realtime request builders
// =============================================================================

#[derive(Clone, Debug)]
pub struct RealtimeBarsRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub what_to_show: RealtimeWhatToShow,
    pub use_rth: bool,
    pub options: Vec<TagValue>,
}

impl Default for RealtimeBarsRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            what_to_show: RealtimeWhatToShow::Trades,
            use_rth: false,
            options: Vec::new(),
        }
    }
}

impl RealtimeBarsRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, v: &Contract) -> Self {
        self.contract = v.clone();
        self
    }
    pub fn what_to_show(mut self, v: RealtimeWhatToShow) -> Self {
        self.what_to_show = v;
        self
    }
    pub fn use_rth(mut self, v: bool) -> Self {
        self.use_rth = v;
        self
    }
    pub fn options(mut self, v: Vec<TagValue>) -> Self {
        self.options = v;
        self
    }
}

impl RequestEncoder for RealtimeBarsRequestBuilder {
    type Proto = proto::RealTimeBarsRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestRealTimeBars;

    fn to_proto(&self) -> Self::Proto {
        proto::RealTimeBarsRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            bar_size: Some(0),
            what_to_show: Some(self.what_to_show.to_string()),
            use_rth: some_bool(self.use_rth),
            real_time_bars_options: tag_values_to_map(&self.options),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TickByTickRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub tick_type: String,
    pub number_of_ticks: i32,
    pub ignore_size: bool,
}

impl Default for TickByTickRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            tick_type: String::new(),
            number_of_ticks: 0,
            ignore_size: false,
        }
    }
}

impl TickByTickRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, v: &Contract) -> Self {
        self.contract = v.clone();
        self
    }
    pub fn tick_type(mut self, v: impl Into<String>) -> Self {
        self.tick_type = v.into();
        self
    }
    pub fn number_of_ticks(mut self, v: i32) -> Self {
        self.number_of_ticks = v;
        self
    }
    pub fn ignore_size(mut self, v: bool) -> Self {
        self.ignore_size = v;
        self
    }
}

impl RequestEncoder for TickByTickRequestBuilder {
    type Proto = proto::TickByTickRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestTickByTickData;

    fn to_proto(&self) -> Self::Proto {
        proto::TickByTickRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            tick_type: some_str(&self.tick_type),
            number_of_ticks: Some(self.number_of_ticks),
            ignore_size: some_bool(self.ignore_size),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MarketDepthRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub number_of_rows: i32,
    pub is_smart_depth: bool,
}

impl Default for MarketDepthRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            number_of_rows: 0,
            is_smart_depth: false,
        }
    }
}

impl MarketDepthRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, v: &Contract) -> Self {
        self.contract = v.clone();
        self
    }
    pub fn number_of_rows(mut self, v: i32) -> Self {
        self.number_of_rows = v;
        self
    }
    pub fn smart_depth(mut self, v: bool) -> Self {
        self.is_smart_depth = v;
        self
    }
}

impl RequestEncoder for MarketDepthRequestBuilder {
    type Proto = proto::MarketDepthRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestMarketDepth;

    fn to_proto(&self) -> Self::Proto {
        proto::MarketDepthRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            num_rows: Some(self.number_of_rows),
            is_smart_depth: some_bool(self.is_smart_depth),
            market_depth_options: Default::default(),
        }
    }
}

empty_request_builder!(
    MarketDepthExchangesRequestBuilder,
    MarketDepthExchangesRequest,
    OutgoingMessages::RequestMktDepthExchanges
);

#[derive(Clone, Debug)]
pub struct MarketDataRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub generic_ticks: Vec<String>,
    pub snapshot: bool,
    pub regulatory_snapshot: bool,
}

impl Default for MarketDataRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            generic_ticks: Vec::new(),
            snapshot: false,
            regulatory_snapshot: false,
        }
    }
}

impl MarketDataRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, v: &Contract) -> Self {
        self.contract = v.clone();
        self
    }
    pub fn generic_ticks<S: AsRef<str>>(mut self, v: &[S]) -> Self {
        self.generic_ticks = v.iter().map(|s| s.as_ref().to_string()).collect();
        self
    }
    pub fn snapshot(mut self, v: bool) -> Self {
        self.snapshot = v;
        self
    }
    pub fn regulatory_snapshot(mut self, v: bool) -> Self {
        self.regulatory_snapshot = v;
        self
    }
}

impl RequestEncoder for MarketDataRequestBuilder {
    type Proto = proto::MarketDataRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestMarketData;

    fn to_proto(&self) -> Self::Proto {
        let joined = self.generic_ticks.join(",");
        proto::MarketDataRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            generic_tick_list: some_str(&joined),
            snapshot: some_bool(self.snapshot),
            regulatory_snapshot: some_bool(self.regulatory_snapshot),
            market_data_options: Default::default(),
        }
    }
}

// Cancel builders (cancel_realtime_bars, cancel_tick_by_tick,
// cancel_market_depth, cancel_market_data) intentionally omitted: same
// rationale as the historical cancels above.

// =============================================================================
// Entry-point functions
// =============================================================================

pub fn head_timestamp_request() -> HeadTimestampRequestBuilder {
    HeadTimestampRequestBuilder::default()
}

pub fn historical_data_request() -> HistoricalDataRequestBuilder {
    HistoricalDataRequestBuilder::default()
}

pub fn historical_ticks_request() -> HistoricalTicksRequestBuilder {
    HistoricalTicksRequestBuilder::default()
}

pub fn histogram_data_request() -> HistogramDataRequestBuilder {
    HistogramDataRequestBuilder::default()
}

pub fn realtime_bars_request() -> RealtimeBarsRequestBuilder {
    RealtimeBarsRequestBuilder::default()
}

pub fn tick_by_tick_request() -> TickByTickRequestBuilder {
    TickByTickRequestBuilder::default()
}

pub fn market_depth_request() -> MarketDepthRequestBuilder {
    MarketDepthRequestBuilder::default()
}

pub fn market_depth_exchanges_request() -> MarketDepthExchangesRequestBuilder {
    MarketDepthExchangesRequestBuilder
}

pub fn market_data_request() -> MarketDataRequestBuilder {
    MarketDataRequestBuilder::default()
}

// =============================================================================
// Realtime response builders (RealTimeBars, TickByTick, MarketDepth,
// TickPrice/Size/String/Generic) — pair with `proto_response()` in tests.
// =============================================================================

#[derive(Clone, Debug)]
pub struct RealTimeBarTickResponse {
    pub request_id: i32,
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub wap: f64,
    pub count: i32,
}

impl Default for RealTimeBarTickResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            time: 0,
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            volume: 0.0,
            wap: 0.0,
            count: 0,
        }
    }
}

impl RealTimeBarTickResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn time(mut self, v: i64) -> Self {
        self.time = v;
        self
    }
    pub fn ohlc(mut self, open: f64, high: f64, low: f64, close: f64) -> Self {
        self.open = open;
        self.high = high;
        self.low = low;
        self.close = close;
        self
    }
    pub fn volume(mut self, v: f64) -> Self {
        self.volume = v;
        self
    }
    pub fn wap(mut self, v: f64) -> Self {
        self.wap = v;
        self
    }
    pub fn count(mut self, v: i32) -> Self {
        self.count = v;
        self
    }
}

impl ResponseProtoEncoder for RealTimeBarTickResponse {
    type Proto = proto::RealTimeBarTick;

    fn to_proto(&self) -> Self::Proto {
        proto::RealTimeBarTick {
            req_id: Some(self.request_id),
            time: Some(self.time),
            open: Some(self.open),
            high: Some(self.high),
            low: Some(self.low),
            close: Some(self.close),
            volume: Some(self.volume.to_string()),
            wap: Some(self.wap.to_string()),
            count: Some(self.count),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TradeTickResponse {
    pub request_id: i32,
    pub tick_type: i32,
    pub time: i64,
    pub price: f64,
    pub size: f64,
    pub past_limit: bool,
    pub unreported: bool,
    pub exchange: String,
    pub special_conditions: String,
}

impl Default for TradeTickResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            tick_type: 1,
            time: 0,
            price: 0.0,
            size: 0.0,
            past_limit: false,
            unreported: false,
            exchange: String::new(),
            special_conditions: String::new(),
        }
    }
}

impl TradeTickResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn tick_type(mut self, v: i32) -> Self {
        self.tick_type = v;
        self
    }
    pub fn time(mut self, v: i64) -> Self {
        self.time = v;
        self
    }
    pub fn price(mut self, v: f64) -> Self {
        self.price = v;
        self
    }
    pub fn size(mut self, v: f64) -> Self {
        self.size = v;
        self
    }
    pub fn attributes(mut self, past_limit: bool, unreported: bool) -> Self {
        self.past_limit = past_limit;
        self.unreported = unreported;
        self
    }
    pub fn exchange(mut self, v: impl Into<String>) -> Self {
        self.exchange = v.into();
        self
    }
    pub fn special_conditions(mut self, v: impl Into<String>) -> Self {
        self.special_conditions = v.into();
        self
    }
}

impl ResponseProtoEncoder for TradeTickResponse {
    type Proto = proto::TickByTickData;

    fn to_proto(&self) -> Self::Proto {
        proto::TickByTickData {
            req_id: Some(self.request_id),
            tick_type: Some(self.tick_type),
            tick: Some(proto::tick_by_tick_data::Tick::HistoricalTickLast(proto::HistoricalTickLast {
                time: Some(self.time),
                tick_attrib_last: Some(proto::TickAttribLast {
                    past_limit: Some(self.past_limit),
                    unreported: Some(self.unreported),
                }),
                price: Some(self.price),
                size: Some(self.size.to_string()),
                exchange: some_str(&self.exchange),
                special_conditions: some_str(&self.special_conditions),
            })),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BidAskTickResponse {
    pub request_id: i32,
    pub time: i64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_size: f64,
    pub ask_size: f64,
    pub bid_past_low: bool,
    pub ask_past_high: bool,
}

impl Default for BidAskTickResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            time: 0,
            bid_price: 0.0,
            ask_price: 0.0,
            bid_size: 0.0,
            ask_size: 0.0,
            bid_past_low: false,
            ask_past_high: false,
        }
    }
}

impl BidAskTickResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn time(mut self, v: i64) -> Self {
        self.time = v;
        self
    }
    pub fn quote(mut self, bid_price: f64, ask_price: f64, bid_size: f64, ask_size: f64) -> Self {
        self.bid_price = bid_price;
        self.ask_price = ask_price;
        self.bid_size = bid_size;
        self.ask_size = ask_size;
        self
    }
    pub fn attributes(mut self, bid_past_low: bool, ask_past_high: bool) -> Self {
        self.bid_past_low = bid_past_low;
        self.ask_past_high = ask_past_high;
        self
    }
}

impl ResponseProtoEncoder for BidAskTickResponse {
    type Proto = proto::TickByTickData;

    fn to_proto(&self) -> Self::Proto {
        proto::TickByTickData {
            req_id: Some(self.request_id),
            tick_type: Some(3),
            tick: Some(proto::tick_by_tick_data::Tick::HistoricalTickBidAsk(proto::HistoricalTickBidAsk {
                time: Some(self.time),
                tick_attrib_bid_ask: Some(proto::TickAttribBidAsk {
                    bid_past_low: Some(self.bid_past_low),
                    ask_past_high: Some(self.ask_past_high),
                }),
                price_bid: Some(self.bid_price),
                price_ask: Some(self.ask_price),
                size_bid: Some(self.bid_size.to_string()),
                size_ask: Some(self.ask_size.to_string()),
            })),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MidPointTickResponse {
    pub request_id: i32,
    pub time: i64,
    pub mid_point: f64,
}

impl Default for MidPointTickResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            time: 0,
            mid_point: 0.0,
        }
    }
}

impl MidPointTickResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn time(mut self, v: i64) -> Self {
        self.time = v;
        self
    }
    pub fn mid_point(mut self, v: f64) -> Self {
        self.mid_point = v;
        self
    }
}

impl ResponseProtoEncoder for MidPointTickResponse {
    type Proto = proto::TickByTickData;

    fn to_proto(&self) -> Self::Proto {
        proto::TickByTickData {
            req_id: Some(self.request_id),
            tick_type: Some(4),
            tick: Some(proto::tick_by_tick_data::Tick::HistoricalTickMidPoint(proto::HistoricalTick {
                time: Some(self.time),
                price: Some(self.mid_point),
                size: Some("0".into()),
            })),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MarketDepthResponse {
    pub request_id: i32,
    pub position: i32,
    pub operation: i32,
    pub side: i32,
    pub price: f64,
    pub size: f64,
}

impl Default for MarketDepthResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            position: 0,
            operation: 0,
            side: 0,
            price: 0.0,
            size: 0.0,
        }
    }
}

impl MarketDepthResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn position(mut self, v: i32) -> Self {
        self.position = v;
        self
    }
    pub fn operation(mut self, v: i32) -> Self {
        self.operation = v;
        self
    }
    pub fn side(mut self, v: i32) -> Self {
        self.side = v;
        self
    }
    pub fn price(mut self, v: f64) -> Self {
        self.price = v;
        self
    }
    pub fn size(mut self, v: f64) -> Self {
        self.size = v;
        self
    }
}

impl ResponseProtoEncoder for MarketDepthResponse {
    type Proto = proto::MarketDepth;

    fn to_proto(&self) -> Self::Proto {
        proto::MarketDepth {
            req_id: Some(self.request_id),
            market_depth_data: Some(proto::MarketDepthData {
                position: Some(self.position),
                operation: Some(self.operation),
                side: Some(self.side),
                price: Some(self.price),
                size: Some(self.size.to_string()),
                market_maker: None,
                is_smart_depth: None,
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TickPriceResponse {
    pub request_id: i32,
    pub tick_type: i32,
    pub price: f64,
    pub size: Option<f64>,
    pub attr_mask: i32,
}

impl Default for TickPriceResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            tick_type: 0,
            price: 0.0,
            size: None,
            attr_mask: 0,
        }
    }
}

impl TickPriceResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn tick_type(mut self, v: i32) -> Self {
        self.tick_type = v;
        self
    }
    pub fn price(mut self, v: f64) -> Self {
        self.price = v;
        self
    }
    pub fn size(mut self, v: f64) -> Self {
        self.size = Some(v);
        self
    }
    pub fn attr_mask(mut self, v: i32) -> Self {
        self.attr_mask = v;
        self
    }
}

impl ResponseProtoEncoder for TickPriceResponse {
    type Proto = proto::TickPrice;

    fn to_proto(&self) -> Self::Proto {
        proto::TickPrice {
            req_id: Some(self.request_id),
            tick_type: Some(self.tick_type),
            price: Some(self.price),
            size: self.size.map(|s| s.to_string()),
            attr_mask: Some(self.attr_mask),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TickSizeResponse {
    pub request_id: i32,
    pub tick_type: i32,
    pub size: f64,
}

impl Default for TickSizeResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            tick_type: 0,
            size: 0.0,
        }
    }
}

impl TickSizeResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn tick_type(mut self, v: i32) -> Self {
        self.tick_type = v;
        self
    }
    pub fn size(mut self, v: f64) -> Self {
        self.size = v;
        self
    }
}

impl ResponseProtoEncoder for TickSizeResponse {
    type Proto = proto::TickSize;

    fn to_proto(&self) -> Self::Proto {
        proto::TickSize {
            req_id: Some(self.request_id),
            tick_type: Some(self.tick_type),
            size: Some(self.size.to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TickStringResponse {
    pub request_id: i32,
    pub tick_type: i32,
    pub value: String,
}

impl Default for TickStringResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            tick_type: 0,
            value: String::new(),
        }
    }
}

impl TickStringResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn tick_type(mut self, v: i32) -> Self {
        self.tick_type = v;
        self
    }
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = v.into();
        self
    }
}

impl ResponseProtoEncoder for TickStringResponse {
    type Proto = proto::TickString;

    fn to_proto(&self) -> Self::Proto {
        proto::TickString {
            req_id: Some(self.request_id),
            tick_type: Some(self.tick_type),
            value: some_str(&self.value),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TickGenericResponse {
    pub request_id: i32,
    pub tick_type: i32,
    pub value: f64,
}

impl Default for TickGenericResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            tick_type: 0,
            value: 0.0,
        }
    }
}

impl TickGenericResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn tick_type(mut self, v: i32) -> Self {
        self.tick_type = v;
        self
    }
    pub fn value(mut self, v: f64) -> Self {
        self.value = v;
        self
    }
}

impl ResponseProtoEncoder for TickGenericResponse {
    type Proto = proto::TickGeneric;

    fn to_proto(&self) -> Self::Proto {
        proto::TickGeneric {
            req_id: Some(self.request_id),
            tick_type: Some(self.tick_type),
            value: Some(self.value),
        }
    }
}

pub fn realtime_bar_tick() -> RealTimeBarTickResponse {
    RealTimeBarTickResponse::default()
}

pub fn trade_tick() -> TradeTickResponse {
    TradeTickResponse::default()
}

pub fn bid_ask_tick() -> BidAskTickResponse {
    BidAskTickResponse::default()
}

pub fn mid_point_tick() -> MidPointTickResponse {
    MidPointTickResponse::default()
}

pub fn market_depth_response() -> MarketDepthResponse {
    MarketDepthResponse::default()
}

pub fn tick_price() -> TickPriceResponse {
    TickPriceResponse::default()
}

pub fn tick_size() -> TickSizeResponse {
    TickSizeResponse::default()
}

pub fn tick_string() -> TickStringResponse {
    TickStringResponse::default()
}

pub fn tick_generic() -> TickGenericResponse {
    TickGenericResponse::default()
}
