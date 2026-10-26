//! Synchronous implementation of display groups functionality

use std::ops::Deref;
use std::sync::Arc;

use crate::client::blocking::{ClientRequestBuilders, Subscription};
use crate::client::sync::Client;
use crate::common::request_helpers::{self, expect_proto};
use crate::transport::MessageBus;
use crate::Error;

use super::common::decoders;
use super::common::stream_decoders::DisplayGroupUpdate;
use super::encoders;

/// A subscription to display group events with the ability to update the displayed contract.
///
/// Created by [`Client::subscribe_to_group_events`](crate::client::blocking::Client::subscribe_to_group_events).
/// Derefs to `Subscription<DisplayGroupUpdate>` for `next()`, `cancel()`, `iter()`, etc.
#[must_use = "DisplayGroupSubscription must be polled (deref to Subscription, then .next() or .iter()) to receive updates; dropping it releases the subscription"]
pub struct DisplayGroupSubscription {
    inner: Subscription<DisplayGroupUpdate>,
    request_id: i32,
    message_bus: Arc<dyn MessageBus>,
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
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:7497", 100).expect("connection failed");
    /// let subscription = client.subscribe_to_group_events(1).expect("subscription failed");
    ///
    /// subscription.update("265598@SMART").expect("update failed");
    /// ```
    pub fn update(&self, contract_info: &str) -> Result<(), Error> {
        let request = encoders::encode_update_display_group(self.request_id, contract_info)?;
        self.message_bus.send_message(&request)
    }
}

impl Deref for DisplayGroupSubscription {
    type Target = Subscription<DisplayGroupUpdate>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> IntoIterator for &'a DisplayGroupSubscription {
    type Item = Result<crate::subscriptions::SubscriptionItem<DisplayGroupUpdate>, crate::Error>;
    type IntoIter = <&'a Subscription<DisplayGroupUpdate> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.inner).into_iter()
    }
}

impl IntoIterator for DisplayGroupSubscription {
    type Item = Result<crate::subscriptions::SubscriptionItem<DisplayGroupUpdate>, crate::Error>;
    type IntoIter = <Subscription<DisplayGroupUpdate> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl Client {
    /// Returns the ids of the display groups visible in TWS, most used first.
    ///
    /// Display Groups are a TWS-only feature (not available in IB Gateway).
    /// Pass an id to [`subscribe_to_group_events`](Self::subscribe_to_group_events).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// # fn main() -> Result<(), ibapi::Error> {
    /// let client = Client::connect("127.0.0.1:7497", 100)?;
    ///
    /// let groups = client.display_groups()?;
    /// println!("visible display groups: {groups:?}");
    /// # Ok(())
    /// # }
    /// ```
    pub fn display_groups(&self) -> Result<Vec<i32>, Error> {
        request_helpers::blocking::one_shot_by_request_id(
            self,
            encoders::encode_query_display_groups,
            expect_proto(decoders::decode_display_group_list_proto),
        )
    }

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
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:7497", 100).expect("connection failed");
    ///
    /// let subscription = client.subscribe_to_group_events(1).expect("subscription failed");
    ///
    /// // Update the displayed contract
    /// subscription.update("265598@SMART").expect("update failed");
    ///
    /// for event in &subscription {
    ///     println!("group event: {:?}", event);
    /// }
    /// ```
    pub fn subscribe_to_group_events(&self, group_id: i32) -> Result<DisplayGroupSubscription, Error> {
        let builder = self.request();
        let request_id = builder.request_id();
        let request = encoders::encode_subscribe_to_group_events(request_id, group_id)?;
        let inner = builder.send(request)?;
        Ok(DisplayGroupSubscription {
            inner,
            request_id,
            message_bus: self.message_bus.clone(),
        })
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
