use time::OffsetDateTime;

/// Bar describes the historical data bar.
pub struct Bar {
    /// The bar's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// The bar's open price.
    pub open: f64,
    /// The bar's high price.
    pub high: f64,
    /// The bar's low price.
    pub low: f64,
    /// The bar's close price.
    pub close: f64,
    /// The bar's traded volume if available (only available for TRADES)
    pub volume: i64,
    /// The bar's Weighted Average Price (only available for TRADES)
    pub wap: f64,
    /// The number of trades during the bar's timespan (only available for TRADES)
    pub count: i32,
}

pub struct Trade {
    /// Tick type: "Last" or "AllLast"
    pub tick_type: String,
    /// The trade's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// Tick last price
    pub price: f64,
    /// Tick last size
    pub size: i64,
    /// Tick attribs (bit 0 - past limit, bit 1 - unreported)
    pub trade_attribute: TradeAttribute,
    /// Tick exchange
    pub exchange: String,
    /// Tick special conditions
    pub special_conditions: String,
}

pub struct TradeAttribute {
    pub past_limit: bool,
    pub unreported: bool,
}

pub struct BidAsk {
    /// The spread's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// tick-by-tick real-time tick bid price
    pub bid_price: f64,
    /// tick-by-tick real-time tick ask price
    pub ask_price: f64,
    /// tick-by-tick real-time tick bid size
    pub bid_size: i64,
    /// tick-by-tick real-time tick ask size
    pub ask_size: i64,
    /// tick-by-tick real-time bid/ask tick attribs (bit 0 - bid past low, bit 1 - ask past high)
    pub bid_ask_attribute: BidAskAttribute,
}

pub struct BidAskAttribute {
    pub bid_past_low: bool,
    pub ask_past_high: bool,
}

pub struct HistogramData {
    pub price: f64,
    pub count: i32,
}

pub struct DepthMktDataDescription {
    pub exchange: String,
    pub sec_type: String,
    pub listing_exch: String,
    pub service_data_type: String,
    pub agg_group: i32,
}

pub struct SmartComponent {
    pub bit_number: i32,
    pub exchange: String,
    pub exchange_letter: String,
}

pub struct TickAttrib {
    pub can_auto_execute: bool,
    pub past_limit: bool,
    pub pre_open: bool,
}

pub struct TickAttribBidAsk {
    pub bid_past_low: bool,
    pub ask_past_high: bool,
}

pub struct TickAttribLast {
    pub past_limit: bool,
    pub unreported: bool,
}

pub struct FamilyCode {
    pub account_id: String,
    pub family_code_str: String,
}

pub struct PriceIncrement {
    pub low_edge: f64,
    pub increment: f64,
}

#[derive(Clone, Debug)]
pub struct NewsProvider {
    pub code: String,
    pub name: String,
}

pub enum ComboParam {
    NonGuaranteed,
    PriceCondConid,
    CondPriceMax,
    CondPriceMin,
    ChangeToMktTime1,
    ChangeToMktTime2,
    DiscretionaryPct,
    DontLeginNext,
    LeginPrio,
    MaxSegSize,
}

pub enum HedgeType {
    None,
    Delta,
    Beta,
    Fx,
    Pair,
}

pub enum Right {
    None,
    Put,
    Call,
}

pub enum VolatilityType {
    None,
    Daily,
    Annual,
}

pub enum ReferencePriceType {
    None,
    Midpoint,
    BidOrAsk,
}
