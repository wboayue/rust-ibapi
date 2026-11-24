use super::*;
use crate::market_data::realtime::{TickGeneric, TickPrice, TickPriceSize, TickRequestParameters, TickSize, TickString};

fn price_size(price_tick_type: TickType, price: f64, size: f64) -> TickTypes {
    let size_tick_type = match price_tick_type {
        TickType::Bid => TickType::BidSize,
        TickType::Ask => TickType::AskSize,
        TickType::DelayedBid => TickType::DelayedBidSize,
        TickType::DelayedAsk => TickType::DelayedAskSize,
        other => panic!("no size tick for {other:?}"),
    };
    TickTypes::PriceSize(TickPriceSize {
        price_tick_type,
        price,
        size_tick_type,
        size,
        ..Default::default()
    })
}

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
            bid_attributes: Some(TickAttribute::default()),
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

/// Live-observed empty sides (`docs/wire-facts/market-data.md#no-quote-sides`): the raw fields keep
/// what TWS sent, the side accessors report no quote.
#[test]
fn empty_sides_have_no_side() {
    let cases = [
        ("option, delayed", TickType::DelayedBid, -1.0),
        ("magnifier 100", TickType::DelayedBid, -100.0),
        ("combo", TickType::Bid, 0.0),
    ];
    for (label, tick_type, placeholder) in cases {
        let quote = Quote::from_ticks(&[price_size(tick_type, placeholder, 0.0)]);
        assert_eq!((quote.bid, quote.bid_size), (Some(placeholder), Some(0.0)), "{label}");
        assert_eq!(quote.bid_side(), None, "{label}");
    }
}

#[test]
fn sides_need_price_and_positive_size() {
    let cases = [
        (
            "calendar spread",
            vec![price_size(TickType::DelayedBid, -71.0, 1.0)],
            Some(QuoteSide { price: -71.0, size: 1.0 }),
        ),
        ("price without size", vec![price(TickType::Bid, 5.0)], None),
        ("size without price", vec![size(TickType::BidSize, 5.0)], None),
        (
            "size drops to zero",
            vec![price_size(TickType::Bid, 5.0, 3.0), size(TickType::BidSize, 0.0)],
            None,
        ),
        (
            "side returns",
            vec![price_size(TickType::Bid, -1.0, 0.0), price_size(TickType::Bid, 5.0, 3.0)],
            Some(QuoteSide { price: 5.0, size: 3.0 }),
        ),
    ];
    for (label, ticks, expected) in cases {
        assert_eq!(Quote::from_ticks(&ticks).bid_side(), expected, "{label}");
    }
}

/// A far out-of-the-money option on delayed-frozen data: an offer and no bid.
#[test]
fn one_sided_quote() {
    let quote = Quote::from_ticks(&[
        price_size(TickType::DelayedBid, -1.0, 0.0),
        price_size(TickType::DelayedAsk, -1.0, 0.0),
        price_size(TickType::DelayedAsk, 0.01, 5.0),
    ]);

    assert_eq!((quote.bid_side(), quote.ask_side()), (None, Some(QuoteSide { price: 0.01, size: 5.0 })));
}

#[test]
fn price_ticks_keep_attributes_per_side() {
    let attributes = |pre_open| TickAttribute {
        can_auto_execute: true,
        past_limit: false,
        pre_open,
    };
    let tick = |tick_type, pre_open| {
        TickTypes::Price(TickPrice {
            tick_type,
            price: 1.0,
            attributes: attributes(pre_open),
        })
    };
    let ask_from_price_size = TickTypes::PriceSize(TickPriceSize {
        price_tick_type: TickType::DelayedAsk,
        price: 1.0,
        attributes: attributes(true),
        size_tick_type: TickType::DelayedAskSize,
        size: 1.0,
    });

    let quote = Quote::from_ticks(&[
        tick(TickType::Bid, true),
        ask_from_price_size,
        tick(TickType::Last, false),
        tick(TickType::High, false),
    ]);

    assert_eq!(
        (quote.bid_attributes, quote.ask_attributes),
        (Some(attributes(true)), Some(attributes(true)))
    );
    assert_eq!(
        Quote::from_ticks(&[tick(TickType::Bid, true), tick(TickType::Bid, false)]).bid_attributes,
        Some(attributes(false))
    );
}
