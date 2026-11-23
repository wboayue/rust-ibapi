use time::macros::datetime;

use super::*;

fn tick(time: OffsetDateTime, price: f64) -> TickLast {
    TickLast {
        time,
        price,
        ..TickLast::default()
    }
}

fn prices(ticks: &[TickLast]) -> Vec<f64> {
    ticks.iter().map(|tick| tick.price).collect()
}

#[test]
fn first_request_ends_after_the_end_second() {
    let pages = TickPages::<TickLast>::new(datetime!(2026-11-13 14:26:44 UTC), datetime!(2026-11-13 14:29:59.4 UTC)).unwrap();

    assert_eq!(pages.request_end(), datetime!(2026-11-13 14:30:00 UTC));
}

#[test]
fn next_request_ends_at_the_earliest_second() {
    let mut pages = TickPages::new(datetime!(2026-11-13 14:26:44 UTC), datetime!(2026-11-13 14:29:59 UTC)).unwrap();

    let more = pages.push(vec![
        tick(datetime!(2026-11-13 14:28:35 UTC), 1.0),
        tick(datetime!(2026-11-13 14:28:35 UTC), 2.0),
        tick(datetime!(2026-11-13 14:29:59 UTC), 3.0),
    ]);

    assert!(more);
    assert_eq!(pages.request_end(), datetime!(2026-11-13 14:28:35 UTC));
}

#[test]
fn pages_join_oldest_first_and_stop_at_the_start_second() {
    let start = datetime!(2026-11-13 14:26:44.5 UTC);
    let end = datetime!(2026-11-13 14:29:59 UTC);
    let mut pages = TickPages::new(start, end).unwrap();

    let first = pages.push(vec![
        tick(datetime!(2026-11-13 14:28:35 UTC), 3.0),
        tick(datetime!(2026-11-13 14:29:59 UTC), 4.0),
    ]);
    let second = pages.push(vec![
        tick(datetime!(2026-11-13 14:26:44 UTC), 1.0),
        tick(datetime!(2026-11-13 14:28:34 UTC), 2.0),
    ]);

    assert!(first);
    assert!(!second, "the page reaches back to the start second");
    // 14:26:44 is before the fractional start, so it is dropped
    assert_eq!(prices(&pages.into_ticks()), vec![2.0, 3.0, 4.0]);
}

#[test]
fn empty_page_stops() {
    let mut pages = TickPages::<TickLast>::new(datetime!(2026-11-13 14:00:00 UTC), datetime!(2026-11-13 15:00:00 UTC)).unwrap();

    let more = pages.push(Vec::new());

    assert!(!more);
    assert!(pages.into_ticks().is_empty());
}

#[test]
fn ticks_at_or_after_the_request_end_are_dropped() {
    let mut pages = TickPages::new(datetime!(2026-11-13 14:00:00 UTC), datetime!(2026-11-13 15:00:00 UTC)).unwrap();
    pages.push(vec![tick(datetime!(2026-11-13 14:30:00 UTC), 2.0)]);

    // A page repeating the previous one adds nothing and ends paging
    let more = pages.push(vec![tick(datetime!(2026-11-13 14:30:00 UTC), 2.0)]);

    assert!(!more);
    assert_eq!(prices(&pages.into_ticks()), vec![2.0]);
}

#[test]
fn ticks_after_the_end_are_dropped() {
    let mut pages = TickPages::new(datetime!(2026-11-13 14:00:00 UTC), datetime!(2026-11-13 14:30:00.5 UTC)).unwrap();

    pages.push(vec![
        tick(datetime!(2026-11-13 13:59:59 UTC), 1.0),
        tick(datetime!(2026-11-13 14:30:00 UTC), 2.0),
        tick(datetime!(2026-11-13 14:30:00.7 UTC), 3.0),
    ]);

    assert_eq!(prices(&pages.into_ticks()), vec![2.0]);
}

#[test]
fn end_not_after_start_is_rejected() {
    let time = datetime!(2026-11-13 14:00:00 UTC);

    let result = TickPages::<TickLast>::new(time, time);

    assert!(matches!(result, Err(Error::InvalidArgument(_))), "got {:?}", result.err());
}

fn quote(time: OffsetDateTime, price_bid: f64) -> TickBidAsk {
    TickBidAsk {
        time,
        price_bid,
        ..TickBidAsk::default()
    }
}

fn bids(ticks: &[TickBidAsk]) -> Vec<f64> {
    ticks.iter().map(|tick| tick.price_bid).collect()
}

#[test]
fn quote_page_refetches_its_partial_earliest_second() {
    let mut pages = TickPages::new(datetime!(2026-11-23 09:16:00 UTC), datetime!(2026-11-23 09:17:01 UTC)).unwrap();

    // TWS sends one tick of the earliest second of a bid/ask page
    let more = pages.push(vec![
        quote(datetime!(2026-11-23 09:16:59 UTC), 1.0),
        quote(datetime!(2026-11-23 09:17:00 UTC), 2.0),
        quote(datetime!(2026-11-23 09:17:01 UTC), 3.0),
    ]);

    assert!(more);
    assert_eq!(pages.request_end(), datetime!(2026-11-23 09:17:00 UTC));

    let more = pages.push(vec![
        quote(datetime!(2026-11-23 09:15:59 UTC), 4.0),
        quote(datetime!(2026-11-23 09:16:59 UTC), 5.0),
        quote(datetime!(2026-11-23 09:16:59 UTC), 6.0),
    ]);

    assert!(!more, "the page's complete seconds start at 09:16:00, the start");
    // 09:16:59 comes from the second page only; the first page's single tick of it is dropped
    assert_eq!(bids(&pages.into_ticks()), vec![5.0, 6.0, 2.0, 3.0]);
}

#[test]
fn quote_page_holding_only_a_partial_second_keeps_it() {
    let mut pages = TickPages::new(datetime!(2026-11-23 09:00:00 UTC), datetime!(2026-11-23 09:17:00 UTC)).unwrap();

    let more = pages.push(vec![quote(datetime!(2026-11-23 09:17:00 UTC), 1.0)]);

    assert!(more);
    assert_eq!(pages.request_end(), datetime!(2026-11-23 09:17:00 UTC));
    assert_eq!(bids(&pages.into_ticks()), vec![1.0]);
}
