use super::*;
use crate::market_data::realtime::{TickGeneric, TickPrice, TickPriceSize, TickRequestParameters, TickSize, TickString};

fn price(tick_type: TickType, price: f64) -> TickTypes {
    TickTypes::Price(TickPrice {
        tick_type,
        price,
        ..Default::default()
    })
}

fn size(tick_type: TickType, size: f64) -> TickTypes {
    TickTypes::Size(TickSize { tick_type, size })
}

fn greeks(field: TickType, delta: f64) -> OptionComputation {
    OptionComputation {
        field,
        delta: Some(delta),
        ..Default::default()
    }
}

#[test]
fn price_ticks_fill_fields() {
    let quote = Quote::from_ticks(&[
        price(TickType::Bid, 1.0),
        price(TickType::Ask, 2.0),
        price(TickType::Last, 3.0),
        price(TickType::Open, 4.0),
        price(TickType::High, 5.0),
        price(TickType::Low, 6.0),
        price(TickType::Close, 7.0),
    ]);

    assert_eq!(quote.bid, Some(1.0));
    assert_eq!(quote.ask, Some(2.0));
    assert_eq!(quote.last, Some(3.0));
    assert_eq!(quote.open, Some(4.0));
    assert_eq!(quote.high, Some(5.0));
    assert_eq!(quote.low, Some(6.0));
    assert_eq!(quote.close, Some(7.0));
}

#[test]
fn delayed_price_ticks_fill_realtime_fields() {
    let quote = Quote::from_ticks(&[
        price(TickType::DelayedBid, 1.0),
        price(TickType::DelayedAsk, 2.0),
        price(TickType::DelayedLast, 3.0),
        price(TickType::DelayedOpen, 4.0),
        price(TickType::DelayedHigh, 5.0),
        price(TickType::DelayedLow, 6.0),
        price(TickType::DelayedClose, 7.0),
    ]);

    assert_eq!(quote.bid, Some(1.0));
    assert_eq!(quote.ask, Some(2.0));
    assert_eq!(quote.last, Some(3.0));
    assert_eq!(quote.open, Some(4.0));
    assert_eq!(quote.high, Some(5.0));
    assert_eq!(quote.low, Some(6.0));
    assert_eq!(quote.close, Some(7.0));
}

#[test]
fn size_ticks_fill_fields() {
    let quote = Quote::from_ticks(&[
        size(TickType::BidSize, 10.0),
        size(TickType::AskSize, 20.0),
        size(TickType::LastSize, 30.0),
        size(TickType::Volume, 40.0),
    ]);

    assert_eq!(quote.bid_size, Some(10.0));
    assert_eq!(quote.ask_size, Some(20.0));
    assert_eq!(quote.last_size, Some(30.0));
    assert_eq!(quote.volume, Some(40.0));
}

#[test]
fn delayed_size_ticks_fill_realtime_fields() {
    let quote = Quote::from_ticks(&[
        size(TickType::DelayedBidSize, 10.0),
        size(TickType::DelayedAskSize, 20.0),
        size(TickType::DelayedLastSize, 30.0),
        size(TickType::DelayedVolume, 40.0),
    ]);

    assert_eq!(quote.bid_size, Some(10.0));
    assert_eq!(quote.ask_size, Some(20.0));
    assert_eq!(quote.last_size, Some(30.0));
    assert_eq!(quote.volume, Some(40.0));
}

#[test]
fn latest_tick_wins() {
    let quote = Quote::from_ticks(&[price(TickType::Bid, 1.0), price(TickType::DelayedBid, 1.5), price(TickType::Bid, 2.0)]);

    assert_eq!(quote.bid, Some(2.0));
}

#[test]
fn price_size_sets_price_and_size() {
    let quote = Quote::from_ticks(&[TickTypes::PriceSize(TickPriceSize {
        price_tick_type: TickType::Ask,
        price: 101.5,
        size_tick_type: TickType::AskSize,
        size: 300.0,
        ..Default::default()
    })]);

    assert_eq!(quote.ask, Some(101.5));
    assert_eq!(quote.ask_size, Some(300.0));
    assert_eq!(quote.bid, None);
    assert_eq!(quote.bid_size, None);
}

#[test]
fn option_computations_route_by_field() {
    let quote = Quote::from_ticks(&[
        TickTypes::OptionComputation(greeks(TickType::BidOption, 0.1)),
        TickTypes::OptionComputation(greeks(TickType::AskOption, 0.2)),
        TickTypes::OptionComputation(greeks(TickType::LastOption, 0.3)),
        TickTypes::OptionComputation(greeks(TickType::ModelOption, 0.4)),
    ]);

    assert_eq!(quote.bid_greeks, Some(greeks(TickType::BidOption, 0.1)));
    assert_eq!(quote.ask_greeks, Some(greeks(TickType::AskOption, 0.2)));
    assert_eq!(quote.last_greeks, Some(greeks(TickType::LastOption, 0.3)));
    assert_eq!(quote.model_greeks, Some(greeks(TickType::ModelOption, 0.4)));
}

#[test]
fn delayed_option_computations_route_by_field() {
    let quote = Quote::from_ticks(&[
        TickTypes::OptionComputation(greeks(TickType::DelayedBidOption, 0.1)),
        TickTypes::OptionComputation(greeks(TickType::DelayedAskOption, 0.2)),
        TickTypes::OptionComputation(greeks(TickType::DelayedLastOption, 0.3)),
        TickTypes::OptionComputation(greeks(TickType::DelayedModelOption, 0.4)),
    ]);

    assert_eq!(quote.bid_greeks, Some(greeks(TickType::DelayedBidOption, 0.1)));
    assert_eq!(quote.ask_greeks, Some(greeks(TickType::DelayedAskOption, 0.2)));
    assert_eq!(quote.last_greeks, Some(greeks(TickType::DelayedLastOption, 0.3)));
    assert_eq!(quote.model_greeks, Some(greeks(TickType::DelayedModelOption, 0.4)));
}

#[test]
fn market_data_type_is_recorded() {
    let quote = Quote::from_ticks(&[TickTypes::MarketDataType(MarketDataType::Delayed)]);

    assert_eq!(quote.market_data_type, Some(MarketDataType::Delayed));
}

#[test]
fn unmapped_ticks_are_ignored() {
    let quote = Quote::from_ticks(&[
        price(TickType::MarkPrice, 1.0),
        size(TickType::AvgVolume, 2.0),
        TickTypes::OptionComputation(greeks(TickType::CustOptionComputation, 0.5)),
        TickTypes::String(TickString {
            tick_type: TickType::LastTimestamp,
            value: "1700000000".to_string(),
        }),
        TickTypes::Generic(TickGeneric {
            tick_type: TickType::Shortable,
            value: 3.0,
        }),
        TickTypes::RequestParameters(TickRequestParameters::default()),
        TickTypes::SnapshotEnd,
    ]);

    assert_eq!(quote, Quote::default());
}

#[test]
fn from_ticks_matches_sequential_apply() {
    let ticks = vec![
        TickTypes::MarketDataType(MarketDataType::Realtime),
        price(TickType::Bid, 1.0),
        size(TickType::BidSize, 10.0),
        TickTypes::OptionComputation(greeks(TickType::ModelOption, 0.4)),
    ];

    let mut applied = Quote::default();
    for tick in &ticks {
        applied.apply(tick);
    }

    assert_eq!(Quote::from_ticks(&ticks), applied);
    assert_eq!(Quote::from_ticks(ticks.iter()), applied);
}
