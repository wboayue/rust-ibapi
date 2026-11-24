//! Shared classification of routed items into tick-subscription actions.
//!
//! Both the sync and async [`TickSubscription`](crate::market_data::historical)
//! pull `RoutedItem`s from the transport and turn each into a [`TickAction`].
//! Keeping the decode + classification pure — no buffer, no transport, no
//! interior mutability — makes the historically bug-prone notice/error handling
//! unit-testable on its own and lets the sync and async drivers share one
//! source of truth.

use log::debug;

use crate::market_data::historical::{TickDecoder, WhatToShow};
use crate::messages::{IncomingMessages, Notice};
use crate::subscriptions::common::{is_undeclared, RoutedItem};
use crate::Error;

/// The decoded outcome of a single routed item for a tick subscription.
pub(crate) enum TickAction<T> {
    /// A decoded batch of ticks plus the decoder's end-of-stream flag.
    Batch(Vec<T>, bool),
    /// A message that isn't this tick type — skip it and pull the next item.
    Skip,
    /// A non-fatal IB notice bound to this subscription; the stream stays open.
    Notice(Notice),
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
        RoutedItem::Response(message) if is_undeclared(T::RESPONSE_MESSAGE_IDS, &message) => match tick_kind(message.message_type()) {
            // TWS answers with another kind when the contract has none of the
            // requested kind, and sends nothing after it: skipping it would
            // wait forever. See docs/wire-facts/market-data.md#historical-ticks-of-another-kind.
            Some(received) => TickAction::Error(Error::UnexpectedTickKind {
                requested: T::WHAT_TO_SHOW,
                received,
            }),
            None => {
                debug!("unexpected message on historical-ticks channel: {message:?}");
                TickAction::Skip
            }
        },
        RoutedItem::Response(message) => match T::decode(&message) {
            Ok((ticks, done)) => TickAction::Batch(ticks, done),
            Err(e) => TickAction::Error(e),
        },
        RoutedItem::Notice(notice) => TickAction::Notice(notice),
        RoutedItem::Error(e) => TickAction::Error(e),
    }
}

/// The kind of historical tick a frame carries, or `None` for any other message.
fn tick_kind(message_type: IncomingMessages) -> Option<WhatToShow> {
    match message_type {
        IncomingMessages::HistoricalTickLast => Some(WhatToShow::Trades),
        IncomingMessages::HistoricalTickBidAsk => Some(WhatToShow::BidAsk),
        IncomingMessages::HistoricalTick => Some(WhatToShow::MidPoint),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tick_tests.rs"]
mod tests;
