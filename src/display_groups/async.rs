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
pub async fn subscribe_to_group_events(client: &Client, group_id: i32) -> Result<DisplayGroupSubscription, Error> {
    let builder = client.request();
    let request = encoders::encode_subscribe_to_group_events(builder.request_id(), group_id)?;
    let inner = builder.send::<DisplayGroupUpdate>(request).await?;
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

    #[tokio::test]
    async fn test_subscribe_to_group_events() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            // DisplayGroupUpdated (68), version 1, reqId 9000, contractInfo "265598@SMART"
            response_messages: vec!["68\x001\x009000\x00265598@SMART\x00".to_string()],
        });

        let client = Client::stubbed(message_bus.clone(), 176);

        let mut subscription = subscribe_to_group_events(&client, 1).await.expect("failed to subscribe");

        // Verify request was sent
        {
            let requests = message_bus.request_messages.read().unwrap();
            assert_eq!(requests.len(), 1);

            let req = &requests[0];
            assert_eq!(req[0], "68"); // SubscribeToGroupEvents
            assert_eq!(req[1], "1"); // Version
            assert_eq!(req[3], "1"); // Group ID
        }

        // Verify response
        let result = subscription.next().await;
        assert!(result.is_some());
        let update = result.unwrap().unwrap();
        assert_eq!(update.contract_info, "265598@SMART");
    }

    #[tokio::test]
    async fn test_subscribe_to_group_events_empty_group() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["68\x001\x009000\x00".to_string()],
        });

        let client = Client::stubbed(message_bus, 176);

        let mut subscription = subscribe_to_group_events(&client, 2).await.expect("failed to subscribe");

        let result = subscription.next().await;
        assert!(result.is_some());
        let update = result.unwrap().unwrap();
        assert_eq!(update.contract_info, "");
    }

    #[tokio::test]
    async fn test_update_display_group() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            // Need a response so subscription can be created
            response_messages: vec!["68\x001\x009000\x00265598@SMART\x00".to_string()],
        });

        let client = Client::stubbed(message_bus.clone(), 176);

        let subscription = subscribe_to_group_events(&client, 1).await.expect("failed to subscribe");
        subscription.update("265598@SMART").await.expect("update failed");

        let requests = message_bus.request_messages.read().unwrap();
        // First request is subscribe, second is update
        assert_eq!(requests.len(), 2);

        let req = &requests[1];
        assert_eq!(req[0], "69"); // UpdateDisplayGroup
        assert_eq!(req[1], "1"); // Version
        assert_eq!(req[3], "265598@SMART"); // Contract info
    }

    #[tokio::test]
    async fn test_subscribe_to_group_events_skips_wrong_message_type() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            // First message is DisplayGroupList (67) - wrong type, should be skipped
            // Second message is DisplayGroupUpdated (68) - correct type
            response_messages: vec![
                "67\x001\x009000\x00wrong message\x00".to_string(),
                "68\x001\x009000\x00correct message\x00".to_string(),
            ],
        });

        let client = Client::stubbed(message_bus, 176);

        let mut subscription = subscribe_to_group_events(&client, 1).await.expect("failed to subscribe");

        // Should skip the wrong message type and return the correct one
        let result = subscription.next().await;
        assert!(result.is_some());
        let update = result.unwrap().unwrap();
        assert_eq!(update.contract_info, "correct message");
    }
}
