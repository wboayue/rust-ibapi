//! Asynchronous implementation of display groups functionality

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use crate::client::ClientRequestBuilders;
use crate::subscriptions::Subscription;
use crate::transport::AsyncMessageBus;
use crate::{Client, Error};

use super::common::stream_decoders::DisplayGroupUpdate;
use super::encoders;

/// A subscription to display group events with the ability to update the displayed contract.
///
/// Created by [`Client::subscribe_to_group_events`](crate::Client::subscribe_to_group_events).
/// Derefs to `Subscription<DisplayGroupUpdate>` for `next()`, `cancel()`, etc.
///
/// The canonical pattern-match form works through `Deref` with no extra ceremony:
///
/// ```ignore
/// while let Some(item) = subscription.next().await { /* match on SubscriptionItem */ }
/// ```
///
/// # `filter_data` and the reborrow gotcha
///
/// `Subscription::filter_data` (from `SubscriptionItemStreamExt`) takes `self`,
/// and method resolution through `DerefMut` is not allowed to *move* the
/// dereferenced value. So `subscription.filter_data()` on a
/// `DisplayGroupSubscription` fails with `cannot move out of dereference`.
///
/// Reborrow first:
///
/// ```ignore
/// let inner = &mut *subscription;            // `&mut Subscription<_>`
/// while let Some(item) = inner.filter_data().next().await { /* ... */ }
/// ```
///
/// This is a Rust language quirk, not a subscription-shape issue. The same
/// reborrow applies to any Deref-wrapping subscription type.
#[must_use = "DisplayGroupSubscription must be polled (deref to Subscription, then .next().await) to receive updates; dropping it releases the subscription"]
pub struct DisplayGroupSubscription {
    inner: Subscription<DisplayGroupUpdate>,
    message_bus: Arc<dyn AsyncMessageBus>,
}

impl DisplayGroupSubscription {
    /// Updates the contract displayed in the TWS display group.
    ///
    /// # Arguments
    /// * `contract_info` - Contract to display:
    ///   - `"contractID@exchange"` for individual contracts (e.g., "265598@SMART")
    ///   - `"none"` for empty selection
    ///   - `"combo"` for combination contracts
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:7497", 100).await.expect("connection failed");
    ///     let subscription = client.subscribe_to_group_events(1).await.expect("subscription failed");
    ///
    ///     subscription.update("265598@SMART").await.expect("update failed");
    /// }
    /// ```
    pub async fn update(&self, contract_info: &str) -> Result<(), Error> {
        let request_id = self.inner.request_id().expect("subscription has no request ID");
        let request = encoders::encode_update_display_group(request_id, contract_info)?;
        self.message_bus.send_message(request).await
    }
}

impl Deref for DisplayGroupSubscription {
    type Target = Subscription<DisplayGroupUpdate>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for DisplayGroupSubscription {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Client {
    /// Subscribes to display group events for the specified group.
    ///
    /// Display Groups are a TWS-only feature (not available in IB Gateway).
    /// They allow organizing contracts into color-coded groups in the TWS UI.
    /// When subscribed, you receive updates whenever the user changes the contract
    /// displayed in that group within TWS.
    ///
    /// # Arguments
    /// * `group_id` - The ID of the group to subscribe to (1-9)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:7497", 100).await.expect("connection failed");
    ///
    ///     let mut subscription = client.subscribe_to_group_events(1).await.expect("subscription failed");
    ///
    ///     // Update the displayed contract
    ///     subscription.update("265598@SMART").await.expect("update failed");
    ///
    ///     // Consume the subscription so display-group events surface.
    ///     while let Some(event) = subscription.next().await {
    ///         println!("group event: {event:?}");
    ///     }
    /// }
    /// ```
    pub async fn subscribe_to_group_events(&self, group_id: i32) -> Result<DisplayGroupSubscription, Error> {
        let builder = self.request();
        let request = encoders::encode_subscribe_to_group_events(builder.request_id(), group_id)?;
        let inner = builder.send::<DisplayGroupUpdate>(request).await?;
        Ok(DisplayGroupSubscription {
            inner,
            message_bus: self.message_bus.clone(),
        })
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
