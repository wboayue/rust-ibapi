//! Subscription for globally routed IB notices (`request_id == -1`).
//!
//! Per-subscription notices already arrive via [`Subscription::next`] as
//! [`SubscriptionItem::Notice`](super::SubscriptionItem::Notice). This module
//! handles the *unrouted* notices — connectivity codes (1100/1101/1102),
//! farm-status (2104/2105/2106/2107/2108), and any error/warning that lacks a
//! request owner. Each call to `Client::notice_stream` returns a fresh,
//! independent notice stream; late subscribers do not see prior notices.

#[cfg(feature = "sync")]
pub mod sync_impl {
    //! Sync `NoticeStream` backed by a crossbeam channel.

    use std::time::Duration;

    use crossbeam::channel::Receiver;

    use crate::messages::Notice;

    /// A handle for receiving globally routed notices on the sync transport.
    ///
    /// Each `NoticeStream` owns one slot in the dispatcher's broadcaster; dropping
    /// the stream releases that slot at the next broadcast (lazy prune).
    pub struct NoticeStream {
        receiver: Receiver<Notice>,
    }

    impl NoticeStream {
        pub(crate) fn new(receiver: Receiver<Notice>) -> Self {
            Self { receiver }
        }

        /// Block until the next notice arrives, returning `None` when the bus shuts down.
        pub fn next(&self) -> Option<Notice> {
            self.receiver.recv().ok()
        }

        /// Return the next notice if one is queued right now, else `None`.
        pub fn try_next(&self) -> Option<Notice> {
            self.receiver.try_recv().ok()
        }

        /// Wait up to `timeout` for the next notice.
        pub fn next_timeout(&self, timeout: Duration) -> Option<Notice> {
            self.receiver.recv_timeout(timeout).ok()
        }

        /// Blocking iterator over notices. Ends when the bus shuts down.
        pub fn iter(&self) -> NoticeStreamIter<'_> {
            NoticeStreamIter { stream: self }
        }
    }

    /// Blocking iterator yielded by [`NoticeStream::iter`].
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub struct NoticeStreamIter<'a> {
        stream: &'a NoticeStream,
    }

    impl Iterator for NoticeStreamIter<'_> {
        type Item = Notice;

        fn next(&mut self) -> Option<Self::Item> {
            self.stream.next()
        }
    }
}

#[cfg(feature = "async")]
pub mod async_impl {
    //! Async `NoticeStream` backed by a `tokio::sync::broadcast` receiver.

    use futures::stream::{unfold, Stream};
    use log::debug;
    use tokio::sync::broadcast::{self, error::RecvError};

    use crate::messages::Notice;

    /// A handle for receiving globally routed notices on the async transport.
    ///
    /// If the channel lags (broadcaster wraps around because a subscriber didn't
    /// keep up), the missed items are skipped with a debug log and `next` resumes
    /// from the most recent notice.
    pub struct NoticeStream {
        receiver: broadcast::Receiver<Notice>,
    }

    impl NoticeStream {
        pub(crate) fn new(receiver: broadcast::Receiver<Notice>) -> Self {
            Self { receiver }
        }

        /// Wait for the next notice. Returns `None` when the bus shuts down.
        pub async fn next(&mut self) -> Option<Notice> {
            loop {
                match self.receiver.recv().await {
                    Ok(notice) => return Some(notice),
                    Err(RecvError::Closed) => return None,
                    Err(RecvError::Lagged(skipped)) => {
                        debug!("NoticeStream lagged, skipped {skipped} notices");
                        continue;
                    }
                }
            }
        }

        /// `Stream` adapter for combinator-style consumption (`.take`, `.filter`, ...).
        pub fn stream(&mut self) -> impl Stream<Item = Notice> + Unpin + '_ {
            Box::pin(unfold(self, |s| async move { s.next().await.map(|n| (n, s)) }))
        }
    }
}

#[cfg(test)]
#[path = "notice_stream_tests.rs"]
mod tests;
