//! Synchronous implementation of display groups functionality

use crate::client::blocking::{ClientRequestBuilders, Subscription};
use crate::client::sync::Client;
use crate::Error;

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
pub fn subscribe_to_group_events(client: &Client, group_id: i32) -> Result<Subscription<DisplayGroupUpdate>, Error> {
    let builder = client.request();
    let request = encoders::encode_subscribe_to_group_events(builder.request_id(), group_id)?;
    builder.send(request)
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
pub fn update_display_group(client: &Client, request_id: i32, contract_info: &str) -> Result<(), Error> {
    let request = encoders::encode_update_display_group(request_id, contract_info)?;
    client.send_message(request)
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
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus.clone(), 176);

        update_display_group(&client, 9000, "265598@SMART").expect("update failed");

        let requests = message_bus.request_messages.read().unwrap();
        assert_eq!(requests.len(), 1);

        let req = &requests[0];
        assert_eq!(req[0], "69"); // UpdateDisplayGroup
        assert_eq!(req[1], "1"); // Version
        assert_eq!(req[2], "9000"); // Request ID
        assert_eq!(req[3], "265598@SMART"); // Contract info
    }
}
