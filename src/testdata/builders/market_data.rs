//! Builders for market-data domain request messages.
//!
//! Covers both `historical` and `realtime` submodules. Response builders are
//! intentionally absent: market-data responses use IB's per-message text wire
//! format (no protobuf), and the existing inline literals in
//! `{historical,realtime}/{sync,async}/tests.rs` already exercise the
//! production decoders end-to-end.

use super::RequestEncoder;
use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::contracts::Contract;
use crate::market_data::historical::{BarSize, Duration, WhatToShow as HistoricalWhatToShow};
use crate::market_data::realtime::WhatToShow as RealtimeWhatToShow;
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::proto::encoders::{encode_contract, some_bool, some_str};
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
}

impl Default for RealtimeBarsRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            what_to_show: RealtimeWhatToShow::Trades,
            use_rth: false,
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
            real_time_bars_options: Default::default(),
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
