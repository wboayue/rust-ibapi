use super::*;
use time::macros::{date, datetime};

fn bar(at: OffsetDateTime, close: f64) -> Bar {
    Bar {
        date: BarTimestamp::DateTime(at),
        close,
        ..Bar::default()
    }
}

fn daily(on: time::Date, close: f64) -> Bar {
    Bar {
        date: BarTimestamp::Date(on),
        close,
        ..Bar::default()
    }
}

fn update(bar: Bar) -> HistoricalDataFrame {
    HistoricalDataFrame::Update(bar)
}

#[test]
fn batch_completes_all_but_the_last_bar() {
    let mut completion = BarCompletion::new(BarSize::Min);
    let bars = vec![
        bar(datetime!(2026-11-24 9:14 UTC), 1.0),
        bar(datetime!(2026-11-24 9:15 UTC), 2.0),
        bar(datetime!(2026-11-24 9:16 UTC), 3.0),
    ];

    let items = completion.apply_at(HistoricalDataFrame::Bars(bars), datetime!(2026-11-24 9:16:34 UTC));

    assert_eq!(
        items,
        vec![
            StreamedBar::Completed(bar(datetime!(2026-11-24 9:14 UTC), 1.0)),
            StreamedBar::Completed(bar(datetime!(2026-11-24 9:15 UTC), 2.0)),
            StreamedBar::Forming(bar(datetime!(2026-11-24 9:16 UTC), 3.0)),
        ]
    );
}

// Live: the batch's last bar was the forming 9:16 bar, and the first updates revised it.
#[test]
fn update_revising_the_last_batch_bar_keeps_it_forming() {
    let mut completion = BarCompletion::new(BarSize::Min);
    completion.apply_at(
        HistoricalDataFrame::Bars(vec![bar(datetime!(2026-11-24 9:16 UTC), 3.0)]),
        datetime!(2026-11-24 9:16:34 UTC),
    );

    let items = completion.apply_at(update(bar(datetime!(2026-11-24 9:16 UTC), 3.5)), datetime!(2026-11-24 9:16:35 UTC));

    assert_eq!(items, vec![StreamedBar::Forming(bar(datetime!(2026-11-24 9:16 UTC), 3.5))]);
}

// The batch's last bar can already be complete; the first update, for the next bar, completes it.
#[test]
fn update_for_a_later_bar_completes_the_forming_one() {
    let mut completion = BarCompletion::new(BarSize::Sec30);
    completion.apply_at(
        HistoricalDataFrame::Bars(vec![bar(datetime!(2026-11-24 9:21:30 UTC), 1.0)]),
        datetime!(2026-11-24 9:22:06 UTC),
    );

    let items = completion.apply_at(update(bar(datetime!(2026-11-24 9:22 UTC), 2.0)), datetime!(2026-11-24 9:22:07 UTC));

    assert_eq!(
        items,
        vec![
            StreamedBar::Completed(bar(datetime!(2026-11-24 9:21:30 UTC), 1.0)),
            StreamedBar::Forming(bar(datetime!(2026-11-24 9:22 UTC), 2.0)),
        ]
    );
}

// Live: a 1-minute bar's twelfth update arrives 0.3 s after the bar ends.
#[test]
fn update_received_after_the_bar_end_completes_it() {
    let mut completion = BarCompletion::new(BarSize::Min);
    completion.apply_at(update(bar(datetime!(2026-11-24 9:17 UTC), 1.0)), datetime!(2026-11-24 9:17:55.3 UTC));

    let last = completion.apply_at(update(bar(datetime!(2026-11-24 9:17 UTC), 2.0)), datetime!(2026-11-24 9:18:00.3 UTC));
    let next = completion.apply_at(update(bar(datetime!(2026-11-24 9:18 UTC), 3.0)), datetime!(2026-11-24 9:18:05.3 UTC));

    assert_eq!(last, vec![StreamedBar::Completed(bar(datetime!(2026-11-24 9:17 UTC), 2.0))]);
    assert_eq!(next, vec![StreamedBar::Forming(bar(datetime!(2026-11-24 9:18 UTC), 3.0))]);
}

