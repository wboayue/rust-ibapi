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
