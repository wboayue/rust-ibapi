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
    let pages = TickPages::<TickLast>::new(Some(datetime!(2026-11-13 14:26:44 UTC)), datetime!(2026-11-13 14:29:59.4 UTC), None).unwrap();

    assert_eq!(pages.request_end(), datetime!(2026-11-13 14:30:00 UTC));
}

#[test]
fn next_request_ends_at_the_earliest_second() {
    let mut pages = TickPages::new(Some(datetime!(2026-11-13 14:26:44 UTC)), datetime!(2026-11-13 14:29:59 UTC), None).unwrap();

    let more = pages.push(vec![
        tick(datetime!(2026-11-13 14:28:35 UTC), 1.0),
        tick(datetime!(2026-11-13 14:28:35 UTC), 2.0),
        tick(datetime!(2026-11-13 14:29:59 UTC), 3.0),
    ]);

    assert_eq!(more, ControlFlow::Continue(()));
    assert_eq!(pages.request_end(), datetime!(2026-11-13 14:28:35 UTC));
}

#[test]
fn pages_join_oldest_first_and_stop_at_the_start_second() {
    let start = datetime!(2026-11-13 14:26:44.5 UTC);
    let end = datetime!(2026-11-13 14:29:59 UTC);
    let mut pages = TickPages::new(Some(start), end, None).unwrap();

    let first = pages.push(vec![
        tick(datetime!(2026-11-13 14:28:35 UTC), 3.0),
        tick(datetime!(2026-11-13 14:29:59 UTC), 4.0),
    ]);
    let second = pages.push(vec![
        tick(datetime!(2026-11-13 14:26:44 UTC), 1.0),
        tick(datetime!(2026-11-13 14:28:34 UTC), 2.0),
    ]);

    assert_eq!(first, ControlFlow::Continue(()));
    assert_eq!(
        second,
        ControlFlow::Break(PagingStop::ReachedStart),
        "the page reaches back to the start second"
    );
    let result = pages.finish(PagingStop::ReachedStart);
    // 14:26:44 is before the fractional start, so it is dropped
    assert_eq!(prices(&result.ticks), vec![2.0, 3.0, 4.0]);
    assert_eq!(result.from, Some(start));
    assert_eq!(result.stop, PagingStop::ReachedStart);
}

#[test]
fn empty_page_stops() {
    let mut pages = TickPages::<TickLast>::new(Some(datetime!(2026-11-13 14:00:00 UTC)), datetime!(2026-11-13 15:00:00 UTC), None).unwrap();

    let more = pages.push(Vec::new());

    assert_eq!(more, ControlFlow::Break(PagingStop::EmptyPage));
    assert_eq!(
        pages.finish(PagingStop::EmptyPage),
        TicksBetween {
            ticks: Vec::new(),
            from: None,
            stop: PagingStop::EmptyPage,
        }
    );
}

#[test]
fn ticks_at_or_after_the_request_end_are_dropped() {
    let mut pages = TickPages::new(Some(datetime!(2026-11-13 14:00:00 UTC)), datetime!(2026-11-13 15:00:00 UTC), None).unwrap();
    assert_eq!(pages.push(vec![tick(datetime!(2026-11-13 14:30:00 UTC), 2.0)]), ControlFlow::Continue(()));

    // A page repeating the previous one adds nothing and ends paging
    let more = pages.push(vec![tick(datetime!(2026-11-13 14:30:00 UTC), 2.0)]);

    assert_eq!(more, ControlFlow::Break(PagingStop::EmptyPage));
    let result = pages.finish(PagingStop::EmptyPage);
    assert_eq!(prices(&result.ticks), vec![2.0]);
    assert_eq!(result.from, Some(datetime!(2026-11-13 14:30:00 UTC)));
}

#[test]
fn ticks_after_the_end_are_dropped() {
    let mut pages = TickPages::new(Some(datetime!(2026-11-13 14:00:00 UTC)), datetime!(2026-11-13 14:30:00.5 UTC), None).unwrap();

    let more = pages.push(vec![
        tick(datetime!(2026-11-13 13:59:59 UTC), 1.0),
        tick(datetime!(2026-11-13 14:30:00 UTC), 2.0),
        tick(datetime!(2026-11-13 14:30:00.7 UTC), 3.0),
    ]);

    assert_eq!(more, ControlFlow::Break(PagingStop::ReachedStart));
    assert_eq!(prices(&pages.into_ticks()), vec![2.0]);
}

