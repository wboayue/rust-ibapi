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
