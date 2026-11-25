//! [`BarCompletion`]: completed bars from a keep-up-to-date historical data stream.

use time::OffsetDateTime;

use super::{Bar, BarSize, BarTimestamp, HistoricalDataFrame};

#[cfg(test)]
#[path = "completion_tests.rs"]
mod tests;

/// A bar read from a keep-up-to-date stream, as [`BarCompletion`] classifies it.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq)]
pub enum StreamedBar {
    /// The current value of a bar that is still forming. A later item revises it.
    Forming(Bar),
    /// A bar that has ended. No later item revises it.
    Completed(Bar),
}

/// Turns the items of a keep-up-to-date historical data stream into completed bars.
///
/// [`HistoricalDataBuilder::subscribe_with_updates`](super::HistoricalDataBuilder::subscribe_with_updates)
/// yields the forming bar again on every [`HistoricalDataFrame::Update`] and never marks one as
/// final. Feed each item to [`apply`](Self::apply): it returns each bar as
/// [`StreamedBar::Completed`] exactly once, oldest first, and every other value as
/// [`StreamedBar::Forming`].
///
/// TWS sends an update about every 5 seconds, just after each 5-second boundary of the wall
/// clock, carrying the bar that holds the 5 seconds just ended, changed or not. A bar completes
/// on whichever comes first:
///
/// - an update for it that arrives at or after its end (its start plus the bar size), read on
///   the local clock. This is TWS's last update for the bar, received under a second after the
///   bar ends (live: 2026-11-24). It needs the local clock within a few seconds of TWS's.
/// - an update for a later bar. Daily and longer bars, whose end depends on the trading
///   session, complete only this way.
///
/// The initial batch's bars are complete except the last, which may still be forming: it is
/// [`Forming`](StreamedBar::Forming) until the first update. A bar no update follows, such as
/// the last one before TWS stops sending, stays forming; [`expire`](BarCompletion::expire)
/// completes it once the local clock is past its end plus a grace the caller chooses.
///
/// At a session break TWS sends nothing until the next session's first bar and never sends bars
/// for the break: a EUR.USD stream opened during the FX daily break got a batch ending with the
/// 21:59 UTC bar, then nothing until the first update for 22:15 at 22:15:06 (live: 2026-11-24).
/// Without [`expire`](BarCompletion::expire), the last bar before a break completes only when the
/// next session starts.
///
/// Bars with volume 0 are TWS's own: they come in historical replies too, and some carry a new
/// price. They are classified like any other bar.
///
/// Updates for a bar already completed are dropped. Reuse one `BarCompletion` across a
/// resubscription: the new batch replays the bars already completed, and they are not
/// repeated.
///
/// # Examples
///
#[cfg_attr(
    feature = "sync",
    doc = r#"
```no_run
use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarCompletion, BarSize, StreamedBar, ToDuration, WhatToShow};

let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
let contract = Contract::forex("EUR", "USD").build();

let subscription = client
    .historical_data(&contract, BarSize::Min)
    .what_to_show(WhatToShow::MidPoint)
    .duration(1.days())
    .subscribe_with_updates()
    .expect("streaming request failed");

let mut completion = BarCompletion::new(BarSize::Min);
for update in subscription.iter_data() {
    let update = update.expect("stream failed");
    for bar in completion.apply(update) {
        match bar {
            StreamedBar::Completed(bar) => println!("completed {bar:?}"),
            StreamedBar::Forming(bar) => println!("forming {}", bar.close),
        }
    }
}
```
"#
)]
#[cfg_attr(
    feature = "async",
    doc = r#"
```no_run
use ibapi::prelude::*;
use ibapi::market_data::historical::{BarCompletion, StreamedBar};

#[tokio::main]
async fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    let contract = Contract::forex("EUR", "USD").build();

    let subscription = client
        .historical_data(&contract, HistoricalBarSize::Min)
        .what_to_show(HistoricalWhatToShow::MidPoint)
        .duration(1.days())
        .subscribe_with_updates()
        .await
        .expect("streaming request failed");

    let mut completion = BarCompletion::new(HistoricalBarSize::Min);
    let mut updates = subscription.filter_data();
    while let Some(update) = updates.next().await {
        let update = update.expect("stream failed");
        for bar in completion.apply(update) {
            match bar {
                StreamedBar::Completed(bar) => println!("completed {bar:?}"),
                StreamedBar::Forming(bar) => println!("forming {}", bar.close),
            }
        }
    }
}
```
"#
)]
#[derive(Clone, Debug)]
pub struct BarCompletion {
    span: Option<time::Duration>,
    forming: Option<Bar>,
    completed_through: Option<BarTimestamp>,
}