#[test]
fn end_not_after_start_is_rejected() {
    let time = datetime!(2026-11-13 14:00:00 UTC);

    let result = TickPages::<TickLast>::new(Some(time), time, None);

    let Err(error) = result else { panic!("end == start was accepted") };
    assert!(matches!(error.error, Error::InvalidArgument(_)), "got {:?}", error.error);
    assert!(error.ticks.is_empty());
    assert_eq!(error.from, None);
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
    let mut pages = TickPages::new(Some(datetime!(2026-11-23 09:16:00 UTC)), datetime!(2026-11-23 09:17:01 UTC), None).unwrap();

    // TWS sends one tick of the earliest second of a bid/ask page
    let more = pages.push(vec![
        quote(datetime!(2026-11-23 09:16:59 UTC), 1.0),
        quote(datetime!(2026-11-23 09:17:00 UTC), 2.0),
        quote(datetime!(2026-11-23 09:17:01 UTC), 3.0),
    ]);

    assert_eq!(more, ControlFlow::Continue(()));
    assert_eq!(pages.request_end(), datetime!(2026-11-23 09:17:00 UTC));

    let more = pages.push(vec![
        quote(datetime!(2026-11-23 09:15:59 UTC), 4.0),
        quote(datetime!(2026-11-23 09:16:59 UTC), 5.0),
        quote(datetime!(2026-11-23 09:16:59 UTC), 6.0),
    ]);

    assert_eq!(
        more,
        ControlFlow::Break(PagingStop::ReachedStart),
        "the page's complete seconds start at 09:16:00, the start"
    );
    // 09:16:59 comes from the second page only; the first page's single tick of it is dropped
    assert_eq!(bids(&pages.into_ticks()), vec![5.0, 6.0, 2.0, 3.0]);
}

#[test]
fn quote_page_holding_only_a_partial_second_keeps_it() {
    let mut pages = TickPages::new(Some(datetime!(2026-11-23 09:00:00 UTC)), datetime!(2026-11-23 09:17:00 UTC), None).unwrap();

    let more = pages.push(vec![quote(datetime!(2026-11-23 09:17:00 UTC), 1.0)]);

    assert_eq!(more, ControlFlow::Continue(()));
    assert_eq!(pages.request_end(), datetime!(2026-11-23 09:17:00 UTC));
    assert_eq!(bids(&pages.into_ticks()), vec![1.0]);
}

#[test]
fn failure_keeps_the_complete_seconds_fetched() {
    let mut pages = TickPages::new(Some(datetime!(2026-11-23 09:00:00 UTC)), datetime!(2026-11-23 09:17:01 UTC), None).unwrap();
    let more = pages.push(vec![
        quote(datetime!(2026-11-23 09:16:59 UTC), 1.0),
        quote(datetime!(2026-11-23 09:17:00 UTC), 2.0),
        quote(datetime!(2026-11-23 09:17:01 UTC), 3.0),
    ]);
    assert_eq!(more, ControlFlow::Continue(()));

    let error = pages.fail(Error::ConnectionReset);

    assert!(matches!(error.error, Error::ConnectionReset), "got {:?}", error.error);
    // The partial earliest second, 09:16:59, is not kept
    assert_eq!(bids(&error.ticks), vec![2.0, 3.0]);
    assert_eq!(error.from, Some(datetime!(2026-11-23 09:17:00 UTC)));
}

#[test]
fn failure_before_any_page_keeps_nothing() {
    let pages = TickPages::<TickLast>::new(Some(datetime!(2026-11-23 09:00:00 UTC)), datetime!(2026-11-23 09:17:01 UTC), None).unwrap();

    let error = pages.fail(Error::ConnectionReset);

    assert!(matches!(error.error, Error::ConnectionReset), "got {:?}", error.error);
    assert!(error.ticks.is_empty());
    assert_eq!(error.from, None);
}

// Trades: two at 14:28:34, three at 14:28:35, one at 14:29:59
fn trade_page() -> Vec<TickLast> {
    vec![
        tick(datetime!(2026-11-13 14:28:34 UTC), 1.0),
        tick(datetime!(2026-11-13 14:28:34.5 UTC), 2.0),
        tick(datetime!(2026-11-13 14:28:35 UTC), 3.0),
        tick(datetime!(2026-11-13 14:28:35.2 UTC), 4.0),
        tick(datetime!(2026-11-13 14:28:35.7 UTC), 5.0),
        tick(datetime!(2026-11-13 14:29:59 UTC), 6.0),
    ]
}

