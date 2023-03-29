use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::domain::TickAttribBidAsk;
use crate::messages::{RequestMessage, ResponseMessage};
use crate::server_versions;
use crate::{Client, Error};

// https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/EClient.cs
// https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/EDecoder.cs#L733

/// Returns the timestamp of earliest available historical data for a contract and data type.
/// ```no_run
///     use anyhow::Result;
///     use ibapi::Client;
///     use ibapi::contracts::{self, Contract};
///     //use ibapi::market_data::historical;
///
///     fn main() -> Result<()> {
///         let mut client = Client::connect("localhost:4002")?;
///
///         let contract = Contract::stock("MSFT");
///         let what_to_show = "trades";
///         let use_rth = true;
///
///         //let result =
///         //    historical::head_timestamp(&mut client, &contract, what_to_show, use_rth);
///
///         //print!("head_timestamp: {result:?}");
///         Ok(())
///     }
/// ```
pub fn head_timestamp(client: &mut Client, contract: &Contract, what_to_show: &str, use_rth: bool) -> Result<OffsetDateTime, Error> {
    client.check_server_version(server_versions::REQ_HEAD_TIMESTAMP, "It does not support head time stamp requests.")?;

    let request_id = client.next_request_id();
    let request = encode_head_timestamp(request_id, contract, what_to_show, use_rth)?;

    let mut promise = client.send_request(request_id, request)?;

    if let Some(mut response) = promise.next() {
        decode_head_timestamp(&mut response)
    } else {
        Err(Error::Simple("did not receive head timestamp message".into()))
    }
}

/// Encodes the head timestamp request
pub(crate) fn encode_head_timestamp(request_id: i32, contract: &Contract, what_to_show: &str, use_rth: bool) -> Result<RequestMessage, Error> {
    let mut packet = RequestMessage::default();

    packet.push_field(&12);
    packet.push_field(&request_id);
    contract.push_fields(&mut packet);
    packet.push_field(&use_rth);
    packet.push_field(&what_to_show);
    packet.push_field(&"format_date");

    // source.AddParameter(value.ConId);
    // source.AddParameter(value.Symbol);
    // source.AddParameter(value.SecType);
    // source.AddParameter(value.LastTradeDateOrContractMonth);
    // source.AddParameter(value.Strike);
    // source.AddParameter(value.Right);
    // source.AddParameter(value.Multiplier);
    // source.AddParameter(value.Exchange);
    // source.AddParameter(value.PrimaryExch);
    // source.AddParameter(value.Currency);
    // source.AddParameter(value.LocalSymbol);
    // source.AddParameter(value.TradingClass);
    // source.AddParameter(value.IncludeExpired);

    Ok(packet)
}

// https://github.com/InteractiveBrokers/tws-api/blob/313c453bfc1a1f8928b0d2fba044947f4c37e380/source/csharpclient/client/IBParamsList.cs#L56

// public static void AddParameter(this BinaryWriter source, Contract value)
// {
//     source.AddParameter(value.ConId);
//     source.AddParameter(value.Symbol);
//     source.AddParameter(value.SecType);
//     source.AddParameter(value.LastTradeDateOrContractMonth);
//     source.AddParameter(value.Strike);
//     source.AddParameter(value.Right);
//     source.AddParameter(value.Multiplier);
//     source.AddParameter(value.Exchange);
//     source.AddParameter(value.PrimaryExch);
//     source.AddParameter(value.Currency);
//     source.AddParameter(value.LocalSymbol);
//     source.AddParameter(value.TradingClass);
//     source.AddParameter(value.IncludeExpired);
// }

fn decode_head_timestamp(packet: &mut ResponseMessage) -> Result<OffsetDateTime, Error> {
    let _request_id = packet.next_int()?;
    let head_timestamp = packet.next_date_time()?;

    Ok(head_timestamp)
}

/// Returns data histogram of specified contract
pub fn histogram_data(client: &Client, contract: &Contract, use_rth: bool, period: &str) -> Result<HistogramDataIterator, Error> {
    // " S (seconds) - " D (days)
    // " W (weeks) - " M (months)
    // " Y (years)
    print!("{client:?} {contract:?} {use_rth:?} {period:?}");
    Err(Error::NotImplemented)
}

#[allow(clippy::too_many_arguments)]
pub fn historical_data(
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

pub fn historical_schedule(client: &Client, contract: &Contract, use_rth: bool, period: &str) -> Result<HistogramDataIterator, Error> {
    print!("{client:?} {contract:?} {use_rth:?} {period:?}");
    Err(Error::NotImplemented)
}

pub fn historical_ticks(
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

pub fn historical_ticks_bid_ask(
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

pub fn historical_ticks_last(
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

#[derive(Default)]
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
    pub time: OffsetDateTime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub wap: f64,
    pub count: i32,
}

pub struct BarIterator {}
// https://interactivebrokers.github.io/tws-api/classIBApi_1_1Bar.html

pub struct HistoricalSchedule {
    //    string startDateTime, string endDateTime, string timeZone, HistoricalSession[]
}

#[cfg(test)]
mod tests;
