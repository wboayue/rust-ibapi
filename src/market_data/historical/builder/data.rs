use time::OffsetDateTime;

use crate::client::ids::RequestId;
use crate::contracts::Contract;
use crate::market_data::historical::{
    Bar, BarRequest, BarSize, BarTimestamp, BarsBetweenError, Duration, HistoricalData, HistoricalDataFrame, WhatToShow,
};
use crate::market_data::TradingHours;
use crate::messages::HMDS_ERROR_CODE;
use crate::Error;

use super::date_spec::{covering_duration, max_request_span, Between, DateSpec, DurationEnding, DurationOnly, EndingOnly, Resolved};
use crate::builder::Missing;

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;

/// Builder for historical bar data requests.
///
/// Required: a date range, either [`duration`](Self::duration) (with optional
/// [`ending`](Self::ending), in either order) or [`between`](Self::between). The
/// third type parameter records which (see [`date_spec`](super::date_spec)): the
/// terminals exist only once the range is complete, and `between` does not
/// combine with `duration` or `ending`.
///
/// Three terminals: `fetch` collects the bars, retrying after a connection
/// reset; `subscribe` sends once and hands back the frames;
/// `subscribe_with_updates`, only after `duration` alone, also keeps the bars
/// up to date.
#[must_use = "HistoricalDataBuilder does nothing until you call .fetch(), .subscribe() or .subscribe_with_updates()"]
pub struct HistoricalDataBuilder<'a, C, D = Missing> {
    client: &'a C,
    contract: &'a Contract,
    request_id: RequestId,
    bar_size: BarSize,
    what_to_show: WhatToShow,
    trading_hours: TradingHours,
    dates: D,
    buffer_limit: Option<usize>,
}

impl<'a, C> HistoricalDataBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract, bar_size: BarSize, request_id: RequestId) -> Self {
        Self {
            client,
            contract,
            request_id,
            bar_size,
            what_to_show: WhatToShow::Trades,
            trading_hours: TradingHours::Regular,
            dates: Missing,
            buffer_limit: None,
        }
    }

    /// Amount of data going back from the end date: now, unless [`ending`](Self::ending)
    /// follows.
    pub fn duration(self, duration: Duration) -> HistoricalDataBuilder<'a, C, DurationOnly> {
        self.with_dates(DurationOnly(duration))
    }

    /// Anchor the query at a specific end date (defaults to now); [`duration`](Self::duration)
    /// must follow before a terminal. TWS rejects an end date for a
    /// [`ContinuousFuture`](crate::contracts::SecurityType::ContinuousFuture) (error 10339), so
    /// for one the request fails with [`Error::InvalidArgument`] and nothing is sent; use
    /// `duration` alone or [`between`](Self::between) instead.
    pub fn ending(self, end_date: OffsetDateTime) -> HistoricalDataBuilder<'a, C, EndingOnly> {
        self.with_dates(EndingOnly(end_date))
    }

    /// Convenience: specify an explicit date range.
    ///
    /// IBKR measures request durations in trading time, so no duration maps exactly onto a
    /// wall-clock range. The request asks for a duration that covers `[start, end)`, and the
    /// result is trimmed to that range with [`HistoricalData::trim_to`], whose docs say which
    /// bar at each edge survives for intraday, daily, weekly and monthly bars.
    ///
    /// TWS rejects any end date for a
    /// [`ContinuousFuture`](crate::contracts::SecurityType::ContinuousFuture) (error 10339), so
    /// for one the request has no end date and covers `[start, now)`; the bars after `end` are
    /// then dropped. Such a range must start before now. The request size grows with
    /// `now - start`, not `end - start`: a short range far in the past fetches every bar since
    /// `start`. A continuous future's one-second request cannot be split, so a start more than
    /// 2000 seconds before now returns [`Error::InvalidArgument`] without sending.
    ///
    /// For other contracts, `fetch` splits a range of one-second bars over 2000 seconds into
    /// requests of up to 2000 seconds each. A full 6.5-hour U.S. equities regular session takes
    /// 12 requests when each full page covers exactly 2000 seconds of wall-clock time.
    /// `subscribe` sends a single request and rejects
    /// a range that needs splitting. Other bar sizes are never split. TWS still applies its
    /// bar-size step limit (error 162), and a large request is slow: a month of 1-minute ES
    /// bars took over a minute (live: 2026-11-22).
    ///
    /// The range is complete: `between` takes no `duration` or `ending`.
    ///
    /// ```compile_fail,E0599
    /// # use ibapi::market_data::historical::{HistoricalDataBuilder, ToDuration};
    /// # use time::macros::datetime;
    /// # fn f<C>(request: HistoricalDataBuilder<'_, C>) {
    /// let request = request
    ///     .between(datetime!(2026-04-08 0:00 UTC), datetime!(2026-04-15 0:00 UTC))
    ///     .duration(7.days());
    /// # }
    /// ```
    pub fn between(self, start: OffsetDateTime, end: OffsetDateTime) -> HistoricalDataBuilder<'a, C, Between> {
        self.with_dates(Between { start, end })
    }
}

