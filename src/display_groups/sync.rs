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
    type Item = DisplayGroupUpdate;
    type IntoIter = <&'a Subscription<DisplayGroupUpdate> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.inner).into_iter()
    }
}

impl IntoIterator for DisplayGroupSubscription {
    type Item = DisplayGroupUpdate;
    type IntoIter = <Subscription<DisplayGroupUpdate> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

/// Subscribes to display group events for the specified group.
///
/// Display Groups are a TWS-only feature (not available in IB Gateway).
/// Returns a [`DisplayGroupSubscription`] that receives updates when the user changes
/// the displayed contract in TWS, and supports [`update()`](DisplayGroupSubscription::update)
/// to change the displayed contract programmatically.
///
/// # Arguments
/// * `client` - The connected client
/// * `group_id` - The ID of the group to subscribe to (1-9)
pub fn subscribe_to_group_events(client: &Client, group_id: i32) -> Result<DisplayGroupSubscription, Error> {
    let builder = client.request();
    let request = encoders::encode_subscribe_to_group_events(builder.request_id(), group_id)?;
    let inner = builder.send(request)?;
    Ok(DisplayGroupSubscription {
        inner,
        message_bus: client.message_bus.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::MessageBusStub;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_update_display_group() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            // Need a response so subscription can be created
            response_messages: vec!["68\x001\x009000\x00265598@SMART\x00".to_string()],
        });

        let client = Client::stubbed(message_bus.clone(), 176);

        let subscription = subscribe_to_group_events(&client, 1).expect("failed to subscribe");
        subscription.update("265598@SMART").expect("update failed");

        let requests = message_bus.request_messages.read().unwrap();
        // First request is subscribe, second is update
        assert_eq!(requests.len(), 2);

        let req = &requests[1];
        assert_eq!(req[0], "69"); // UpdateDisplayGroup
        assert_eq!(req[1], "1"); // Version
        assert_eq!(req[3], "265598@SMART"); // Contract info
    }
}
