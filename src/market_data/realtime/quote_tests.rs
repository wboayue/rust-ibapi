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

type Field<T> = fn(&Quote) -> Option<T>;

/// (real-time tick, delayed tick, field it fills)
const PRICE_FIELDS: &[(TickType, TickType, Field<f64>)] = &[
    (TickType::Bid, TickType::DelayedBid, |q| q.bid),
    (TickType::Ask, TickType::DelayedAsk, |q| q.ask),
    (TickType::Last, TickType::DelayedLast, |q| q.last),
    (TickType::Open, TickType::DelayedOpen, |q| q.open),
    (TickType::High, TickType::DelayedHigh, |q| q.high),
    (TickType::Low, TickType::DelayedLow, |q| q.low),
    (TickType::Close, TickType::DelayedClose, |q| q.close),
];

const SIZE_FIELDS: &[(TickType, TickType, Field<f64>)] = &[
    (TickType::BidSize, TickType::DelayedBidSize, |q| q.bid_size),
    (TickType::AskSize, TickType::DelayedAskSize, |q| q.ask_size),
    (TickType::LastSize, TickType::DelayedLastSize, |q| q.last_size),
    (TickType::Volume, TickType::DelayedVolume, |q| q.volume),
];

const GREEKS_FIELDS: &[(TickType, TickType, Field<OptionComputation>)] = &[
    (TickType::BidOption, TickType::DelayedBidOption, |q| q.bid_greeks.clone()),
    (TickType::AskOption, TickType::DelayedAskOption, |q| q.ask_greeks.clone()),
    (TickType::LastOption, TickType::DelayedLastOption, |q| q.last_greeks.clone()),
    (TickType::ModelOption, TickType::DelayedModelOption, |q| q.model_greeks.clone()),
];

#[test]
fn price_ticks_fill_their_field() {
    for &(realtime, delayed, field) in PRICE_FIELDS {
        for tick_type in [realtime, delayed] {
            let quote = Quote::from_ticks(&[price(tick_type, 1.5)]);
            assert_eq!(field(&quote), Some(1.5), "{tick_type:?}");
        }
    }
}

#[test]
fn size_ticks_fill_their_field() {
    for &(realtime, delayed, field) in SIZE_FIELDS {
        for tick_type in [realtime, delayed] {
            let quote = Quote::from_ticks(&[size(tick_type, 300.0)]);
            assert_eq!(field(&quote), Some(300.0), "{tick_type:?}");
        }
    }
}

#[test]
fn option_computations_route_by_field() {
    for &(realtime, delayed, field) in GREEKS_FIELDS {
        for tick_type in [realtime, delayed] {
            let quote = Quote::from_ticks(&[TickTypes::OptionComputation(greeks(tick_type, 0.4))]);
            assert_eq!(field(&quote), Some(greeks(tick_type, 0.4)), "{tick_type:?}");
        }
    }
}

/// Each tick fills only its own field.
#[test]
fn one_tick_fills_one_field() {
    let quote = Quote::from_ticks(&[price(TickType::Bid, 1.5)]);

    assert_eq!(
        quote,
        Quote {
            bid: Some(1.5),
            ..Quote::default()
        }
    );
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