impl<'a, C> HistoricalDataBuilder<'a, C, EndingOnly> {
    /// Amount of data going back from the end date set by
    /// [`ending`](HistoricalDataBuilder::ending).
    pub fn duration(self, duration: Duration) -> HistoricalDataBuilder<'a, C, DurationEnding> {
        let ending = self.dates.0;
        self.with_dates(DurationEnding { duration, ending })
    }
}

impl<'a, C> HistoricalDataBuilder<'a, C, DurationOnly> {
    /// Anchor the query at a specific end date instead of now, with the same
    /// continuous-future restriction as [`ending`](HistoricalDataBuilder::ending)
    /// before `duration`.
    pub fn ending(self, end_date: OffsetDateTime) -> HistoricalDataBuilder<'a, C, DurationEnding> {
        let duration = self.dates.0;
        self.with_dates(DurationEnding { duration, ending: end_date })
    }
}

impl<'a, C, D> HistoricalDataBuilder<'a, C, D> {
    /// The request id the terminal will send. Allocated when the builder was
    /// made; nothing has been written yet. A dropped builder skips the id.
    /// `fetch` sends its first attempt under it, and a resend after a
    /// connection reset under a new one.
    pub fn request_id(&self) -> i32 {
        self.request_id.raw()
    }

    /// Override the data type to retrieve (defaults to [`WhatToShow::Trades`]).
    pub fn what_to_show(mut self, what_to_show: WhatToShow) -> Self {
        self.what_to_show = what_to_show;
        self
    }

    /// Override regular- vs extended-hours (defaults to [`TradingHours::Regular`]).
    pub fn trading_hours(mut self, trading_hours: TradingHours) -> Self {
        self.trading_hours = trading_hours;
        self
    }

    /// Fail the subscription from `subscribe` or `subscribe_with_updates` instead of queueing
    /// more than `limit` unread items.
    ///
    /// When `limit` items are unread and another arrives, the subscription
    /// yields every queued item, then [`Error::BufferLimitExceeded`], then
    /// ends. Same semantics and caveats as
    /// [`ContractDetailsBuilder::buffer_limit`](crate::contracts::ContractDetailsBuilder::buffer_limit),
    /// with one difference: `subscribe_with_updates` has no end marker (the
    /// [`HistoricalDataFrame::End`] after the initial bars is an item and
    /// counts), so a reader that stops reading always overflows eventually.
    /// On `subscribe`, `Bars` and `End` are two items: with a `limit` of 1,
    /// `End` overflows unless `Bars` was read first.
    /// `limit` must be `1..=`[`MAX_BUFFER_LIMIT`](crate::subscriptions::MAX_BUFFER_LIMIT).
    ///
    /// [`fetch`](Self::fetch) returns a single response, so it rejects a
    /// builder with a `buffer_limit` ([`Error::InvalidArgument`]).
    ///
    /// # Examples
    #[cfg_attr(
        feature = "sync",
        doc = r#"
```no_run
use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, HistoricalDataFrame, ToDuration};
use ibapi::Error;

let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

let contract = Contract::stock("SPY").build();
let subscription = client
    .historical_data(&contract, BarSize::Min15)
    .duration(1.days())
    .buffer_limit(64)
    .subscribe_with_updates()
    .expect("streaming request failed");
for update in subscription.iter_data() {
    match update {
        Ok(HistoricalDataFrame::Update(bar)) => println!("{bar:?}"),
        Ok(other) => println!("{other:?}"),
        Err(Error::BufferLimitExceeded { limit }) => {
            eprintln!("fell {limit} updates behind; stopping");
            break;
        }
        Err(e) => {
            eprintln!("error: {e}");
            break;
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
use ibapi::market_data::historical::HistoricalDataFrame;
use ibapi::Error;

#[tokio::main]
async fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");

    let contract = Contract::stock("SPY").build();
    let subscription = client
        .historical_data(&contract, HistoricalBarSize::Min15)
        .duration(1.days())
        .buffer_limit(64)
        .subscribe_with_updates()
        .await
        .expect("streaming request failed");
    let mut updates = subscription.filter_data();
    while let Some(update) = updates.next().await {
        match update {
            Ok(HistoricalDataFrame::Update(bar)) => println!("{bar:?}"),
            Ok(other) => println!("{other:?}"),
            Err(Error::BufferLimitExceeded { limit }) => {
                eprintln!("fell {limit} updates behind; stopping");
                break;
            }
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        }
    }
}
```
"#
    )]
    pub fn buffer_limit(mut self, limit: usize) -> Self {
        self.buffer_limit = Some(limit);
        self
    }

    fn with_dates<E>(self, dates: E) -> HistoricalDataBuilder<'a, C, E> {
        HistoricalDataBuilder {
            client: self.client,
            contract: self.contract,
            request_id: self.request_id,
            bar_size: self.bar_size,
            what_to_show: self.what_to_show,
            trading_hours: self.trading_hours,
            dates,
            buffer_limit: self.buffer_limit,
        }
    }

    /// The request's parameters, with the resolved date spec.
    fn bar_request(&self, end_date: Option<OffsetDateTime>, duration: Duration) -> BarRequest<'a> {
        BarRequest {
            contract: self.contract,
            end_date,
            duration,
            bar_size: self.bar_size,
            what_to_show: self.what_to_show,
            trading_hours: self.trading_hours,
        }
    }
}

