//! Resubscribe a stream across transport reconnects: the packaged form of the
//! `is_connection_lost` → `wait_connected` → resubscribe loop. Also reopens a
//! keep-up-to-date bar request that 10182 ended once the historical-data farms
//! are back, and any stream after a 1101 (connectivity restored, data lost).
//!
//! Entry points are `Client::resubscribing` (sync and async). Only types marked
//! [`Resubscribable`] are accepted, so the helper can't re-send a request that
//! places, cancels or exercises.

use std::time::Duration;

use crate::errors::Error;
use crate::messages::{resubscribed_notice, Notice, SUBSCRIPTION_DISCONNECTED_CODE};
use crate::subscriptions::{StreamDecoder, SubscriptionItem};
use crate::transport::farm_watch::{FarmMark, FarmRecovery};

/// The longest wait for the historical-data farms after a 10182 before
/// reopening anyway, in case TWS never reports the farm back. A request sent
/// while the farm is still down got no frame and no error live, and whether
/// TWS answers it once the farm is back is unknown, so the reopened stream
/// is reopened once more when TWS confirms the farms back before its first
/// bar. Live, the farm came back 2 s to 19 min 41 s after the 10182, with the
/// outage; in the longest run the reopen at this limit got no answer and the
/// second reopen, at the farm's OK, delivered the whole outage.
const FARM_RECOVERY_LIMIT: Duration = Duration::from_secs(15 * 60);

/// The wait after a 10182 that arrives with no historical-data farm reported
/// down. Nothing then says when the farm is back, and a TWS that answered
/// every reopen with 10182 would otherwise get a new request at once, over
/// and over. Not seen live: every 10182 in the outage runs followed a 2105
/// for its HMDS farm.
const FARM_REOPEN_MINIMUM: Duration = Duration::from_secs(30);

/// How long to wait for the historical-data farms after a 10182 marked at
/// `mark`: up to `limit` for a farm that is down, the shorter
/// [`FARM_REOPEN_MINIMUM`] when none is.
fn farm_wait(mark: FarmMark, limit: Duration) -> Duration {
    if mark.farm_down() {
        limit
    } else {
        FARM_REOPEN_MINIMUM.min(limit)
    }
}

/// Stream item types that are safe to resubscribe after a reconnect.
///
/// Sealed. The accepted streams, and what each repeats after a resubscribe, are
/// listed on `Client::resubscribing`. Order types are excluded (a resubscribe
/// would place, cancel or exercise again), and so are requests that end on their
/// own (contract details, option chains). Historical bars are the exception:
/// `subscribe` shares [`HistoricalDataFrame`](crate::market_data::historical::HistoricalDataFrame)
/// with `subscribe_with_updates`, so a `subscribe` cut by a reconnect before
/// `End` is sent again from the start; the stream ends after `End`. Display
/// groups are left out: their
/// `DisplayGroupSubscription` wrapper isn't a `Subscription`.
#[allow(private_bounds)]
pub trait Resubscribable: StreamDecoder<Self> + Sized {}

macro_rules! resubscribable {
    ($($ty:ty),* $(,)?) => {
        $(impl Resubscribable for $ty {})*
    };
}

resubscribable!(
    crate::market_data::realtime::TickTypes,
    crate::market_data::realtime::Bar,
    crate::market_data::TickLast,
    crate::market_data::TickBidAsk,
    crate::market_data::TickMidpoint,
    crate::market_data::realtime::MarketDepths,
    crate::market_data::historical::HistoricalDataFrame,
    crate::accounts::AccountUpdate,
    crate::accounts::AccountUpdateMulti,
    crate::accounts::AccountSummaryResult,
    crate::accounts::PositionUpdate,
    crate::accounts::PositionUpdateMulti,
    crate::accounts::PnL,
    crate::accounts::PnLSingle,
    Vec<crate::scanner::ScannerData>,
    crate::news::NewsBulletin,
);

