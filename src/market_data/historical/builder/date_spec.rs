//! Typestate for the date range of a
//! [`HistoricalDataBuilder`](super::HistoricalDataBuilder).
//!
//! The builder's third type parameter records which of
//! [`duration`](super::HistoricalDataBuilder::duration),
//! [`ending`](super::HistoricalDataBuilder::ending) and
//! [`between`](super::HistoricalDataBuilder::between) have been called. The
//! terminals exist only on a complete spec, so a request with no range, or one
//! mixing `between` with `duration` / `ending`, fails to compile:
//!
//! | State | Reached by | Terminals |
//! |---|---|---|
//! | [`Missing`] | `historical_data(..)` | none |
//! | [`EndingOnly`] | `.ending(t)` | none; add `.duration(d)` |
//! | [`DurationOnly`] | `.duration(d)` | `fetch`, `subscribe`, `subscribe_with_updates` |
//! | [`DurationEnding`] | `.duration(d).ending(t)` or `.ending(t).duration(d)` | `fetch`, `subscribe` |
//! | [`Between`] | `.between(start, end)` | `fetch`, `subscribe` |
//!
//! The builder starts at [`Missing`], the crate's shared typestate start marker.
//! The other states are nameable, for a function that takes or returns a builder,
//! but only the builder constructs them. A function that accepts a builder in any
//! complete state is generic over [`DateSpec`].

use time::OffsetDateTime;

use crate::contracts::{Contract, SecurityType};
use crate::market_data::historical::Duration;
use crate::Error;

#[cfg(doc)]
use crate::builder::Missing;

/// An end date without a duration: call `duration` to complete it.
#[derive(Debug, Clone, Copy)]
pub struct EndingOnly(pub(super) OffsetDateTime);

/// A duration ending now. The only state with `subscribe_with_updates`.
#[derive(Debug, Clone, Copy)]
pub struct DurationOnly(pub(super) Duration);

/// A duration ending at an explicit end date.
#[derive(Debug, Clone, Copy)]
pub struct DurationEnding {
    pub(super) duration: Duration,
    pub(super) ending: OffsetDateTime,
}

/// An explicit `[start, end)` range; the result is trimmed to it.
#[derive(Debug, Clone, Copy)]
pub struct Between {
    pub(super) start: OffsetDateTime,
    pub(super) end: OffsetDateTime,
}

/// A complete date spec: [`DurationOnly`], [`DurationEnding`] or [`Between`].
///
/// Sealed; bounds the builder's `fetch` and `subscribe`.
pub trait DateSpec: sealed::Sealed {}

impl DateSpec for DurationOnly {}
impl DateSpec for DurationEnding {}
impl DateSpec for Between {}

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

/// The `(end_date, duration)` of a [`between`](super::HistoricalDataBuilder::between) request for a
/// continuous future. TWS rejects any end date for one (error 10339), so the request covers
/// `[start, now)` and the trim to `[start, end)` drops the bars after `end`.
fn continuous_future_range(start: OffsetDateTime, now: OffsetDateTime) -> Result<(Option<OffsetDateTime>, Duration), Error> {
    if start >= now {
        return Err(Error::InvalidArgument(
            "historical_data: .between(start, end) on a continuous future requires start before now".to_owned(),
        ));
    }
    Ok((None, covering_duration(now - start)))
}

pub(super) use sealed::Resolved;

mod sealed {
    use super::*;

    /// A date spec resolved for the wire. `pub` in a private module, so
    /// nameable only inside the crate.
    pub struct Resolved {
        pub end_date: Option<OffsetDateTime>,
        pub duration: Duration,
        /// The `between` range to trim the result to.
        pub range: Option<(OffsetDateTime, OffsetDateTime)>,
    }

    pub trait Sealed {
        /// The request's end date and duration, and the range to trim to. Errors
        /// for what only a runtime value can show: `end <= start`, or an end
        /// date TWS rejects for a continuous future.
        fn resolve(&self, contract: &Contract) -> Result<Resolved, Error>;
    }

    impl Sealed for DurationOnly {
        fn resolve(&self, _contract: &Contract) -> Result<Resolved, Error> {
            Ok(Resolved {
                end_date: None,
                duration: self.0,
                range: None,
            })
        }
    }

    impl Sealed for DurationEnding {
        fn resolve(&self, contract: &Contract) -> Result<Resolved, Error> {
            if contract.security_type == SecurityType::ContinuousFuture {
                return Err(Error::InvalidArgument(
                    "historical_data: TWS rejects .ending() for a continuous future (error 10339); use .duration() alone".to_owned(),
                ));
            }
            Ok(Resolved {
                end_date: Some(self.ending),
                duration: self.duration,
                range: None,
            })
        }
    }

    impl Sealed for Between {
        fn resolve(&self, contract: &Contract) -> Result<Resolved, Error> {
            let Between { start, end } = *self;
            if end <= start {
                return Err(Error::InvalidArgument(
                    "historical_data: .between(start, end) requires end > start".to_owned(),
                ));
            }
            let (end_date, duration) = if contract.security_type == SecurityType::ContinuousFuture {
                continuous_future_range(start, OffsetDateTime::now_utc())?
            } else {
                (Some(end), covering_duration(end - start))
            };
            Ok(Resolved {
                end_date,
                duration,
                range: Some((start, end)),
            })
        }
    }
}

#[cfg(test)]
#[path = "date_spec_tests.rs"]
mod tests;