// Live: 5-second bars get one update each, after the bar ends.
#[test]
fn five_second_bars_complete_on_their_only_update() {
    let mut completion = BarCompletion::new(BarSize::Sec5);

    let first = completion.apply_at(update(bar(datetime!(2026-11-24 9:16:35 UTC), 1.0)), datetime!(2026-11-24 9:16:40.9 UTC));
    let second = completion.apply_at(update(bar(datetime!(2026-11-24 9:16:40 UTC), 2.0)), datetime!(2026-11-24 9:16:45.3 UTC));

    assert_eq!(first, vec![StreamedBar::Completed(bar(datetime!(2026-11-24 9:16:35 UTC), 1.0))]);
    assert_eq!(second, vec![StreamedBar::Completed(bar(datetime!(2026-11-24 9:16:40 UTC), 2.0))]);
}

#[test]
fn update_for_a_completed_bar_is_dropped() {
    let mut completion = BarCompletion::new(BarSize::Min);
    completion.apply_at(update(bar(datetime!(2026-11-24 9:17 UTC), 1.0)), datetime!(2026-11-24 9:18:00.3 UTC));

    let late = completion.apply_at(update(bar(datetime!(2026-11-24 9:17 UTC), 2.0)), datetime!(2026-11-24 9:18:01 UTC));

    assert_eq!(late, vec![]);
}

#[test]
fn resubscription_replay_completes_the_discarded_forming_bar_once() {
    let mut completion = BarCompletion::new(BarSize::Min);
    completion.apply_at(update(bar(datetime!(2026-11-24 9:16 UTC), 1.0)), datetime!(2026-11-24 9:17:00.3 UTC));
    completion.apply_at(update(bar(datetime!(2026-11-24 9:17 UTC), 2.0)), datetime!(2026-11-24 9:17:05 UTC));
    let replay = vec![
        bar(datetime!(2026-11-24 9:16 UTC), 1.0),
        bar(datetime!(2026-11-24 9:17 UTC), 2.5),
        bar(datetime!(2026-11-24 9:18 UTC), 3.0),
    ];

    let items = completion.apply_at(HistoricalDataFrame::Bars(replay), datetime!(2026-11-24 9:18:10 UTC));

    assert_eq!(
        items,
        vec![
            StreamedBar::Completed(bar(datetime!(2026-11-24 9:17 UTC), 2.5)),
            StreamedBar::Forming(bar(datetime!(2026-11-24 9:18 UTC), 3.0)),
        ]
    );
}

#[test]
fn daily_bars_complete_only_on_the_next_bar() {
    let mut completion = BarCompletion::new(BarSize::Day);
    let first = completion.apply_at(update(daily(date!(2026 - 11 - 23), 1.0)), datetime!(2026-11-25 0:00 UTC));
    let second = completion.apply_at(update(daily(date!(2026 - 11 - 24), 2.0)), datetime!(2026-11-25 0:00 UTC));

    assert_eq!(first, vec![StreamedBar::Forming(daily(date!(2026 - 11 - 23), 1.0))]);
    assert_eq!(
        second,
        vec![
            StreamedBar::Completed(daily(date!(2026 - 11 - 23), 1.0)),
            StreamedBar::Forming(daily(date!(2026 - 11 - 24), 2.0)),
        ]
    );
}

#[test]
fn end_and_empty_batch_return_nothing() {
    let mut completion = BarCompletion::new(BarSize::Min);
    let now = datetime!(2026-11-24 9:00 UTC);

    let batch = completion.apply_at(HistoricalDataFrame::Bars(vec![]), now);
    let end = completion.apply_at(HistoricalDataFrame::End { start: now, end: now }, now);

    assert_eq!(batch, vec![]);
    assert_eq!(end, vec![]);
}

