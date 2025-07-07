use log::debug;

use crate::client::{ResponseContext, Subscription};
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::orders::TagValue;
use crate::server_versions;
use crate::{Client, Error};

use super::common::{decoders, encoders};
use super::{BarSize, BidAsk, DepthMarketDataDescription, MarketDepths, MidPoint, TickTypes, Trade, WhatToShow, Bar};

// Requests realtime bars.
pub(crate) fn realtime_bars<'a>(
    client: &'a Client,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_rth: bool,
    options: Vec<TagValue>,
) -> Result<Subscription<'a, Bar>, Error> {
    let request_id = client.next_request_id();
    let request = encoders::encode_request_realtime_bars(client.server_version(), request_id, contract, bar_size, what_to_show, use_rth, options)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests tick by tick AllLast ticks.
pub(crate) fn tick_by_tick_all_last<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<'a, Trade>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let request = encoders::encode_tick_by_tick(server_version, request_id, contract, "AllLast", number_of_ticks, ignore_size)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Validates that server supports the given request.
pub(super) fn validate_tick_by_tick_request(client: &Client, _contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<(), Error> {
    client.check_server_version(server_versions::TICK_BY_TICK, "It does not support tick-by-tick requests.")?;

    if number_of_ticks != 0 || ignore_size {
        client.check_server_version(
            server_versions::TICK_BY_TICK_IGNORE_SIZE,
            "It does not support ignore_size and number_of_ticks parameters in tick-by-tick requests.",
        )?;
    }

    Ok(())
}

// Requests tick by tick Last ticks.
pub(crate) fn tick_by_tick_last<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<'a, Trade>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let request = encoders::encode_tick_by_tick(server_version, request_id, contract, "Last", number_of_ticks, ignore_size)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests tick by tick BidAsk ticks.
pub(crate) fn tick_by_tick_bid_ask<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<'a, BidAsk>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let request = encoders::encode_tick_by_tick(server_version, request_id, contract, "BidAsk", number_of_ticks, ignore_size)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests tick by tick MidPoint ticks.
pub(crate) fn tick_by_tick_midpoint<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<'a, MidPoint>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let request_id = client.next_request_id();

    let request = encoders::encode_tick_by_tick(server_version, request_id, contract, "MidPoint", number_of_ticks, ignore_size)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

pub(crate) fn market_depth<'a>(
    client: &'a Client,
    contract: &Contract,
    number_of_rows: i32,
    is_smart_depth: bool,
) -> Result<Subscription<'a, MarketDepths>, Error> {
    if is_smart_depth {
        client.check_server_version(server_versions::SMART_DEPTH, "It does not support SMART depth request.")?;
    }
    if !contract.primary_exchange.is_empty() {
        client.check_server_version(
            server_versions::MKT_DEPTH_PRIM_EXCHANGE,
            "It does not support primary_exchange parameter in request_market_depth",
        )?;
    }

    let request_id = client.next_request_id();
    let request = encoders::encode_request_market_depth(client.server_version, request_id, contract, number_of_rows, is_smart_depth)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(
        client,
        subscription,
        ResponseContext {
            is_smart_depth,
            ..Default::default()
        },
    ))
}

// Requests venues for which market data is returned to market_depth (those with market makers)
pub fn market_depth_exchanges(client: &Client) -> Result<Vec<DepthMarketDataDescription>, Error> {
    client.check_server_version(
        server_versions::REQ_MKT_DEPTH_EXCHANGES,
        "It does not support market depth exchanges requests.",
    )?;

    loop {
        let request = encoders::encode_request_market_depth_exchanges()?;
        let subscription = client.send_shared_request(OutgoingMessages::RequestMktDepthExchanges, request)?;
        let response = subscription.next();

        match response {
            Some(Ok(mut message)) => return decoders::decode_market_depth_exchanges(client.server_version(), &mut message),
            Some(Err(Error::ConnectionReset)) => {
                debug!("connection reset. retrying market_depth_exchanges");
                continue;
            }
            Some(Err(e)) => return Err(e),
            None => return Ok(Vec::new()),
        }
    }
}

// Requests real time market data.
pub fn market_data<'a>(
    client: &'a Client,
    contract: &Contract,
    generic_ticks: &[&str],
    snapshot: bool,
    regulatory_snapshot: bool,
) -> Result<Subscription<'a, TickTypes>, Error> {
    let request_id = client.next_request_id();
    let request = encoders::encode_request_market_data(
        client.server_version(),
        request_id,
        contract,
        generic_ticks,
        snapshot,
        regulatory_snapshot,
    )?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}