impl<C, D: DateSpec> HistoricalDataBuilder<'_, C, D> {
    /// Resolve the builder for a one-shot `fetch`: the date spec, and no
    /// `buffer_limit` (a single response has nothing to cap).
    fn resolve_for_fetch(&self) -> Result<Resolved, Error> {
        if self.buffer_limit.is_some() {
            return Err(Error::InvalidArgument(
                "historical_data().fetch(): buffer_limit applies to .subscribe() and .subscribe_with_updates() only".to_owned(),
            ));
        }
        self.dates.resolve(self.contract, self.bar_size)
    }

    fn bar_pages(&self, resolved: &Resolved) -> Option<BarPages> {
        let (start, end) = resolved.range?;
        // Continuous futures have no wire end date and cannot use the pager.
        resolved.end_date?;
        let limit = max_request_span(self.bar_size).filter(|limit| end - start > *limit)?;
        Some(BarPages::new(start, end, limit))
    }

    fn bar_pages_for_fetch(&self, resolved: &Resolved) -> Result<Option<BarPages>, Error> {
        let Some(pages) = self.bar_pages(resolved) else {
            return Ok(None);
        };
        let max_span = pages.limit * SPLIT_MAX_REQUESTS as i32;
        if pages.end - pages.start > max_span {
            return Err(Error::InvalidArgument(format!(
                "historical_data().fetch(): a .between(..) range of {} bars over {} S needs more than {SPLIT_MAX_REQUESTS} requests of {} S; split it",
                self.bar_size,
                max_span.whole_seconds(),
                pages.limit.whole_seconds()
            )));
        }
        Ok(Some(pages))
    }

    fn resolve_for_subscribe(&self) -> Result<Resolved, Error> {
        let resolved = self.dates.resolve(self.contract, self.bar_size)?;
        if let Some(pages) = self.bar_pages(&resolved) {
            return Err(Error::InvalidArgument(format!(
                "historical_data().subscribe(): TWS serves at most {} S of {} bars per request (error 162); .fetch() splits a longer .between(..) range",
                pages.limit.whole_seconds(),
                self.bar_size
            )));
        }
        Ok(resolved)
    }
}

/// The most requests `fetch` splits a range into: half of IB's pacing limit of 60 requests for
/// bars of 30 seconds or less in any ten minutes, which other requests on the account share.
const SPLIT_MAX_REQUESTS: i64 = 30;
/// The least time between the starts of two split requests. IB's pacing rules refuse six or more
/// requests for the same contract and `whatToShow` within two seconds.
const SPLIT_REQUEST_SPACING: std::time::Duration = std::time::Duration::from_millis(500);

/// The requests of a [`between`](HistoricalDataBuilder::between) range that `fetch` splits to
/// stay within TWS's request limit, sent from `end` backward, and the bars they returned.
///
/// TWS counts a duration in trading time back from the end date and serves the bars before it:
/// a request ending at 09:40 on a Monday also returns the previous Friday's last bars. So each
/// request ends at the earliest bar the one before returned, which neither skips nor repeats a
/// bar, or one limit before its own end in wall-clock time if that is earlier: a duration in
/// trading time covers at least as much wall-clock time, so the bars in between, if any, came
/// back already. Each request so moves back at least one limit, and a range of `n` limits takes
/// at most `n` requests. The requests stop once one covers `start`.
struct BarPages {
    start: OffsetDateTime,
    end: OffsetDateTime,
    limit: time::Duration,
    /// The end of the next request; `None` once the requests reach `start`.
    request_end: Option<OffsetDateTime>,
    pages: Vec<Vec<Bar>>,
}

