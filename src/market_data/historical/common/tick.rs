//! Shared classification of routed items into tick-subscription actions.
//!
//! Both the sync and async [`TickSubscription`](crate::market_data::historical)
//! pull `RoutedItem`s from the transport and turn each into a [`TickAction`].
//! Keeping the decode + classification pure — no buffer, no transport, no
//! interior mutability — makes the historically bug-prone notice/error handling
//! unit-testable on its own and lets the sync and async drivers share one
//! source of truth.

use log::debug;

use crate::market_data::historical::TickDecoder;
use crate::messages::Notice;
use crate::subscriptions::common::RoutedItem;
use crate::Error;

/// The decoded outcome of a single routed item for a tick subscription.
pub(crate) enum TickAction<T> {
    /// A decoded batch of ticks plus the decoder's end-of-stream flag.
    Batch(Vec<T>, bool),
    /// A message that isn't this tick type — skip it and pull the next item.
    Skip,
    /// A non-fatal IB notice bound to this subscription; the stream stays open.
    Notice(Notice),
    /// The stream ended cleanly.
    EndOfStream,
    /// A terminal error; the stream is over.
    Error(Error),
}

/// Classify a routed item into a [`TickAction`].
///
/// Pure: decodes the payload but touches no buffer or transport state. A decode
/// failure becomes [`TickAction::Error`] rather than panicking, so callers
/// surface it through the subscription's `Err` arm.
pub(crate) fn classify<T: TickDecoder<T>>(item: RoutedItem) -> TickAction<T> {
    match item {
        RoutedItem::Response(message) if message.message_type() == T::MESSAGE_TYPE => match T::decode(&message) {
            Ok((ticks, done)) => TickAction::Batch(ticks, done),
            Err(e) => TickAction::Error(e),
        },
        RoutedItem::Response(message) => {
            debug!("unexpected message on historical-ticks channel: {message:?}");
            TickAction::Skip
        }
        RoutedItem::Notice(notice) => TickAction::Notice(notice),
        RoutedItem::Error(Error::EndOfStream) => TickAction::EndOfStream,
        RoutedItem::Error(e) => TickAction::Error(e),
    }
}

#[cfg(test)]
#[path = "tick_tests.rs"]
mod tests;
