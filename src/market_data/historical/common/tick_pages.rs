//! Shared paging for historical ticks over a time range.
//!
//! TWS answers a historical ticks request ending at `end` with ticks before `end`, read at whole
//! seconds: a fractional `end` is truncated and the end second itself is excluded. A page asked
//! for `n` ticks holds whole seconds, so it can exceed `n`, except at its earliest second: a trade
//! page holds every trade of that second, but a bid/ask or midpoint page holds only one of its
//! ticks. [`TickPages`] pages backward under those rules: each request ends where the previous
//! page's complete seconds begin, which neither skips nor repeats a tick. See the wire facts in
//! `docs/wire-facts/market-data.md`.

use time::{Duration, OffsetDateTime};

use crate::market_data::{TickBidAsk, TickLast, TickMidpoint};
use crate::Error;

/// A historical tick type, as [`TickPages`] pages it.
pub(crate) trait PagedTick {
    /// Whether a page holds every tick of its earliest second.
    const PAGE_HOLDS_EARLIEST_SECOND: bool;

    fn time(&self) -> OffsetDateTime;
}

impl PagedTick for TickLast {
    const PAGE_HOLDS_EARLIEST_SECOND: bool = true;

    fn time(&self) -> OffsetDateTime {
        self.time
    }
}

impl PagedTick for TickBidAsk {
    const PAGE_HOLDS_EARLIEST_SECOND: bool = false;

    fn time(&self) -> OffsetDateTime {
        self.time
    }
}

impl PagedTick for TickMidpoint {
    const PAGE_HOLDS_EARLIEST_SECOND: bool = false;

    fn time(&self) -> OffsetDateTime {
        self.time
    }
}

/// The pages of ticks fetched backward over `[start, end]`, and the end of the next request.
pub(crate) struct TickPages<T> {
    start: OffsetDateTime,
    end: OffsetDateTime,
    request_end: OffsetDateTime,
    pages: Vec<Vec<T>>,
}

impl<T: PagedTick> TickPages<T> {
    /// Starts paging over `[start, end]`. The first request ends one second after `end`'s whole
    /// second, as TWS excludes the end second.
    pub(crate) fn new(start: OffsetDateTime, end: OffsetDateTime) -> Result<Self, Error> {
        if end <= start {
            return Err(Error::InvalidArgument(
                "historical_ticks: .between(start, end) requires end > start".to_owned(),
            ));
        }

        Ok(Self {
            start,
            end,
            request_end: whole_second(end) + Duration::SECOND,
            pages: Vec::new(),
        })
    }

    /// The end of the next request.
    pub(crate) fn request_end(&self) -> OffsetDateTime {
        self.request_end
    }

    /// Adds the ticks a request ending at [`request_end`](Self::request_end) returned, and
    /// returns whether another request is needed.
    ///
    /// The page keeps its complete seconds: ticks at or after the request end are dropped, and so
    /// is a partial earliest second, which the next request fetches whole. Paging stops at a page
    /// that holds no ticks or whose complete seconds reach back to `start`.
    pub(crate) fn push(&mut self, page: Vec<T>) -> bool {
        let Some(earliest) = page.iter().map(PagedTick::time).filter(|time| *time < self.request_end).min() else {
            return false;
        };

        let earliest_second = whole_second(earliest);
        let mut complete_from = if T::PAGE_HOLDS_EARLIEST_SECOND {
            earliest_second
        } else {
            earliest_second + Duration::SECOND
        };
        // A page whose only second is partial cannot move the end back; keep it as it is
        if complete_from >= self.request_end {
            complete_from = earliest_second;
        }

        let (from, to) = (complete_from, self.request_end);
        self.pages
            .push(page.into_iter().filter(|tick| (from..to).contains(&tick.time())).collect());

        if complete_from <= self.start {
            return false;
        }

        self.request_end = complete_from;
        true
    }

    /// The ticks in `[start, end]`, oldest first.
    pub(crate) fn into_ticks(self) -> Vec<T> {
        let (start, end) = (self.start, self.end);
        self.pages
            .into_iter()
            .rev()
            .flatten()
            .filter(|tick| (start..=end).contains(&tick.time()))
            .collect()
    }
}

fn whole_second(time: OffsetDateTime) -> OffsetDateTime {
    time.replace_nanosecond(0).expect("zero is a valid nanosecond")
}

#[cfg(test)]
#[path = "tick_pages_tests.rs"]
mod tests;
