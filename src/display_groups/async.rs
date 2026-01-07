//! Asynchronous implementation of display groups functionality

use crate::client::ClientRequestBuilders;
use crate::subscriptions::Subscription;
use crate::{Client, Error};

use super::common::stream_decoders::DisplayGroupUpdate;
use super::encoders;

/// Subscribes to display group events for the specified group.
///
/// Display Groups are a TWS-only feature (not available in IB Gateway).
/// When subscribed, you receive updates whenever the user changes the contract
/// displayed in that group within TWS.
///
/// # Arguments
/// * `client` - The connected client
/// * `group_id` - The ID of the group to subscribe to (1-9)
pub async fn subscribe_to_group_events(client: &Client, group_id: i32) -> Result<Subscription<DisplayGroupUpdate>, Error> {
    let builder = client.request();
    let request = encoders::encode_subscribe_to_group_events(builder.request_id(), group_id)?;
    builder.send::<DisplayGroupUpdate>(request).await
}

/// Updates the contract displayed in a TWS display group.
///
/// This function changes the contract shown in the specified display group within TWS.
/// You must first subscribe to the group using [`subscribe_to_group_events`] before
/// calling this function. The update will trigger a `DisplayGroupUpdated` callback
/// on the existing subscription.
///
/// # Arguments
/// * `client` - The connected client
/// * `request_id` - The request ID from the subscription (use `subscription.request_id()`)
/// * `contract_info` - Contract to display:
///   - `"contractID@exchange"` for individual contracts (e.g., "265598@SMART")
///   - `"none"` for empty selection
///   - `"combo"` for combination contracts
pub async fn update_display_group(client: &Client, request_id: i32, contract_info: &str) -> Result<(), Error> {
    let request = encoders::encode_update_display_group(request_id, contract_info)?;
    client.send_message(request).await
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