impl BarPages {
    fn new(start: OffsetDateTime, end: OffsetDateTime, limit: time::Duration) -> Self {
        Self {
            start,
            end,
            limit,
            request_end: Some(end),
            pages: Vec::new(),
        }
    }

    /// The end date and duration of the next request, or `None` once the requests reach `start`.
    fn next_request(&self) -> Option<(OffsetDateTime, Duration)> {
        let end = self.request_end?;
        Some((end, covering_duration((end - self.start).min(self.limit))))
    }

    fn with_response(mut self, result: Result<HistoricalData, Error>) -> Result<Self, Error> {
        const NO_DATA_PREFIX: &str = "Historical Market Data Service error message:HMDS query returned no data: ";

        let bars = match result {
            Ok(data) => data.bars,
            Err(Error::Notice(notice)) if notice.code == HMDS_ERROR_CODE && notice.message.starts_with(NO_DATA_PREFIX) => Vec::new(),
            Err(e) => return Err(self.fail(e)),
        };
        self.push(bars);
        Ok(self)
    }

    /// Adds the bars the request from [`next_request`](Self::next_request) returned. Bars at or
    /// after its end belong to the request before and are dropped.
    fn push(&mut self, bars: Vec<Bar>) {
        let Some(request_end) = self.request_end else {
            return;
        };
        let before = BarTimestamp::from(request_end);
        let bars: Vec<Bar> = bars.into_iter().filter(|bar| bar.date < before).collect();
        let next_end = match bars.iter().map(|bar| bar.date).min() {
            Some(BarTimestamp::DateTime(earliest)) => earliest,
            Some(BarTimestamp::Date(earliest)) => earliest.midnight().assume_utc(),
            None => request_end,
        }
        .min(request_end - self.limit);
        let covered_start = request_end - self.start <= self.limit || next_end <= self.start;
        self.request_end = (!covered_start).then_some(next_end);
        self.pages.push(bars);
    }

    /// The bars in `[start, end)`, oldest first.
    fn finish(self) -> HistoricalData {
        let (start, end) = (self.start, self.end);
        self.fetched(start).trim_to(start, end)
    }

    /// The error a failed request returns: as it is for the first request, and as
    /// [`Error::BarsBetween`] with the bars fetched before it for a later one.
    fn fail(self, error: Error) -> Error {
        match (self.pages.is_empty(), self.request_end) {
            (false, Some(from)) => {
                let end = self.end;
                let fetched = self.fetched(from).trim_to(from, end);
                Error::BarsBetween(Box::new(BarsBetweenError { error, fetched }))
            }
            _ => error,
        }
    }

    fn fetched(self, start: OffsetDateTime) -> HistoricalData {
        HistoricalData {
            start,
            end: self.end,
            bars: self.pages.into_iter().rev().flatten().collect(),
        }
    }
}

/// How long to wait before the next split request, `elapsed` after the previous one started.
fn split_request_wait(elapsed: std::time::Duration) -> std::time::Duration {
    SPLIT_REQUEST_SPACING.saturating_sub(elapsed)
}

/// Drop bars outside `[start, end)` when the request came from [`between`](HistoricalDataBuilder::between).
fn trim_to_range(data: HistoricalData, range: Option<(OffsetDateTime, OffsetDateTime)>) -> HistoricalData {
    match range {
        Some((start, end)) => data.trim_to(start, end),
        None => data,
    }
}