impl BarCompletion {
    /// A tracker for a stream of `bar_size` bars, the size the stream was requested with.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::market_data::historical::{BarCompletion, BarSize};
    ///
    /// let completion = BarCompletion::new(BarSize::Min5);
    /// # drop(completion);
    /// ```
    pub fn new(bar_size: BarSize) -> Self {
        Self {
            span: span(bar_size),
            forming: None,
            completed_through: None,
        }
    }

    /// Applies one stream item and returns the bars it completes, oldest first, then the bar it
    /// leaves forming, if any. [`End`](HistoricalDataFrame::End) returns nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::market_data::historical::{Bar, BarCompletion, BarSize, BarTimestamp, HistoricalDataFrame, StreamedBar};
    /// use time::macros::datetime;
    ///
    /// let bar = |at| Bar { date: BarTimestamp::DateTime(at), ..Bar::default() };
    /// let mut completion = BarCompletion::new(BarSize::Min);
    ///
    /// // The batch's last bar may still be forming.
    /// let batch = vec![bar(datetime!(2026-04-14 9:30 UTC)), bar(datetime!(2026-04-14 9:31 UTC))];
    /// let items = completion.apply(HistoricalDataFrame::Bars(batch));
    /// assert_eq!(items[0], StreamedBar::Completed(bar(datetime!(2026-04-14 9:30 UTC))));
    /// assert_eq!(items[1], StreamedBar::Forming(bar(datetime!(2026-04-14 9:31 UTC))));
    ///
    /// // An update received after the bar's end completes it.
    /// let items = completion.apply(HistoricalDataFrame::Update(bar(datetime!(2026-04-14 9:31 UTC))));
    /// assert_eq!(items, vec![StreamedBar::Completed(bar(datetime!(2026-04-14 9:31 UTC)))]);
    /// ```
    pub fn apply(&mut self, update: HistoricalDataFrame) -> Vec<StreamedBar> {
        self.apply_at(update, OffsetDateTime::now_utc())
    }

    fn apply_at(&mut self, update: HistoricalDataFrame, now: OffsetDateTime) -> Vec<StreamedBar> {
        let mut items = Vec::new();
        match update {
            HistoricalDataFrame::Bars(bars) => {
                self.forming = None;
                let mut bars = bars.into_iter();
                let last = bars.next_back();
                for bar in bars {
                    self.complete(bar, &mut items);
                }
                if let Some(bar) = last.filter(|bar| self.is_new(bar)) {
                    self.forming = Some(bar);
                    items.push(StreamedBar::Forming(bar));
                }
            }
            HistoricalDataFrame::Update(bar) => {
                if !self.is_new(&bar) {
                    return items;
                }
                if let Some(forming) = self.forming.take_if(|forming| forming.date < bar.date) {
                    self.complete(forming, &mut items);
                }
                if self.has_ended(&bar, now) {
                    self.forming = None;
                    self.complete(bar, &mut items);
                } else {
                    self.forming = Some(bar);
                    items.push(StreamedBar::Forming(bar));
                }
            }
            HistoricalDataFrame::End { .. } => {}
        }
        items
    }

