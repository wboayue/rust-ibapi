use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::domain::TickAttribBidAsk;
use crate::messages::{RequestMessage, ResponseMessage};
use crate::{server_versions, ToField};
use crate::{Client, Error};

mod decoders;
mod encoders;
#[cfg(test)]
mod tests;

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

#[derive(Clone, Debug, Copy)]
pub enum BarSize {
    Sec,
    Sec5,
    Sec15,
    Sec30,
    Min,
    Min2,
    Min3,
    Min5,
    Min15,
    Min30,
    Hour,
    Day,
}

impl ToString for BarSize {
    fn to_string(&self) -> String {
        match self {
            Self::Sec => "TRADES".into(),
            _ => "SCHEDULE".into(),
        }
    }
}

impl ToField for BarSize {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

#[derive(Debug)]
struct HistogramData {
    pub price: f64,
    pub count: i32,
}

struct HistoricalSchedule {
    //    string startDateTime, string endDateTime, string timeZone, HistoricalSession[]
}

struct HistoricalTick {
    pub time: i32,
    pub price: f64,
    pub size: i32,
}

struct HistoricalTickBidAsk {
    pub time: i32,
    pub tick_attrib_bid_ask: TickAttribBidAsk,
    pub price_bid: f64,
    pub price_ask: f64,
    pub size_bid: i32,
    pub size_ask: i32,
}

struct HistoricalTickLast {
    pub time: i32,
    pub price: f64,
    pub size: i32,
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum WhatToShow {
    Trades,
    MidPoint,
    Bid,
    Ask,
    BidAsk,
    HistoriclVolatility,
    OptionImpliedVolatility,
    FeeRate,
    Schedule,
}

impl ToString for WhatToShow {
    fn to_string(&self) -> String {
        match self {
            Self::Trades => "TRADES".to_string(),
            Self::MidPoint => "MIDPOINT".to_string(),
            Self::Bid => "BID".to_string(),
            Self::Ask => "ASK".to_string(),
            Self::BidAsk => "BID_ASK".to_string(),
            Self::HistoriclVolatility => "HISTORICAL_VOLATILITY".to_string(),
            Self::OptionImpliedVolatility => "OPTION_IMPLIED_VOLATILITY".to_string(),
            Self::FeeRate => "FEE_RATE".to_string(),
            Self::Schedule => "SCHEDULE".to_string(),
        }
    }
}

impl ToField for WhatToShow {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<WhatToShow> {
    fn to_field(&self) -> String {
        match self {
            Some(what_to_show) => what_to_show.to_string(),
            None => "".into(),
        }
    }
}

// Returns the timestamp of earliest available historical data for a contract and data type.
pub(crate) fn head_timestamp(client: &Client, contract: &Contract, what_to_show: WhatToShow, use_rth: bool) -> Result<OffsetDateTime, Error> {
    client.check_server_version(server_versions::REQ_HEAD_TIMESTAMP, "It does not support head time stamp requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_head_timestamp(request_id, contract, what_to_show, use_rth)?;

    let mut messages = client.send_request(request_id, request)?;

    if let Some(mut message) = messages.next() {
        decoders::decode_head_timestamp(&mut message)
    } else {
        Err(Error::Simple("did not receive head timestamp message".into()))
    }
}

/// Returns data histogram of specified contract
fn histogram_data(client: &Client, contract: &Contract, use_rth: bool, period: &str) -> Result<HistogramDataIterator, Error> {
    // " S (seconds) - " D (days)
    // " W (weeks) - " M (months)
    // " Y (years)
    print!("{client:?} {contract:?} {use_rth:?} {period:?}");
    Err(Error::NotImplemented)
}

pub(crate) fn historical_data(
    client: &Client,
    contract: &Contract,
    start_date: &OffsetDateTime,
    end_date: &OffsetDateTime,
    bar_size: BarSize,
    what_to_show: Option<WhatToShow>,
    use_rth: bool,
) -> Result<BarIterator, Error> {
    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support contract_id nor trading class parameters when requesting historical data.",
        )?;
    }

    if what_to_show == Some(WhatToShow::Schedule) {
        client.check_server_version(
            server_versions::HISTORICAL_SCHEDULE,
            "It does not support requesting of historical schedule.",
        )?;
    }

    let request_id = client.next_request_id();
    let request = encoders::encode_historical_data(
        client.server_version(),
        request_id,
        contract,
        start_date,
        end_date,
        bar_size,
        what_to_show,
        use_rth,
        false,
    )?;

    let mut messages = client.send_request(request_id, request)?;

    // https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_duration
    // https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_barsize
    // https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_what_to_show
    print!("{client:?} {contract:?} {end_date:?} {start_date:?} {bar_size:?} {what_to_show:?} {use_rth:?}");

    Err(Error::NotImplemented)
}

fn historical_schedule(client: &Client, contract: &Contract, use_rth: bool, period: &str) -> Result<HistogramDataIterator, Error> {
    print!("{client:?} {contract:?} {use_rth:?} {period:?}");
    Err(Error::NotImplemented)
}

fn historical_ticks(
    client: &Client,
    contract: &Contract,
    start_date: Option<OffsetDateTime>,
    end_date: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: i32,
    ignore_size: bool,
) -> Result<HistoricalTickIterator, Error> {
    print!("{client:?} {contract:?} {start_date:?} {end_date:?} {number_of_ticks:?} {use_rth:?} {ignore_size:?}");
    Err(Error::NotImplemented)
}

fn historical_ticks_bid_ask(
    client: &Client,
    contract: &Contract,
    start_date: Option<OffsetDateTime>,
    end_date: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: i32,
    ignore_size: bool,
) -> Result<HistoricalTickBidAskIterator, Error> {
    print!("{client:?} {contract:?} {start_date:?} {end_date:?} {number_of_ticks:?} {use_rth:?} {ignore_size:?}");

    Err(Error::NotImplemented)
}

fn historical_ticks_last(
    client: &Client,
    contract: &Contract,
    start_date: Option<OffsetDateTime>,
    end_date: Option<OffsetDateTime>,
    number_of_ticks: i32,
    use_rth: i32,
    ignore_size: bool,
) -> Result<HistoricalTickLastIterator, Error> {
    print!("{client:?} {contract:?} {start_date:?} {end_date:?} {number_of_ticks:?} {use_rth:?} {ignore_size:?}");
    Err(Error::NotImplemented)
}

#[derive(Default)]
struct HistoricalTickIterator {}

impl HistoricalTickIterator {
    pub fn new() -> HistoricalTickIterator {
        HistoricalTickIterator {}
    }
}

impl Iterator for HistoricalTickIterator {
    // we will be counting with usize
    type Item = HistoricalTick;

    // next() is the only required method
    fn next(&mut self) -> Option<HistoricalTick> {
        None
    }
}

struct HistoricalTickBidAskIterator {}

struct HistoricalTickLastIterator {}

struct HistogramDataIterator {}

pub(crate) struct BarIterator {}
// https://interactivebrokers.github.io/tws-api/classIBApi_1_1Bar.html

impl Iterator for BarIterator {
    // we will be counting with usize
    type Item = Bar;

    // next() is the only required method
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