#[cfg(feature = "sync")]
impl<'a, D: DateSpec> HistoricalDataBuilder<'a, crate::client::sync::Client, D> {
    /// Submit a one-shot request and return the [`HistoricalData`] bars.
    ///
    /// Succeeds only after TWS sends its end marker; an empty result with the marker is valid.
    ///
    /// For a `between` range over 2000 seconds of one-second bars, this fetches up to
    /// 30 pages of at most 2000 seconds, with at least 0.5 seconds between requests. Each
    /// page has its own one-shot timeout. Empty HMDS no-data replies count as empty pages.
    /// A later request failure returns [`Error::BarsBetween`] with the part already covered.
    ///
    /// # Errors
    /// - [`Error::InvalidArgument`] for a range the client rejects before sending (see [`between`](HistoricalDataBuilder::between)
    ///   and [`ending`](HistoricalDataBuilder::ending)), or if
    ///   [`buffer_limit`](Self::buffer_limit) is set; nothing is sent.
    /// - [`Error::Notice`] when TWS rejects the request (e.g. 162, no data).
    /// - [`Error::UnexpectedEndOfStream`] if the response closes before the end
    ///   marker, so a partial download is never returned as complete.
    /// - [`Error::UnexpectedResponse`] if the bars and the end marker arrive out of order.
    /// - [`Error::Timeout`] if the end marker hasn't arrived within the client's
    ///   [`one_shot_timeout`](crate::client::blocking::ClientBuilder::one_shot_timeout).
    ///
    /// Without a `one_shot_timeout`, waits without a time limit. Any error
    /// before the end marker cancels the request at TWS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{BarSize, ToDuration};
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// // 7 days of hourly bars, ending now:
    /// let bars = client
    ///     .historical_data(&contract, BarSize::Hour)
    ///     .duration(7.days())
    ///     .fetch()
    ///     .expect("historical data request failed");
    ///
    /// // Equivalent via explicit date range:
    /// let bars = client
    ///     .historical_data(&contract, BarSize::Hour)
    ///     .between(datetime!(2023-04-08 0:00 UTC), datetime!(2023-04-15 0:00 UTC))
    ///     .fetch()
    ///     .expect("historical data request failed");
    /// # let _ = bars;
    /// ```
    ///
    /// Without a date range there is no `fetch`:
    ///
    /// ```compile_fail,E0599
    /// # use ibapi::client::blocking::Client;
    /// # use ibapi::contracts::Contract;
    /// # use ibapi::market_data::historical::BarSize;
    /// # fn f(client: &Client, contract: &Contract) {
    /// let bars = client.historical_data(contract, BarSize::Hour).fetch();
    /// # }
    /// ```
    pub fn fetch(self) -> Result<HistoricalData, Error> {
        let resolved = self.resolve_for_fetch()?;
        if let Some(mut pages) = self.bar_pages_for_fetch(&resolved)? {
            let mut first_id = Some(self.request_id);
            let mut last_sent: Option<std::time::Instant> = None;
            while let Some((end_date, duration)) = pages.next_request() {
                if let Some(sent) = last_sent {
                    std::thread::sleep(split_request_wait(sent.elapsed()));
                }
                last_sent = Some(std::time::Instant::now());
                let request_id = first_id.take().unwrap_or_else(|| self.client.mint_request_id());
                let result =
                    crate::market_data::historical::sync::historical_data(self.client, self.bar_request(Some(end_date), duration), request_id);
                pages = pages.with_response(result)?;
            }
            return Ok(pages.finish());
        }
        let data = crate::market_data::historical::sync::historical_data(
            self.client,
            self.bar_request(resolved.end_date, resolved.duration),
            self.request_id,
        )?;
        Ok(trim_to_range(data, resolved.range))
    }

    /// Send the request once and return a subscription of its
    /// [`HistoricalDataFrame`]s: [`Bars`](HistoricalDataFrame::Bars), then
    /// [`End`](HistoricalDataFrame::End), the last frame. Stop reading on `End`:
    /// the subscription stays open after it, so a further read waits.
    ///
    /// Unlike [`fetch`](Self::fetch), the caller owns the request: its id is
    /// [`request_id`](Self::request_id), it is never resent (after a connection
    /// reset, build a new request, or open it through `Client::resubscribing`),
    /// and only the caller bounds the wait.
    /// [`ending`](Self::ending), [`between`](Self::between) and
    /// [`buffer_limit`](Self::buffer_limit) all apply. The bars come as TWS sent
    /// them: after `between`, build a [`HistoricalData`] from them and the `End`
    /// window and call [`trim_to`](HistoricalData::trim_to).
    ///
    /// Dropping the subscription before `End` sends `cancelHistoricalData`. To
    /// know when TWS is done with the request, use `cancel_and_drain`. Any TWS
    /// error ends a historical request, so after one it returns `Rejected`
    /// without writing a cancel. Observed live (server 226), TWS answers a
    /// cancel of a request still running with error 162 ("API historical data
    /// query cancelled"), so `cancel_and_drain` returns `Rejected` with code
    /// 162: the request is over at TWS.
    ///
    /// # Errors
    /// [`Error::InvalidArgument`], with nothing sent, for a range the client rejects before sending (see
    /// [`between`](HistoricalDataBuilder::between) and [`ending`](HistoricalDataBuilder::ending)),
    /// [`WhatToShow::AdjustedLast`] with an end date, or a `buffer_limit`
    /// outside `1..=`[`MAX_BUFFER_LIMIT`](crate::subscriptions::MAX_BUFFER_LIMIT).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{BarSize, HistoricalData, HistoricalDataFrame};
    /// use ibapi::subscriptions::SubscriptionItem;
    /// use std::time::{Duration, Instant};
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    /// let (start, end) = (datetime!(2023-04-10 0:00 UTC), datetime!(2023-04-15 0:00 UTC));
    ///
    /// let request = client.historical_data(&contract, BarSize::Hour).between(start, end);
    /// println!("request id: {}", request.request_id());
    /// let subscription = request.subscribe().expect("request failed");
    ///
    /// // Bars read before a timed-out wait are kept; the next read resumes.
    /// let deadline = Instant::now() + Duration::from_secs(30);
    /// let mut bars = None;
    /// loop {
    ///     let remaining = deadline.saturating_duration_since(Instant::now());
    ///     match subscription.next_timeout(remaining) {
    ///         Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(received)))) => bars = Some(received),
    ///         Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End { start: served_start, end: served_end }))) => {
    ///             if let Some(bars) = bars.take() {
    ///                 let data = HistoricalData { start: served_start, end: served_end, bars };
    ///                 println!("{} bars", data.trim_to(start, end).bars.len());
    ///             }
    ///             break;
    ///         }
    ///         // Only `subscribe_with_updates` yields updates.
    ///         Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Update(_)))) => {}
    ///         Some(Ok(SubscriptionItem::Notice(notice))) => eprintln!("notice: {notice}"),
    ///         Some(Err(e)) => {
    ///             eprintln!("error: {e}");
    ///             break;
    ///         }
    ///         None if subscription.is_terminated() => break,
    ///         None => {
    ///             // Timed out: cancel, and wait a little to learn whether TWS is done.
    ///             let outcome = subscription.cancel_and_drain(Instant::now() + Duration::from_secs(5));
    ///             eprintln!("timed out: {outcome:?}");
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    pub fn subscribe(self) -> Result<crate::subscriptions::sync::Subscription<HistoricalDataFrame>, Error> {
        let resolved = self.resolve_for_subscribe()?;
        crate::market_data::historical::sync::historical_data_subscription(
            self.client,
            self.bar_request(resolved.end_date, resolved.duration),
            self.request_id,
            self.buffer_limit,
        )
    }
}

