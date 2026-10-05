use crate::orders::Order;
use crate::orders::OrderId;
use crate::{server_versions, Error};

/// `order_id` as an [`OrderId`], after checking it and every order id `order`
/// carries against the request-id range ([`OrderId::checked`]). TWS creates
/// the preset attached orders under their ids, so their status and errors
/// route by them; `parent_id` (when set) names an existing order.
pub(crate) fn verify_order_ids(order_id: OrderId, order: &Order) -> Result<OrderId, Error> {
    let order_id = order_id.checked()?;
    let carried = [
        order.preset_stop_loss_order_id,
        order.preset_profit_taker_order_id,
        Some(order.parent_id).filter(|&id| id != 0),
    ];
    for id in carried.into_iter().flatten() {
        OrderId::from(id).checked()?;
    }
    Ok(order_id)
}

pub(crate) trait VersionedClient {
    fn check_version(&self, version: i32, message: &str) -> Result<(), Error>;
}

#[cfg(feature = "sync")]
impl VersionedClient for crate::client::sync::Client {
    fn check_version(&self, version: i32, message: &str) -> Result<(), Error> {
        self.check_server_version(version, message)
    }
}

#[cfg(feature = "async")]
impl VersionedClient for crate::client::r#async::Client {
    fn check_version(&self, version: i32, message: &str) -> Result<(), Error> {
        self.check_server_version(version, message)
    }
}

// Gates whose server version is at or below the connection floor
// (`PROTOBUF_REST_MESSAGES_3`) are omitted: they always pass, since the
// handshake rejects older servers.
pub(crate) fn verify_order(client: &impl VersionedClient, order: &Order) -> Result<(), Error> {
    if order.hedge_max_size.is_some() {
        client.check_version(server_versions::HEDGE_MAX_SIZE, "It does not support hedge_max_size parameter")?
    }

    if order.preset_stop_loss_order_id.is_some() || order.preset_profit_taker_order_id.is_some() {
        client.check_version(server_versions::ATTACHED_ORDERS, "It does not support attached orders.")?
    }

    Ok(())
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