/// Where the loop is. Shared by the sync iterator and the async stream.
enum State<S> {
    /// Opening the first or a replacement subscription.
    Opening(Reopen),
    /// Reading `S`, whose request went out after `FarmMark`: a 1101 since voids
    /// it. While the flag is set, no bar has arrived on a request sent before
    /// TWS confirmed the farms back, and a confirmation since reopens it.
    Reading(S, FarmMark, bool),
    /// A 10182 ended the request: waiting for the historical-data farms from `mark`.
    Waiting(FarmMark),
    Done,
}

impl<S> State<S> {
    fn after_item<T>(subscription: S, mark: FarmMark, unconfirmed: bool, item: &SubscriptionItem<T>, snapshot_ended: bool) -> Self {
        if snapshot_ended {
            Self::Done
        } else {
            Self::Reading(subscription, mark, unconfirmed && !matches!(item, SubscriptionItem::Data(_)))
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reopen {
    Normal,
    Transport,
    BeforeFarmConfirmation,
}

impl From<FarmRecovery> for Reopen {
    fn from(recovery: FarmRecovery) -> Self {
        match recovery {
            FarmRecovery::Recovered => Self::Normal,
            FarmRecovery::Unconfirmed | FarmRecovery::TimedOut => Self::BeforeFarmConfirmation,
            FarmRecovery::SessionEnded => Self::Transport,
        }
    }
}

enum Read<T> {
    Item(SubscriptionItem<T>),
    Reconnect,
    WaitForFarm(Notice),
    Failed(Error),
    End,
}

impl<T> From<Option<Result<SubscriptionItem<T>, Error>>> for Read<T> {
    fn from(next: Option<Result<SubscriptionItem<T>, Error>>) -> Self {
        match next {
            Some(Ok(item)) => Self::Item(item),
            Some(Err(error)) if error.is_connection_lost() => Self::Reconnect,
            Some(Err(Error::Notice(notice))) if notice.code == SUBSCRIPTION_DISCONNECTED_CODE => Self::WaitForFarm(notice),
            Some(Err(error)) => Self::Failed(error),
            None => Self::End,
        }
    }
}

/// What to do with an `open()` result.
enum Opened<S> {
    Subscribed(S),
    /// Connection lost before the request went out: wait, then open again.
    Lost,
    Failed(Error),
}

impl<S> From<Result<S, Error>> for Opened<S> {
    fn from(result: Result<S, Error>) -> Self {
        match result {
            Ok(subscription) => Opened::Subscribed(subscription),
            Err(e) if e.is_connection_lost() => Opened::Lost,
            Err(e) => Opened::Failed(e),
        }
    }
}

/// Marks where a resubscribe replaced a subscription the reconnect killed.
fn gap_notice<T>() -> SubscriptionItem<T> {
    SubscriptionItem::Notice(resubscribed_notice())
}

#[cfg(feature = "sync")]
pub use sync_impl::Resubscribing;

#[cfg(feature = "sync")]
mod sync_impl {
    use std::time::Instant;

    use super::*;
    use crate::client::sync::Client;
    use crate::subscriptions::sync::Subscription;
    use crate::transport::farm_watch::sync_impl::FarmSignal;

    /// Blocking iterator returned by
    /// [`Client::resubscribing`](crate::client::blocking::Client::resubscribing).
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub struct Resubscribing<'a, T: Resubscribable, F> {
        client: &'a Client,
        open: F,
        state: State<Subscription<T>>,
        /// Wakes a read blocked on the subscription when the farm state changes.
        farm_changed: FarmSignal,
        farm_recovery_limit: Duration,
    }

    impl<'a, T: Resubscribable, F> Resubscribing<'a, T, F> {
        pub(crate) fn new(client: &'a Client, open: F) -> Self {
            Self::within(client, open, FARM_RECOVERY_LIMIT)
        }

        /// [`new`](Self::new) with `farm_recovery_limit` in place of [`FARM_RECOVERY_LIMIT`].
        pub(super) fn within(client: &'a Client, open: F, farm_recovery_limit: Duration) -> Self {
            Self {
                client,
                open,
                state: State::Opening(Reopen::Normal),
                farm_changed: client.message_bus.farm_watch().signal(),
                farm_recovery_limit,
            }
        }
    }

    impl<T, F> Iterator for Resubscribing<'_, T, F>
    where
        T: Resubscribable,
        F: FnMut() -> Result<Subscription<T>, Error>,
    {
        type Item = Result<SubscriptionItem<T>, Error>;

        fn next(&mut self) -> Option<Self::Item> {
            let farms = self.client.message_bus.farm_watch();
            loop {
                // Done unless an arm puts a live state back.
                match std::mem::replace(&mut self.state, State::Done) {
                    State::Done => return None,
                    State::Opening(reopen) => {
                        // Before the request goes out, so a 1101 while it does counts.
                        let mark = farms.mark();
                        match Opened::from((self.open)()) {
                            Opened::Subscribed(subscription) => {
                                self.state = State::Reading(subscription, mark, reopen == Reopen::BeforeFarmConfirmation);
                                if reopen == Reopen::Transport {
                                    return Some(Ok(gap_notice()));
                                }
                            }
                            Opened::Lost => {
                                if let Err(e) = self.client.wait_connected() {
                                    return Some(Err(e));
                                }
                                self.state = State::Opening(reopen);
                            }
                            Opened::Failed(e) => return Some(Err(e)),
                        }
                    }
                    State::Reading(subscription, mark, unconfirmed) => {
                        if let Some(notice) = farms.data_lost_since(mark) {
                            // Written on this thread, so before the reopen's request.
                            subscription.cancel();
                            self.state = State::Opening(Reopen::Normal);
                            return Some(Ok(SubscriptionItem::Notice(notice)));
                        }
                        if unconfirmed && farms.confirmed(mark) {
                            subscription.cancel();
                            self.state = State::Opening(Reopen::Normal);
                            continue;
                        }
                        let Some(next) = subscription.next_unless(self.farm_changed.receiver()) else {
                            self.state = State::Reading(subscription, mark, unconfirmed);
                            continue;
                        };
                        match Read::from(next) {
                            Read::Reconnect => {
                                // Its request died with the old session; dropping it sends no cancel.
                                drop(subscription);
                                if let Err(e) = self.client.wait_connected() {
                                    return Some(Err(e));
                                }
                                self.state = State::Opening(Reopen::Transport);
                            }
                            Read::Item(item) => {
                                let ended = subscription.snapshot_ended();
                                self.state = State::after_item(subscription, mark, unconfirmed, &item, ended);
                                return Some(Ok(item));
                            }
                            Read::WaitForFarm(notice) => {
                                let mark = subscription.farm_disconnect_mark().unwrap_or_else(|| farms.mark());
                                self.state = State::Waiting(mark);
                                return Some(Ok(SubscriptionItem::Notice(notice)));
                            }
                            Read::Failed(e) => return Some(Err(e)),
                            Read::End => return None,
                        }
                    }
                    State::Waiting(mark) => {
                        let deadline = Instant::now() + farm_wait(mark, self.farm_recovery_limit);
                        let reopen = Reopen::from(farms.wait(mark, deadline));
                        if reopen == Reopen::Transport {
                            if let Err(e) = self.client.wait_connected() {
                                return Some(Err(e));
                            }
                        }
                        self.state = State::Opening(reopen);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "async")]
pub(crate) use async_impl::resubscribing;

#[cfg(feature = "async")]
mod async_impl {
    use std::future::Future;

    use futures::{Stream, StreamExt};

    use super::*;
    use crate::client::r#async::Client;
    use crate::subscriptions::r#async::Subscription;

    /// The stream behind the async `Client::resubscribing`.
    pub(crate) fn resubscribing<'a, T, F, Fut>(client: &'a Client, open: F) -> impl Stream<Item = Result<SubscriptionItem<T>, Error>> + 'a
    where
        T: Resubscribable + Send + 'static,
        F: FnMut() -> Fut + 'a,
        Fut: Future<Output = Result<Subscription<T>, Error>> + 'a,
    {
        resubscribing_within(client, open, FARM_RECOVERY_LIMIT)
    }

    /// [`resubscribing`] with `farm_recovery_limit` in place of [`FARM_RECOVERY_LIMIT`].
    pub(super) fn resubscribing_within<'a, T, F, Fut>(
        client: &'a Client,
        open: F,
        farm_recovery_limit: Duration,
    ) -> impl Stream<Item = Result<SubscriptionItem<T>, Error>> + 'a
    where
        T: Resubscribable + Send + 'static,
        F: FnMut() -> Fut + 'a,
        Fut: Future<Output = Result<Subscription<T>, Error>> + 'a,
    {
        let farms = client.message_bus.farm_watch();
        futures::stream::unfold((State::Opening(Reopen::Normal), open), move |(mut state, mut open)| async move {
            loop {
                match state {
                    State::Done => return None,
                    State::Opening(reopen) => {
                        // Before the request goes out, so a 1101 while it does counts.
                        let mark = farms.mark();
                        match Opened::from(open().await) {
                            Opened::Subscribed(subscription) => {
                                state = State::Reading(subscription, mark, reopen == Reopen::BeforeFarmConfirmation);
                                if reopen == Reopen::Transport {
                                    return Some((Ok(gap_notice()), (state, open)));
                                }
                            }
                            Opened::Lost => {
                                if let Err(e) = client.wait_connected().await {
                                    return Some((Err(e), (State::Done, open)));
                                }
                                // `state` is still `Opening`: the match only copied its fields.
                            }
                            Opened::Failed(e) => return Some((Err(e), (State::Done, open))),
                        }
                    }
                    State::Reading(mut subscription, mark, unconfirmed) => {
                        let next = tokio::select! {
                            // A 1101 first: the old request's queued items are void with it.
                            biased;
                            notice = farms.data_lost(mark) => Err(Some(notice)),
                            () = farms.confirmed(mark), if unconfirmed => Err(None),
                            item = subscription.next() => Ok(item),
                        };
                        let next = match next {
                            Ok(next) => next,
                            Err(notice) => {
                                // Awaited: a dropped subscription's cancel is spawned, and
                                // could reach TWS after the reopen's request.
                                subscription.cancel().await;
                                state = State::Opening(Reopen::Normal);
                                match notice {
                                    Some(notice) => return Some((Ok(SubscriptionItem::Notice(notice)), (state, open))),
                                    None => continue,
                                }
                            }
                        };
                        match Read::from(next) {
                            Read::Reconnect => {
                                // Its request died with the old session; dropping it sends no cancel.
                                drop(subscription);
                                if let Err(e) = client.wait_connected().await {
                                    return Some((Err(e), (State::Done, open)));
                                }
                                state = State::Opening(Reopen::Transport);
                            }
                            Read::Item(item) => {
                                let ended = subscription.snapshot_ended();
                                let state = State::after_item(subscription, mark, unconfirmed, &item, ended);
                                return Some((Ok(item), (state, open)));
                            }
                            Read::WaitForFarm(notice) => {
                                let mark = subscription.farm_disconnect_mark().unwrap_or_else(|| farms.mark());
                                let state = State::Waiting(mark);
                                return Some((Ok(SubscriptionItem::Notice(notice)), (state, open)));
                            }
                            Read::Failed(e) => return Some((Err(e), (State::Done, open))),
                            Read::End => return None,
                        }
                    }
                    State::Waiting(mark) => {
                        let deadline = tokio::time::Instant::now() + farm_wait(mark, farm_recovery_limit);
                        let reopen = Reopen::from(farms.wait(mark, deadline).await);
                        if reopen == Reopen::Transport {
                            if let Err(e) = client.wait_connected().await {
                                return Some((Err(e), (State::Done, open)));
                            }
                        }
                        state = State::Opening(reopen);
                    }
                }
            }
        })
    }
}

#[cfg(test)]
#[path = "resubscribe_tests.rs"]
mod tests;