#[cfg(feature = "sync")]
impl HistoricalDataBuilder<'_, crate::client::sync::Client, DurationOnly> {
    /// Submit a request with IBKR's `keepUpToDate` and return a
    /// [`Subscription`](crate::subscriptions::Subscription) of [`HistoricalDataFrame`]:
    /// `Bars`, `End`, then `Update` for the forming bar until cancelled.
    ///
    /// Only after [`duration`](HistoricalDataBuilder::duration) alone: IBKR requires
    /// `end_date = None` for streaming, so a builder with
    /// [`ending`](HistoricalDataBuilder::ending) or [`between`](HistoricalDataBuilder::between)
    /// has no `subscribe_with_updates`.
    ///
    /// ```compile_fail,E0599
    /// # use ibapi::client::blocking::Client;
    /// # use ibapi::contracts::Contract;
    /// # use ibapi::market_data::historical::{BarSize, ToDuration};
    /// # use time::macros::datetime;
    /// # fn f(client: &Client, contract: &Contract) {
    /// let subscription = client
    ///     .historical_data(contract, BarSize::Min15)
    ///     .duration(1.days())
    ///     .ending(datetime!(2026-04-15 0:00 UTC))
    ///     .subscribe_with_updates();
    /// # }
    /// ```
    ///
    /// # Errors
    /// [`Error::InvalidArgument`], with nothing sent, for a `buffer_limit` outside
    /// `1..=`[`MAX_BUFFER_LIMIT`](crate::subscriptions::MAX_BUFFER_LIMIT).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::{BarSize, ToDuration};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("SPY").build();
    ///
    /// let subscription = client
    ///     .historical_data(&contract, BarSize::Min15)
    ///     .duration(1.days())
    ///     .subscribe_with_updates()
    ///     .expect("streaming request failed");
    /// # drop(subscription);
    /// ```
    pub fn subscribe_with_updates(self) -> Result<crate::subscriptions::sync::Subscription<HistoricalDataFrame>, Error> {
        crate::market_data::historical::sync::historical_data_updates(
            self.client,
            self.bar_request(None, self.dates.0),
            self.request_id,
            self.buffer_limit,
        )
    }
}