    /// Completes the forming bar if the local clock has passed its end plus `grace`; `None`
    /// otherwise.
    ///
    /// For a bar that no later update completes. Live, TWS sent the batch and then no update
    /// at all for contracts without real-time permission (ES futures, DAX and ESTX50 indices)
    /// and for AAPL and NVDA overnight, so [`apply`](Self::apply) left the batch's last bar
    /// forming for good: for ES at 11:23 UTC, the 21:59 UTC bar of the day before
    /// (2026-11-24, server 223). The same holds at a session break: after a batch ending with
    /// the 21:59 UTC EUR.USD bar, TWS sent nothing until the next session's first bar at 22:15.
    /// At the SEHK close TWS did send the last bar's final update, 1.2 s after the bar ended, so
    /// [`apply`](Self::apply) completed it without `expire` (live: 2026-11-25).
    ///
    /// The bar is completed with the last values received, which are final only if TWS sent
    /// its last update for the bar. TWS sent that update 0.3 to 2.6 s after a 1-minute bar ended,
    /// so a `grace` below about 3 s can complete a bar before its final values arrive. An update
    /// for the bar after `expire` completed it is dropped, so no bar is completed twice. Daily
    /// and longer bars have no end to expire at ([`forming_end`](Self::forming_end) is `None`).
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::market_data::historical::{Bar, BarCompletion, BarSize, BarTimestamp, HistoricalDataFrame};
    /// use time::macros::datetime;
    ///
    /// let mut completion = BarCompletion::new(BarSize::Min);
    /// let bar = Bar { date: BarTimestamp::DateTime(datetime!(2026-04-14 9:31 UTC)), ..Bar::default() };
    /// completion.apply(HistoricalDataFrame::Bars(vec![bar]));
    ///
    /// // Long after 9:32, with no update since the batch, the forming bar completes.
    /// assert_eq!(completion.expire(time::Duration::seconds(10)), Some(bar));
    /// assert_eq!(completion.expire(time::Duration::seconds(10)), None);
    /// ```
    pub fn expire(&mut self, grace: time::Duration) -> Option<Bar> {
        self.expire_at(grace, OffsetDateTime::now_utc())
    }

    fn expire_at(&mut self, grace: time::Duration, now: OffsetDateTime) -> Option<Bar> {
        if self.forming_end().is_none_or(|end| now < end + grace) {
            return None;
        }
        let bar = self.forming.take()?;
        self.completed_through = Some(bar.date);
        Some(bar)
    }

    /// The end of the forming bar, its start plus the bar size; `None` when no bar is forming or
    /// for daily and longer bars, whose end follows the trading session.
    ///
    /// The time to call [`expire`](Self::expire) is this end plus the grace passed to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::market_data::historical::{Bar, BarCompletion, BarSize, BarTimestamp, HistoricalDataFrame};
    /// use time::macros::datetime;
    ///
    /// let mut completion = BarCompletion::new(BarSize::Min);
    /// assert_eq!(completion.forming_end(), None);
    ///
    /// let bar = Bar { date: BarTimestamp::DateTime(datetime!(2026-04-14 9:31 UTC)), ..Bar::default() };
    /// completion.apply(HistoricalDataFrame::Bars(vec![bar]));
    /// assert_eq!(completion.forming_end(), Some(datetime!(2026-04-14 9:32 UTC)));
    /// ```
    pub fn forming_end(&self) -> Option<OffsetDateTime> {
        self.end(&self.forming?)
    }

    fn is_new(&self, bar: &Bar) -> bool {
        self.completed_through.is_none_or(|through| bar.date > through)
    }

    fn complete(&mut self, bar: Bar, items: &mut Vec<StreamedBar>) {
        if self.is_new(&bar) {
            self.completed_through = Some(bar.date);
            items.push(StreamedBar::Completed(bar));
        }
    }

    fn has_ended(&self, bar: &Bar, now: OffsetDateTime) -> bool {
        self.end(bar).is_some_and(|end| now >= end)
    }

    fn end(&self, bar: &Bar) -> Option<OffsetDateTime> {
        match (bar.date, self.span) {
            (BarTimestamp::DateTime(start), Some(span)) => Some(start + span),
            _ => None,
        }
    }
}

/// The length of a `bar_size` bar; `None` for daily and longer bars, which follow the session.
fn span(bar_size: BarSize) -> Option<time::Duration> {
    let seconds = match bar_size {
        BarSize::Sec => 1,
        BarSize::Sec5 => 5,
        BarSize::Sec10 => 10,
        BarSize::Sec15 => 15,
        BarSize::Sec30 => 30,
        BarSize::Min => 60,
        BarSize::Min2 => 2 * 60,
        BarSize::Min3 => 3 * 60,
        BarSize::Min4 => 4 * 60,
        BarSize::Min5 => 5 * 60,
        BarSize::Min10 => 10 * 60,
        BarSize::Min15 => 15 * 60,
        BarSize::Min20 => 20 * 60,
        BarSize::Min30 => 30 * 60,
        BarSize::Hour => 3600,
        BarSize::Hour2 => 2 * 3600,
        BarSize::Hour3 => 3 * 3600,
        BarSize::Hour4 => 4 * 3600,
        BarSize::Hour8 => 8 * 3600,
        BarSize::Day | BarSize::Week | BarSize::Month => return None,
    };
    Some(time::Duration::seconds(seconds))
}
