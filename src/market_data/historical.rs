use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::domain::TickAttribBidAsk;
use crate::messages::{RequestMessage, ResponseMessage};
use crate::server_versions;
use crate::{Client, Error};

pub use super::WhatToShow;

mod decoders;
mod encoders;
#[cfg(test)]
mod tests;

struct Bar {
    pub time: OffsetDateTime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub wap: f64,
    pub count: i32,
}

struct HistogramData {}

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

// https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/EClient.cs
// https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/EDecoder.cs#L733

// Returns the timestamp of earliest available historical data for a contract and data type.
pub fn head_timestamp(client: &Client, contract: &Contract, what_to_show: WhatToShow, use_rth: bool) -> Result<OffsetDateTime, Error> {
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

#[allow(clippy::too_many_arguments)]
fn historical_data(
    client: &Client,
    contract: &Contract,
    end: &OffsetDateTime,
    duration: &str,
    bar_size: &str,
    what_to_show: &str,
    use_rth: bool,
    keep_up_to_date: bool,
) -> Result<BarIterator, Error> {
    // https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_duration
    // https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_barsize
    // https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_what_to_show
    print!("{client:?} {contract:?} {end:?} {duration:?} {bar_size:?} {what_to_show:?} {use_rth:?} {keep_up_to_date:?}");

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

struct BarIterator {}
// https://interactivebrokers.github.io/tws-api/classIBApi_1_1Bar.html