#[cfg(feature = "async")]
impl<'a, D: DateSpec> HistoricalDataBuilder<'a, crate::client::r#async::Client, D> {
    /// Submit a one-shot request and return the [`HistoricalData`] bars.
    ///
    /// Succeeds only after TWS sends its end marker; an empty result with the marker is valid.
    ///
    /// For a `between` range over 2000 seconds of one-second bars, this fetches up to
    /// 30 pages of at most 2000 seconds, with at least 0.5 seconds between requests. Each
    /// page has its own one-shot timeout. Empty HMDS no-data replies count as empty pages.
    /// A later request failure returns [`Error::BarsBetween`] with the part already covered.
    ///
    /// # Errors
    /// - [`Error::InvalidArgument`] for a range the client rejects before sending (see [`between`](HistoricalDataBuilder::between)
    ///   and [`ending`](HistoricalDataBuilder::ending)), or if
    ///   [`buffer_limit`](Self::buffer_limit) is set; nothing is sent.
    /// - [`Error::Notice`] when TWS rejects the request (e.g. 162, no data).
    /// - [`Error::UnexpectedEndOfStream`] if the response closes before the end
    ///   marker, so a partial download is never returned as complete.
    /// - [`Error::UnexpectedResponse`] if the bars and the end marker arrive out of order.
    /// - [`Error::Timeout`] if the end marker hasn't arrived within the client's
    ///   [`one_shot_timeout`](crate::ClientBuilder::one_shot_timeout).
    ///
    /// Without a `one_shot_timeout`, waits without a time limit; to bound one
    /// call, wrap the future in `tokio::time::timeout`. Dropping it, or any
    /// error, before the end marker cancels the request at TWS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     // 7 days of hourly bars, ending now:
    ///     let bars = client
    ///         .historical_data(&contract, HistoricalBarSize::Hour)
    ///         .duration(7.days())
    ///         .fetch()
    ///         .await
    ///         .expect("historical data request failed");
    ///
    ///     // Equivalent via explicit date range:
    ///     let bars = client
    ///         .historical_data(&contract, HistoricalBarSize::Hour)
    ///         .between(datetime!(2023-04-08 0:00 UTC), datetime!(2023-04-15 0:00 UTC))
    ///         .fetch()
    ///         .await
    ///         .expect("historical data request failed");
    ///     let _ = bars;
    /// }
    /// ```
    ///
    /// Without a date range there is no `fetch`:
    ///
    /// ```compile_fail,E0599
    /// # use ibapi::client::Client;
    /// # use ibapi::contracts::Contract;
    /// # use ibapi::market_data::historical::BarSize;
    /// # async fn f(client: &Client, contract: &Contract) {
    /// let bars = client.historical_data(contract, BarSize::Hour).fetch().await;
    /// # }
    /// ```
    pub async fn fetch(self) -> Result<HistoricalData, Error> {
        let resolved = self.resolve_for_fetch()?;
        if let Some(mut pages) = self.bar_pages_for_fetch(&resolved)? {
            let mut first_id = Some(self.request_id);
            let mut last_sent: Option<tokio::time::Instant> = None;
            while let Some((end_date, duration)) = pages.next_request() {
                if let Some(sent) = last_sent {
                    tokio::time::sleep(split_request_wait(sent.elapsed())).await;
                }
                last_sent = Some(tokio::time::Instant::now());
                let request_id = first_id.take().unwrap_or_else(|| self.client.mint_request_id());
                let result =
                    crate::market_data::historical::r#async::historical_data(self.client, self.bar_request(Some(end_date), duration), request_id)
                        .await;
                pages = pages.with_response(result)?;
            }
            return Ok(pages.finish());
        }
        let data = crate::market_data::historical::r#async::historical_data(
            self.client,
            self.bar_request(resolved.end_date, resolved.duration),
            self.request_id,
        )
        .await?;
        Ok(trim_to_range(data, resolved.range))
    }

    /// Send the request once and return a subscription of its
    /// [`HistoricalDataFrame`]s: [`Bars`](HistoricalDataFrame::Bars), then
    /// [`End`](HistoricalDataFrame::End), the last frame. Stop reading on `End`:
    /// the subscription stays open after it, so a further read waits.
    ///
    /// Unlike [`fetch`](Self::fetch), the caller owns the request: its id is
    /// [`request_id`](Self::request_id), it is never resent (after a connection
    /// reset, build a new request, or open it through `Client::resubscribing`),
    /// and only the caller bounds the wait.
    /// [`ending`](Self::ending), [`between`](Self::between) and
    /// [`buffer_limit`](Self::buffer_limit) all apply. The bars come as TWS sent
    /// them: after `between`, build a [`HistoricalData`] from them and the `End`
    /// window and call [`trim_to`](HistoricalData::trim_to).
    ///
    /// Dropping the subscription before `End` sends `cancelHistoricalData`. To
    /// know when TWS is done with the request, use `cancel_and_drain`. Any TWS
    /// error ends a historical request, so after one it returns `Rejected`
    /// without writing a cancel. Observed live (server 226), TWS answers a
    /// cancel of a request still running with error 162 ("API historical data
    /// query cancelled"), so `cancel_and_drain` returns `Rejected` with code
    /// 162: the request is over at TWS.
    ///
    /// # Errors
    /// [`Error::InvalidArgument`], with nothing sent, for a range the client rejects before sending (see
    /// [`between`](HistoricalDataBuilder::between) and [`ending`](HistoricalDataBuilder::ending)),
    /// [`WhatToShow::AdjustedLast`] with an end date, or a `buffer_limit`
    /// outside `1..=`[`MAX_BUFFER_LIMIT`](crate::subscriptions::MAX_BUFFER_LIMIT).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use ibapi::market_data::historical::{HistoricalData, HistoricalDataFrame};
    /// use time::macros::datetime;
    /// use tokio::time::{timeout_at, Duration, Instant};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///     let (start, end) = (datetime!(2023-04-10 0:00 UTC), datetime!(2023-04-15 0:00 UTC));
    ///
    ///     let request = client.historical_data(&contract, HistoricalBarSize::Hour).between(start, end);
    ///     println!("request id: {}", request.request_id());
    ///     let mut subscription = request.subscribe().await.expect("request failed");
    ///
    ///     // `next()` is cancel-safe: bars read before a timed-out wait are kept.
    ///     let deadline = Instant::now() + Duration::from_secs(30);
    ///     let mut bars = None;
    ///     loop {
    ///         match timeout_at(deadline, subscription.next()).await {
    ///             Ok(Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(received))))) => bars = Some(received),
    ///             Ok(Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End { start: served_start, end: served_end })))) => {
    ///                 if let Some(bars) = bars.take() {
    ///                     let data = HistoricalData { start: served_start, end: served_end, bars };
    ///                     println!("{} bars", data.trim_to(start, end).bars.len());
    ///                 }
    ///                 break;
    ///             }
    ///             // Only `subscribe_with_updates` yields updates.
    ///             Ok(Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Update(_))))) => {}
    ///             Ok(Some(Ok(SubscriptionItem::Notice(notice)))) => eprintln!("notice: {notice}"),
    ///             Ok(Some(Err(e))) => {
    ///                 eprintln!("error: {e}");
    ///                 break;
    ///             }
    ///             Ok(None) => break,
    ///             Err(_elapsed) => {
    ///                 // Timed out: cancel, and wait a little to learn whether TWS is done.
    ///                 let outcome = subscription.cancel_and_drain(Instant::now() + Duration::from_secs(5)).await;
    ///                 eprintln!("timed out: {outcome:?}");
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn subscribe(self) -> Result<crate::subscriptions::r#async::Subscription<HistoricalDataFrame>, Error> {
        let resolved = self.resolve_for_subscribe()?;
        crate::market_data::historical::r#async::historical_data_subscription(
            self.client,
            self.bar_request(resolved.end_date, resolved.duration),
            self.request_id,
            self.buffer_limit,
        )
        .await
    }
}

