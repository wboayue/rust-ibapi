use time::OffsetDateTime;

use crate::client::ids::RequestId;
use crate::contracts::Contract;
use crate::market_data::historical::{BarRequest, BarSize, Duration, HistoricalBarUpdate, HistoricalData, HistoricalDataFrame, WhatToShow};
use crate::market_data::TradingHours;
use crate::Error;

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;

/// Builder for historical bar data requests.
///
/// Required: one of [`duration`](Self::duration) (with optional [`ending`](Self::ending))
/// or [`between`](Self::between) to specify the time range. Mixing the two styles
/// errors at the terminal.
///
/// Three terminals: `fetch` collects the bars, retrying after a connection
/// reset; `subscribe` sends once and hands back the frames; `stream` keeps the
/// bars up to date.
#[must_use = "HistoricalDataBuilder does nothing until you call .fetch(), .subscribe() or .stream()"]
pub struct HistoricalDataBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    request_id: RequestId,
    bar_size: BarSize,
    what_to_show: WhatToShow,
    trading_hours: TradingHours,
    duration: Option<Duration>,
    ending: Option<OffsetDateTime>,
    between: Option<(OffsetDateTime, OffsetDateTime)>,
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
            duration: None,
            ending: None,
            between: None,
            buffer_limit: None,
        }
    }

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

    /// Amount of data going back from the end date (now if [`ending`](Self::ending) is unset).
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Anchor the query at a specific end date (defaults to now).
    pub fn ending(mut self, end_date: OffsetDateTime) -> Self {
        self.ending = Some(end_date);
        self
    }

    /// Convenience: specify an explicit date range.
    ///
    /// IBKR measures request durations in trading time, so no duration maps exactly onto a
    /// wall-clock range. The request asks for a duration that covers `[start, end)`, and bars
    /// timestamped outside that range are dropped from the result. Bars are stamped with their
    /// start time, so a bar that begins before `start` is dropped even if it overlaps the range.
    /// Daily and longer bars carry only a date and compare as midnight UTC: a `start` after
    /// midnight UTC drops that day's bar.
    pub fn between(mut self, start: OffsetDateTime, end: OffsetDateTime) -> Self {
        self.between = Some((start, end));
        self
    }

    /// Fail the subscription from `stream` or `subscribe` instead of queueing
    /// more than `limit` unread items.
    ///
    /// When `limit` items are unread and another arrives, the subscription
    /// yields every queued item, then [`Error::BufferLimitExceeded`], then
    /// ends. Same semantics and caveats as
    /// [`ContractDetailsBuilder::buffer_limit`](crate::contracts::ContractDetailsBuilder::buffer_limit),
    /// with one difference: this stream has no end marker (the
    /// [`HistoricalBarUpdate::End`] after the initial bars is an item and
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
use ibapi::market_data::historical::{BarSize, HistoricalBarUpdate, ToDuration};
use ibapi::Error;

let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

let contract = Contract::stock("SPY").build();
let subscription = client
    .historical_data(&contract, BarSize::Min15)
    .duration(1.days())
    .buffer_limit(64)
    .stream()
    .expect("streaming request failed");
