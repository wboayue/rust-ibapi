//! Synchronous implementation of display groups functionality

use std::ops::Deref;
use std::sync::Arc;

use crate::client::blocking::{ClientRequestBuilders, Subscription};
use crate::client::sync::Client;
use crate::transport::MessageBus;
use crate::Error;

use super::common::stream_decoders::DisplayGroupUpdate;
use super::encoders;

/// A subscription to display group events with the ability to update the displayed contract.
///
/// Created by [`Client::subscribe_to_group_events`](crate::client::blocking::Client::subscribe_to_group_events).
/// Derefs to `Subscription<DisplayGroupUpdate>` for `next()`, `cancel()`, `iter()`, etc.
pub struct DisplayGroupSubscription {
    inner: Subscription<DisplayGroupUpdate>,
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
    pub fn update(&self, contract_info: &str) -> Result<(), Error> {
        let request_id = self.inner.request_id().expect("subscription has no request ID");
        let request = encoders::encode_update_display_group(request_id, contract_info)?;
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
        let request = encoders::encode_subscribe_to_group_events(builder.request_id(), group_id)?;
        let inner = builder.send(request)?;
        Ok(DisplayGroupSubscription {
            inner,
            message_bus: self.message_bus.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::MessageBusStub;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_update_display_group() {
        use crate::common::test_utils::helpers::assert_proto_msg_id;
        use crate::messages::OutgoingMessages;

        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            // Need a response so subscription can be created
            response_messages: vec!["68\x001\x009000\x00265598@SMART\x00".to_string()],
        });

        let client = Client::stubbed(message_bus.clone(), 176);

        let subscription = client.subscribe_to_group_events(1).expect("failed to subscribe");
        subscription.update("265598@SMART").expect("update failed");

        let requests = message_bus.request_messages.read().unwrap();
        // First request is subscribe, second is update
        assert_eq!(requests.len(), 2);

        assert_proto_msg_id(&requests[0], OutgoingMessages::SubscribeToGroupEvents);
        assert_proto_msg_id(&requests[1], OutgoingMessages::UpdateDisplayGroup);
    }
}