#[cfg(feature = "async")]
impl HistoricalDataBuilder<'_, crate::client::r#async::Client, DurationOnly> {
    /// Submit a request with IBKR's `keepUpToDate` and return a
    /// [`Subscription`](crate::subscriptions::Subscription) of [`HistoricalDataFrame`]:
    /// `Bars`, `End`, then `Update` for the forming bar until cancelled.
    ///
    /// Only after [`duration`](HistoricalDataBuilder::duration) alone: IBKR requires
    /// `end_date = None` for streaming, so a builder with
    /// [`ending`](HistoricalDataBuilder::ending) or [`between`](HistoricalDataBuilder::between)
    /// has no `subscribe_with_updates`.
    ///
    /// ```compile_fail,E0599
    /// # use ibapi::client::Client;
    /// # use ibapi::contracts::Contract;
    /// # use ibapi::market_data::historical::BarSize;
    /// # use time::macros::datetime;
    /// # async fn f(client: &Client, contract: &Contract) {
    /// let subscription = client
    ///     .historical_data(contract, BarSize::Min15)
    ///     .between(datetime!(2026-04-08 0:00 UTC), datetime!(2026-04-15 0:00 UTC))
    ///     .subscribe_with_updates()
    ///     .await;
    /// # }
    /// ```
    ///
    /// # Errors
    /// [`Error::InvalidArgument`], with nothing sent, for a `buffer_limit` outside
    /// `1..=`[`MAX_BUFFER_LIMIT`](crate::subscriptions::MAX_BUFFER_LIMIT).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("SPY").build();
    ///
    ///     let subscription = client
    ///         .historical_data(&contract, HistoricalBarSize::Min15)
    ///         .duration(1.days())
    ///         .subscribe_with_updates()
    ///         .await
    ///         .expect("streaming request failed");
    ///     drop(subscription);
    /// }
    /// ```
    pub async fn subscribe_with_updates(self) -> Result<crate::subscriptions::r#async::Subscription<HistoricalDataFrame>, Error> {
        crate::market_data::historical::r#async::historical_data_updates(
            self.client,
            self.bar_request(None, self.dates.0),
            self.request_id,
            self.buffer_limit,
        )
        .await
    }
}