#[test]
fn limit_keeps_the_latest_whole_seconds() {
    let mut pages = TickPages::new(None, datetime!(2026-11-13 14:29:59 UTC), Some(3)).unwrap();
    assert!(!pages.limit_reached());

    assert_eq!(pages.push(trade_page()), ControlFlow::Continue(()));
    assert!(pages.limit_reached());
    let result = pages.finish(PagingStop::LimitReached);

    // The third latest tick is in 14:28:35, kept whole: four ticks for a limit of three
    assert_eq!(prices(&result.ticks), vec![3.0, 4.0, 5.0, 6.0]);
    assert_eq!(result.from, Some(datetime!(2026-11-13 14:28:35 UTC)));
    assert_eq!(result.stop, PagingStop::LimitReached);
}

#[test]
fn limit_smaller_than_a_second_keeps_that_second() {
    let mut pages = TickPages::new(None, datetime!(2026-11-13 14:28:35.9 UTC), Some(1)).unwrap();

    let _ = pages.push(trade_page());
    let result = pages.finish(PagingStop::LimitReached);

    assert_eq!(prices(&result.ticks), vec![3.0, 4.0, 5.0]);
    assert_eq!(result.from, Some(datetime!(2026-11-13 14:28:35 UTC)));
}

#[test]
fn limit_counts_only_ticks_in_the_range() {
    // 14:29:59 is after the end, so the page holds five ticks in the range, not six
    let end = datetime!(2026-11-13 14:29:00 UTC);
    let mut six = TickPages::new(None, end, Some(6)).unwrap();
    let mut five = TickPages::new(None, end, Some(5)).unwrap();

    let _ = six.push(trade_page());
    let _ = five.push(trade_page());

    assert!(!six.limit_reached());
    assert!(five.limit_reached());
}

#[test]
fn limit_of_zero_is_reached_before_any_request() {
    let pages = TickPages::<TickLast>::new(None, datetime!(2026-11-13 14:29:59 UTC), Some(0)).unwrap();

    assert!(pages.limit_reached());
    assert_eq!(
        pages.finish(PagingStop::LimitReached),
        TicksBetween {
            ticks: Vec::new(),
            from: None,
            stop: PagingStop::LimitReached,
        }
    );
}

#[test]
fn limit_dropping_ticks_after_the_start_reports_the_limit() {
    let mut pages = TickPages::new(Some(datetime!(2026-11-13 14:28:34 UTC)), datetime!(2026-11-13 14:29:59 UTC), Some(3)).unwrap();

    let more = pages.push(trade_page());
    let result = pages.finish(PagingStop::ReachedStart);

    assert_eq!(more, ControlFlow::Break(PagingStop::ReachedStart));
    assert_eq!(prices(&result.ticks), vec![3.0, 4.0, 5.0, 6.0]);
    assert_eq!(result.from, Some(datetime!(2026-11-13 14:28:35 UTC)));
    assert_eq!(result.stop, PagingStop::LimitReached);
}

#[test]
fn limit_dropping_nothing_keeps_the_stop() {
    let start = datetime!(2026-11-13 14:28:34 UTC);
    let mut pages = TickPages::new(Some(start), datetime!(2026-11-13 14:29:59 UTC), Some(6)).unwrap();

    let more = pages.push(trade_page());
    let result = pages.finish(PagingStop::ReachedStart);

    assert_eq!(more, ControlFlow::Break(PagingStop::ReachedStart));
    assert_eq!(prices(&result.ticks), vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(result.from, Some(start));
    assert_eq!(result.stop, PagingStop::ReachedStart);
}

#[test]
fn without_a_start_paging_goes_on_until_an_empty_page() {
    let mut pages = TickPages::new(None, datetime!(2026-11-13 14:29:59 UTC), Some(100)).unwrap();

    let first = pages.push(trade_page());
    let second = pages.push(Vec::new());
    let result = pages.finish(PagingStop::EmptyPage);

    assert_eq!(first, ControlFlow::Continue(()));
    assert_eq!(second, ControlFlow::Break(PagingStop::EmptyPage));
    assert_eq!(prices(&result.ticks), vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(result.from, Some(datetime!(2026-11-13 14:28:34 UTC)));
    assert_eq!(result.stop, PagingStop::EmptyPage);
}
