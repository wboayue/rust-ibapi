use log::debug;

use crate::client::ClientRequestBuilders;
use crate::contracts::{Contract, TagValue};
use crate::messages::{IncomingMessages, Notice, OutgoingMessages, ResponseMessage};
use crate::protocol::{check_version, Features};
use crate::subscriptions::{AsyncDataStream, Subscription};
use crate::{Client, Error};

use super::common::{decoders, encoders};
use super::{Bar, BarSize, BidAsk, DepthMarketDataDescription, MarketDepths, MidPoint, TickTypes, Trade, WhatToShow};

// === AsyncDataStream implementations ===

impl AsyncDataStream<BidAsk> for BidAsk {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickByTick];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickByTick => decoders::decode_bid_ask_tick(message),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl AsyncDataStream<MidPoint> for MidPoint {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickByTick];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickByTick => decoders::decode_mid_point_tick(message),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl AsyncDataStream<Bar> for Bar {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::RealTimeBars];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_realtime_bar(message)
    }
}

impl AsyncDataStream<Trade> for Trade {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickByTick];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickByTick => decoders::decode_trade_tick(message),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl AsyncDataStream<MarketDepths> for MarketDepths {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] =
        &[IncomingMessages::MarketDepth, IncomingMessages::MarketDepthL2, IncomingMessages::Error];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        use crate::messages;
        match message.message_type() {
            IncomingMessages::MarketDepth => Ok(MarketDepths::MarketDepth(decoders::decode_market_depth(message)?)),
            IncomingMessages::MarketDepthL2 => Ok(MarketDepths::MarketDepthL2(decoders::decode_market_depth_l2(
                client.server_version(),
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
}

impl AsyncDataStream<TickTypes> for TickTypes {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
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
            IncomingMessages::TickPrice => Ok(decoders::decode_tick_price(client.server_version(), message)?),
            IncomingMessages::TickSize => Ok(TickTypes::Size(decoders::decode_tick_size(message)?)),
            IncomingMessages::TickString => Ok(TickTypes::String(decoders::decode_tick_string(message)?)),
            IncomingMessages::TickEFP => Ok(TickTypes::EFP(decoders::decode_tick_efp(message)?)),
            IncomingMessages::TickGeneric => Ok(TickTypes::Generic(decoders::decode_tick_generic(message)?)),
            IncomingMessages::TickOptionComputation => Ok(TickTypes::OptionComputation(decoders::decode_tick_option_computation(
                client.server_version(),
                message,
            )?)),
            IncomingMessages::TickReqParams => Ok(TickTypes::RequestParameters(decoders::decode_tick_request_parameters(message)?)),
            IncomingMessages::TickSnapshotEnd => Ok(TickTypes::SnapshotEnd),
            IncomingMessages::Error => Ok(TickTypes::Notice(Notice::from(message))),
            _ => Err(Error::NotImplemented),
        }
    }
}

// === Public API Functions ===

/// Requests realtime bars.
pub async fn realtime_bars(
    client: &Client,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_rth: bool,
    options: Vec<TagValue>,
) -> Result<Subscription<Bar>, Error> {
    let builder = client.request();
    let request = encoders::encode_request_realtime_bars(
        client.server_version(),
        builder.request_id(),
        contract,
        bar_size,
        what_to_show,
        use_rth,
        options,
    )?;

    builder.send::<Bar>(request).await
}

/// Requests tick by tick AllLast ticks.
pub async fn tick_by_tick_all_last(
    client: &Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<Trade>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let builder = client.request();

    let request = encoders::encode_tick_by_tick(server_version, builder.request_id(), contract, "AllLast", number_of_ticks, ignore_size)?;

    builder.send::<Trade>(request).await
}

/// Validates that server supports the given request.
pub(super) fn validate_tick_by_tick_request(client: &Client, _contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<(), Error> {
    check_version(client.server_version(), Features::TICK_BY_TICK)?;

    if number_of_ticks != 0 || ignore_size {
        check_version(client.server_version(), Features::TICK_BY_TICK_IGNORE_SIZE)?;
    }

    Ok(())
}

/// Requests tick by tick Last ticks.
pub async fn tick_by_tick_last(client: &Client, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<Subscription<Trade>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let builder = client.request();

    let request = encoders::encode_tick_by_tick(server_version, builder.request_id(), contract, "Last", number_of_ticks, ignore_size)?;

    builder.send::<Trade>(request).await
}

/// Requests tick by tick BidAsk ticks.
pub async fn tick_by_tick_bid_ask(
    client: &Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<BidAsk>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let builder = client.request();

    let request = encoders::encode_tick_by_tick(server_version, builder.request_id(), contract, "BidAsk", number_of_ticks, ignore_size)?;

    builder.send::<BidAsk>(request).await
}

/// Requests tick by tick MidPoint ticks.
pub async fn tick_by_tick_midpoint(
    client: &Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<MidPoint>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let builder = client.request();

    let request = encoders::encode_tick_by_tick(server_version, builder.request_id(), contract, "MidPoint", number_of_ticks, ignore_size)?;

    builder.send::<MidPoint>(request).await
}

/// Requests market depth data.
pub async fn market_depth(
    client: &Client,
    contract: &Contract,
    number_of_rows: i32,
    is_smart_depth: bool,
) -> Result<Subscription<MarketDepths>, Error> {
    if is_smart_depth {
        check_version(client.server_version(), Features::SMART_DEPTH)?;
    }
    if !contract.primary_exchange.is_empty() {
        check_version(client.server_version(), Features::MKT_DEPTH_PRIM_EXCHANGE)?;
    }

    let builder = client.request();
    let request = encoders::encode_request_market_depth(client.server_version(), builder.request_id(), contract, number_of_rows, is_smart_depth)?;

    builder.send::<MarketDepths>(request).await
}

/// Requests venues for which market data is returned to market_depth (those with market makers)
pub async fn market_depth_exchanges(client: &Client) -> Result<Vec<DepthMarketDataDescription>, Error> {
    check_version(client.server_version(), Features::REQ_MKT_DEPTH_EXCHANGES)?;

    loop {
        let request = encoders::encode_request_market_depth_exchanges()?;
        let mut subscription = client
            .shared_request(OutgoingMessages::RequestMktDepthExchanges)
            .send_raw(request)
            .await?;
        let response = subscription.next().await;

        match response {
            Some(mut message) => return decoders::decode_market_depth_exchanges(client.server_version(), &mut message),
            None => {
                debug!("connection reset. retrying market_depth_exchanges");
                continue;
            }
        }
    }
}

/// Requests real time market data.
pub async fn market_data(
    client: &Client,
    contract: &Contract,
    generic_ticks: &[&str],
    snapshot: bool,
    regulatory_snapshot: bool,
) -> Result<Subscription<TickTypes>, Error> {
    let builder = client.request();
    let request = encoders::encode_request_market_data(
        client.server_version(),
        builder.request_id(),
        contract,
        generic_ticks,
        snapshot,
        regulatory_snapshot,
    )?;

    builder.send::<TickTypes>(request).await
}