for update in subscription.iter_data() {
    match update {
        Ok(HistoricalBarUpdate::Update(bar)) => println!("{bar:?}"),
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
use ibapi::market_data::historical::HistoricalBarUpdate;
use ibapi::Error;

#[tokio::main]
async fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");

    let contract = Contract::stock("SPY").build();
    let subscription = client
        .historical_data(&contract, HistoricalBarSize::Min15)
        .duration(1.days())
        .buffer_limit(64)
        .stream()
        .await
        .expect("streaming request failed");
    let mut updates = subscription.filter_data();
    while let Some(update) = updates.next().await {
        match update {
            Ok(HistoricalBarUpdate::Update(bar)) => println!("{bar:?}"),
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

    /// Resolve the builder for a one-shot [`fetch`](Self::fetch): the date spec,
    /// and no `buffer_limit` (a single response has nothing to cap).
    fn resolve_for_fetch(&self) -> Result<(Option<OffsetDateTime>, Duration), Error> {
        if self.buffer_limit.is_some() {
            return Err(Error::InvalidArgument(
                "historical_data().fetch(): buffer_limit applies to .stream() and .subscribe() only".to_owned(),
            ));
        }
        self.resolve_date_spec()
    }

    /// Resolve the builder's date spec into (end_date, duration). Errors if the
    /// user mixed `.between` with `.duration`/`.ending`, or set neither.
    fn resolve_date_spec(&self) -> Result<(Option<OffsetDateTime>, Duration), Error> {
        match (self.between, self.duration, self.ending) {
            (Some(_), Some(_), _) | (Some(_), _, Some(_)) => Err(Error::InvalidArgument(
                "historical_data: cannot mix .between(...) with .duration()/.ending()".to_owned(),
            )),
            (Some((start, end)), None, None) => {
                if end <= start {
                    return Err(Error::InvalidArgument(
                        "historical_data: .between(start, end) requires end > start".to_owned(),
                    ));
                }
                Ok((Some(end), covering_duration(end - start)))
            }
            (None, Some(duration), ending) => Ok((ending, duration)),
            (None, None, _) => Err(Error::InvalidArgument(
                "historical_data: must set .duration() or .between(...)".to_owned(),
            )),
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

    /// Resolve the builder for a streaming request (`keep_up_to_date = true`).
    /// IBKR requires `end_date = None` for streaming, so `.ending()` / `.between()`
    /// are rejected.
    fn resolve_for_stream(&self) -> Result<Duration, Error> {
        if self.ending.is_some() || self.between.is_some() {
            return Err(Error::InvalidArgument(
                "historical_data().stream(): IBKR requires end_date = None for streaming; drop .ending() / .between()".to_owned(),
            ));
        }
        self.duration
            .ok_or_else(|| Error::InvalidArgument("historical_data().stream(): must set .duration()".to_owned()))
    }
}

const SECONDS_PER_DAY: u64 = 86_400;
/// IBKR rejects `S` durations above one day and `D` durations above 365.
const MAX_DAYS: u64 = 365;

/// An IBKR duration that covers a wall-clock `span` ending at the request's end date.
///
/// IBKR counts `S` in trading seconds, `D` in whole sessions counted back from the
/// session containing the end date, and `Y` in calendar years. Trading time never exceeds
/// wall-clock time, so `S` and `D` over-fetch rather than fall short, provided `D` asks for
/// one extra session to cover a range that starts partway through one. `span` is positive.
fn covering_duration(span: time::Duration) -> Duration {
    let seconds = span.whole_seconds().unsigned_abs() + u64::from(span.subsec_nanoseconds() != 0);
    if seconds <= SECONDS_PER_DAY {
        return Duration::seconds(seconds as i32);
    }
    let days = seconds.div_ceil(SECONDS_PER_DAY) + 1;
    if days <= MAX_DAYS {
        return Duration::days(days as i32);
    }
    Duration::years(seconds.div_ceil(MAX_DAYS * SECONDS_PER_DAY) as i32)
}

/// Drop bars outside `[start, end)` when the request came from [`between`](HistoricalDataBuilder::between).
fn trim_to_range(data: HistoricalData, range: Option<(OffsetDateTime, OffsetDateTime)>) -> HistoricalData {
    match range {
        Some((start, end)) => data.trim_to(start, end),
        None => data,
    }
}

#[cfg(feature = "sync")]
impl<'a> HistoricalDataBuilder<'a, crate::client::sync::Client> {
    /// Submit a one-shot request and return the [`HistoricalData`] bars.
    ///
    /// Succeeds only after TWS sends its end marker; an empty result with the marker is valid.
    ///
    /// # Errors
    /// - [`Error::InvalidArgument`] for an invalid date range, or if
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
    pub fn fetch(self) -> Result<HistoricalData, Error> {
        let (end_date, duration) = self.resolve_for_fetch()?;
        let data = crate::market_data::historical::sync::historical_data(self.client, self.bar_request(end_date, duration), self.request_id)?;
        Ok(trim_to_range(data, self.between))
    }

    /// Send the request once and return a subscription of its
    /// [`HistoricalDataFrame`]s: [`Bars`](HistoricalDataFrame::Bars), then
    /// [`End`](HistoricalDataFrame::End), after which the stream ends.
    ///
    /// Unlike [`fetch`](Self::fetch), the caller owns the request: its id is
    /// [`request_id`](Self::request_id), it is never resent (after a connection
    /// reset, build a new request), and only the caller bounds the wait.
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
    /// [`Error::InvalidArgument`], with nothing sent, for an invalid date range,
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
        let (end_date, duration) = self.resolve_date_spec()?;
        crate::market_data::historical::sync::historical_data_subscription(
            self.client,
            self.bar_request(end_date, duration),
            self.request_id,
            self.buffer_limit,
        )
    }

    /// Submit a streaming request (`keep_up_to_date = true`) and return a
    /// [`Subscription`](crate::subscriptions::Subscription) of [`HistoricalBarUpdate`].
    /// IBKR sends initial bars, then per-bar updates as they form.
    ///
    /// Rejects builders that called [`ending`](Self::ending) or [`between`](Self::between) —
    /// IBKR requires `end_date = None` for streaming.
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
    ///     .stream()
    ///     .expect("streaming request failed");
    /// # drop(subscription);
    /// ```
    pub fn stream(self) -> Result<crate::subscriptions::sync::Subscription<HistoricalBarUpdate>, Error> {
        let duration = self.resolve_for_stream()?;
        crate::market_data::historical::sync::historical_data_stream(
            self.client,
            self.bar_request(None, duration),
            self.request_id,
            self.buffer_limit,
        )
    }
}

#[cfg(feature = "async")]
impl<'a> HistoricalDataBuilder<'a, crate::client::r#async::Client> {
    /// Submit a one-shot request and return the [`HistoricalData`] bars.
    ///
    /// Succeeds only after TWS sends its end marker; an empty result with the marker is valid.
    ///
    /// # Errors
    /// - [`Error::InvalidArgument`] for an invalid date range, or if
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
    pub async fn fetch(self) -> Result<HistoricalData, Error> {
        let (end_date, duration) = self.resolve_for_fetch()?;
        let data =
            crate::market_data::historical::r#async::historical_data(self.client, self.bar_request(end_date, duration), self.request_id).await?;
        Ok(trim_to_range(data, self.between))
    }

    /// Send the request once and return a subscription of its
    /// [`HistoricalDataFrame`]s: [`Bars`](HistoricalDataFrame::Bars), then
    /// [`End`](HistoricalDataFrame::End), after which the stream ends.
    ///
    /// Unlike [`fetch`](Self::fetch), the caller owns the request: its id is
    /// [`request_id`](Self::request_id), it is never resent (after a connection
    /// reset, build a new request), and only the caller bounds the wait.
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
    /// [`Error::InvalidArgument`], with nothing sent, for an invalid date range,
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
        let (end_date, duration) = self.resolve_date_spec()?;
        crate::market_data::historical::r#async::historical_data_subscription(
            self.client,
            self.bar_request(end_date, duration),
            self.request_id,
            self.buffer_limit,
        )
        .await
    }

    /// Submit a streaming request (`keep_up_to_date = true`) and return a
    /// [`Subscription`](crate::subscriptions::Subscription) of [`HistoricalBarUpdate`].
    ///
    /// Rejects builders that called [`ending`](Self::ending) or [`between`](Self::between) —
    /// IBKR requires `end_date = None` for streaming.
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
    ///         .stream()
    ///         .await
    ///         .expect("streaming request failed");
    ///     drop(subscription);
    /// }
    /// ```
    pub async fn stream(self) -> Result<crate::subscriptions::r#async::Subscription<HistoricalBarUpdate>, Error> {
        let duration = self.resolve_for_stream()?;
        crate::market_data::historical::r#async::historical_data_stream(
            self.client,
            self.bar_request(None, duration),
            self.request_id,
            self.buffer_limit,
        )
        .await
    }
}
