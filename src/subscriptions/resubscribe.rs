//! Resubscribe a stream across transport reconnects: the packaged form of the
//! `is_connection_lost` → `wait_connected` → resubscribe loop.
//!
//! Entry points are `Client::resubscribing` (sync and async). Only types marked
//! [`Resubscribable`] are accepted, so the helper can't re-send a request that
//! places, cancels or exercises.

use crate::errors::Error;
use crate::messages::resubscribed_notice;
use crate::subscriptions::{StreamDecoder, SubscriptionItem};

/// Stream item types that are safe to resubscribe after a reconnect.
///
/// Sealed: implemented only for streaming market data, account, position, PnL,
/// scanner, news bulletin and display group subscriptions. Order types are
/// excluded (a resubscribe would place, cancel or exercise again), and so are
/// requests that end on their own (contract details, option chains, historical
/// bars without `keep_up_to_date`).
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
    crate::market_data::realtime::Trade,
    crate::market_data::realtime::BidAsk,
    crate::market_data::realtime::MidPoint,
    crate::market_data::realtime::MarketDepths,
    crate::market_data::historical::HistoricalBarUpdate,
    crate::accounts::AccountUpdate,
    crate::accounts::AccountUpdateMulti,
    crate::accounts::AccountSummaryResult,
    crate::accounts::PositionUpdate,
    crate::accounts::PositionUpdateMulti,
    crate::accounts::PnL,
    crate::accounts::PnLSingle,
    Vec<crate::scanner::ScannerData>,
    crate::news::NewsBulletin,
    crate::display_groups::DisplayGroupUpdate,
);

/// Where the loop is. Shared by the sync iterator and the async stream.
enum State<S> {
    /// Opening a subscription; `gap` is set when it replaces one the reconnect killed.
    Opening {
        gap: bool,
    },
    Reading(S),
    Done,
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
    use super::*;
    use crate::client::sync::Client;
    use crate::subscriptions::sync::Subscription;

    /// Blocking iterator returned by
    /// [`Client::resubscribing`](crate::client::blocking::Client::resubscribing).
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub struct Resubscribing<'a, T: Resubscribable, F> {
        client: &'a Client,
        open: F,
        state: State<Subscription<T>>,
    }

    impl<'a, T: Resubscribable, F> Resubscribing<'a, T, F> {
        pub(crate) fn new(client: &'a Client, open: F) -> Self {
            Self {
                client,
                open,
                state: State::Opening { gap: false },
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
            loop {
                // Done unless an arm puts a live state back.
                match std::mem::replace(&mut self.state, State::Done) {
                    State::Done => return None,
                    State::Opening { gap } => match (self.open)().into() {
                        Opened::Subscribed(subscription) => {
                            self.state = State::Reading(subscription);
                            if gap {
                                return Some(Ok(gap_notice()));
                            }
                        }
                        Opened::Lost => {
                            if let Err(e) = self.client.wait_connected() {
                                return Some(Err(e));
                            }
                            self.state = State::Opening { gap };
                        }
                        Opened::Failed(e) => return Some(Err(e)),
                    },
                    State::Reading(subscription) => match subscription.next() {
                        Some(Err(e)) if e.is_connection_lost() => {
                            // Its request died with the old session; dropping it sends no cancel.
                            drop(subscription);
                            if let Err(e) = self.client.wait_connected() {
                                return Some(Err(e));
                            }
                            self.state = State::Opening { gap: true };
                        }
                        Some(Ok(item)) => {
                            self.state = State::Reading(subscription);
                            return Some(Ok(item));
                        }
                        Some(Err(e)) => return Some(Err(e)),
                        None => return None,
                    },
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
        futures::stream::unfold((State::Opening { gap: false }, open), move |(mut state, mut open)| async move {
            loop {
                match state {
                    State::Done => return None,
                    State::Opening { gap } => match open().await.into() {
                        Opened::Subscribed(subscription) => {
                            state = State::Reading(subscription);
                            if gap {
                                return Some((Ok(gap_notice()), (state, open)));
                            }
                        }
                        Opened::Lost => {
                            if let Err(e) = client.wait_connected().await {
                                return Some((Err(e), (State::Done, open)));
                            }
                        }
                        Opened::Failed(e) => return Some((Err(e), (State::Done, open))),
                    },
                    State::Reading(mut subscription) => match subscription.next().await {
                        Some(Err(e)) if e.is_connection_lost() => {
                            // Its request died with the old session; dropping it sends no cancel.
                            drop(subscription);
                            if let Err(e) = client.wait_connected().await {
                                return Some((Err(e), (State::Done, open)));
                            }
                            state = State::Opening { gap: true };
                        }
                        Some(Ok(item)) => return Some((Ok(item), (State::Reading(subscription), open))),
                        Some(Err(e)) => return Some((Err(e), (State::Done, open))),
                        None => return None,
                    },
                }
            }
        })
    }
}

#[cfg(test)]
#[path = "resubscribe_tests.rs"]
mod tests;