#[test]
fn span_of_each_bar_size() {
    let cases = [
        (BarSize::Sec, Some(1)),
        (BarSize::Sec5, Some(5)),
        (BarSize::Sec10, Some(10)),
        (BarSize::Sec15, Some(15)),
        (BarSize::Sec30, Some(30)),
        (BarSize::Min, Some(60)),
        (BarSize::Min2, Some(120)),
        (BarSize::Min3, Some(180)),
        (BarSize::Min4, Some(240)),
        (BarSize::Min5, Some(300)),
        (BarSize::Min10, Some(600)),
        (BarSize::Min15, Some(900)),
        (BarSize::Min20, Some(1200)),
        (BarSize::Min30, Some(1800)),
        (BarSize::Hour, Some(3600)),
        (BarSize::Hour2, Some(7200)),
        (BarSize::Hour3, Some(10800)),
        (BarSize::Hour4, Some(14400)),
        (BarSize::Hour8, Some(28800)),
        (BarSize::Day, None),
        (BarSize::Week, None),
        (BarSize::Month, None),
    ];

    for (bar_size, seconds) in cases {
        assert_eq!(span(bar_size), seconds.map(time::Duration::seconds), "{bar_size}");
    }
}

fn forming_batch_bar() -> BarCompletion {
    let mut completion = BarCompletion::new(BarSize::Min);
    completion.apply_at(
        HistoricalDataFrame::Bars(vec![bar(datetime!(2026-11-24 9:15 UTC), 2.0), bar(datetime!(2026-11-24 9:16 UTC), 3.0)]),
        datetime!(2026-11-24 9:16:34 UTC),
    );
    completion
}

// Live: without real-time permission TWS sent the batch and then no update, leaving the batch's
// last bar forming.
#[test]
fn expire_completes_the_forming_bar_once_past_its_end_plus_grace() {
    let mut completion = forming_batch_bar();
    let grace = time::Duration::seconds(10);

    let end = completion.forming_end();
    let early = completion.expire_at(grace, datetime!(2026-11-24 9:17:09.999 UTC));
    let due = completion.expire_at(grace, datetime!(2026-11-24 9:17:10 UTC));
    let again = completion.expire_at(grace, datetime!(2026-11-24 9:18:00 UTC));

    assert_eq!(end, Some(datetime!(2026-11-24 9:17 UTC)));
    assert_eq!(early, None);
    assert_eq!(due, Some(bar(datetime!(2026-11-24 9:16 UTC), 3.0)));
    assert_eq!(again, None, "a completed bar is not completed twice");
    assert_eq!(completion.forming_end(), None);
}

#[test]
fn update_for_an_expired_bar_is_dropped() {
    let mut completion = forming_batch_bar();
    completion.expire_at(time::Duration::seconds(10), datetime!(2026-11-24 9:17:10 UTC));

    let late = completion.apply_at(update(bar(datetime!(2026-11-24 9:16 UTC), 3.5)), datetime!(2026-11-24 9:17:11 UTC));
    let next = completion.apply_at(update(bar(datetime!(2026-11-24 9:17 UTC), 4.0)), datetime!(2026-11-24 9:17:15 UTC));

    assert_eq!(late, Vec::new());
    assert_eq!(next, vec![StreamedBar::Forming(bar(datetime!(2026-11-24 9:17 UTC), 4.0))]);
}

#[test]
fn expire_leaves_a_daily_bar_forming() {
    let mut completion = BarCompletion::new(BarSize::Day);
    completion.apply_at(
        HistoricalDataFrame::Bars(vec![daily(date!(2026 - 11 - 24), 3.0)]),
        datetime!(2026-11-24 9:16:34 UTC),
    );

    let expired = completion.expire_at(time::Duration::seconds(10), datetime!(2026-12-24 0:00 UTC));

    assert_eq!(expired, None);
    assert_eq!(completion.forming_end(), None);
}
