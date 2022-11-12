use anyhow::{anyhow, Result};
use time::OffsetDateTime;

use crate::client::{Client, Packet};
use crate::domain::Contract;
use crate::domain::TickAttribBidAsk;

struct RequestHeadTimestamp<'a> {
    request_id: i32,
    contract: &'a Contract,
    what_to_show: &'a str,
    use_rth: bool,
}

impl RequestHeadTimestamp<'_> {
    fn encode(&self) -> Result<Packet> {
        Err(anyhow!("not implemented!"))
    }
}

struct ReceiveHeadTimestamp {
}

impl ReceiveHeadTimestamp {
    fn decode(packet: &Packet) -> Result<ReceiveHeadTimestamp> {
        Err(anyhow!("not implemented!"))
    }
}

/// Returns the timestamp of earliest available historical data for a contract and data type.
pub fn head_timestamp<C: Client>(client: &C, contract: &Contract, what_to_show: &str, use_rth: bool) -> Result<OffsetDateTime> {
    let request = RequestHeadTimestamp{
        request_id: client.next_request_id(),
        contract: contract,
        what_to_show: what_to_show,
        use_rth: use_rth
    };

    client.send_packet(&request.encode()?);
    let packet = client.receive_packet(request.request_id);

    ReceiveHeadTimestamp::decode(&packet);

    Err(anyhow!("not implemented!"))
}

/// Returns data histogram of specified contract
pub fn histogram_data<C: Client>(client: &C, contract: &Contract, use_rth: bool, period: &str) -> Result<HistogramDataIterator> {
    // " S (seconds) - " D (days)
    // " W (weeks) - " M (months)
    // " Y (years)
    Err(anyhow!("not implemented!"))
}

pub fn historical_data<C: Client>(client: &C, contract: &Contract, end: &OffsetDateTime, duration: &str, bar_size: &str, what_to_show: &str, use_rth: bool, keep_up_to_date: bool) -> Result<BarIterator> {
/// https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_duration
/// https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_barsize
/// https://interactivebrokers.github.io/tws-api/historical_bars.html#hd_what_to_show
    Err(anyhow!("not implemented!"))
}

pub fn historical_schedule<C: Client>(client: &C, contract: &Contract, use_rth: bool, period: &str) -> Result<HistogramDataIterator> {
    Err(anyhow!("not implemented!"))
}

pub fn historical_ticks<C: Client>(client: &C, contract: &Contract, start_date: Option<OffsetDateTime>, end_date: Option<OffsetDateTime>, number_of_ticks: i32, use_rth: i32, ignore_size: bool) -> Result<HistoricalTickIterator> {
    Err(anyhow!("not implemented!"))
}

pub fn historical_ticks_bid_ask<C: Client>(client: &C, contract: &Contract, start_date: Option<OffsetDateTime>, end_date: Option<OffsetDateTime>, number_of_ticks: i32, use_rth: i32, ignore_size: bool) -> Result<HistoricalTickBidAskIterator> {
    Err(anyhow!("not implemented!"))
}

pub fn historical_ticks_last<C: Client>(client: &C, contract: &Contract, start_date: Option<OffsetDateTime>, end_date: Option<OffsetDateTime>, number_of_ticks: i32, use_rth: i32, ignore_size: bool) -> Result<HistoricalTickLastIterator> {
    Err(anyhow!("not implemented!"))
}

pub struct HistoricalTick {
    pub time: i32,
    pub price: f64,
    pub size: i32,
}

pub struct HistoricalTickBidAsk {
    pub time: i32,
    pub tick_attrib_bid_ask: TickAttribBidAsk,
    pub price_bid: f64,
    pub price_ask: f64,
    pub size_bid: i32,
    pub size_ask: i32,
}

pub struct HistoricalTickLast {
    pub time: i32,
    pub price: f64,
    pub size: i32,
}

pub struct HistoricalTickIterator {}

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

pub struct HistoricalTickBidAskIterator {}

pub struct HistoricalTickLastIterator {}

pub struct HistogramData {}
pub struct HistogramDataIterator {}

pub struct Bar {
    time: OffsetDateTime,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    wap: f64,
    count: i32,
}

pub struct BarIterator {}
/// https://interactivebrokers.github.io/tws-api/classIBApi_1_1Bar.html

pub struct HistoricalSchedule {
//    string startDateTime, string endDateTime, string timeZone, HistoricalSession[]
}
