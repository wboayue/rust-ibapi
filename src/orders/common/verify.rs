use crate::orders::Order;
use crate::orders::OrderId;
use crate::protocol::{check_version, ProtocolFeature};
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

// Gates whose server version is at or below the connection floor
// (`PROTOBUF_REST_MESSAGES_3`, 213) are omitted: they always pass, since the
// handshake rejects older servers.
const HEDGE_MAX_SIZE: ProtocolFeature = ProtocolFeature::new("hedge max size", server_versions::HEDGE_MAX_SIZE);
const DEACTIVATE: ProtocolFeature = ProtocolFeature::new("deactivate", server_versions::ADDITIONAL_ORDER_PARAMS_1);
const POST_ONLY: ProtocolFeature = ProtocolFeature::new("post only", server_versions::ADDITIONAL_ORDER_PARAMS_1);
const ALLOW_PRE_OPEN: ProtocolFeature = ProtocolFeature::new("allow pre-open", server_versions::ADDITIONAL_ORDER_PARAMS_1);
const IGNORE_OPEN_AUCTION: ProtocolFeature = ProtocolFeature::new("ignore open auction", server_versions::ADDITIONAL_ORDER_PARAMS_1);
const ROUTE_MARKETABLE_TO_BBO: ProtocolFeature = ProtocolFeature::new("route marketable to BBO", server_versions::ADDITIONAL_ORDER_PARAMS_2);
const SEEK_PRICE_IMPROVEMENT: ProtocolFeature = ProtocolFeature::new("seek price improvement", server_versions::ADDITIONAL_ORDER_PARAMS_2);
const WHAT_IF_TYPE: ProtocolFeature = ProtocolFeature::new("what-if type", server_versions::ADDITIONAL_ORDER_PARAMS_2);
const ATTACHED_ORDERS: ProtocolFeature = ProtocolFeature::new("attached orders", server_versions::ATTACHED_ORDERS);
const CONDITIONS_INCLUDE_OVERNIGHT: ProtocolFeature =
    ProtocolFeature::new("conditions include overnight", server_versions::CONDITIONS_INCLUDE_OVERNIGHT);

/// Rejects `order` when it uses a field `server_version` does not support.
pub(crate) fn verify_order(server_version: i32, order: &Order) -> Result<(), Error> {
    if order.hedge_max_size.is_some() {
        check_version(server_version, HEDGE_MAX_SIZE)?;
    }

    // Mirrors C# `ValidateOrderParameters`; the encoder sends each flag only when set.
    if order.deactivate {
        check_version(server_version, DEACTIVATE)?;
    }

    if order.post_only {
        check_version(server_version, POST_ONLY)?;
    }

    if order.allow_pre_open {
        check_version(server_version, ALLOW_PRE_OPEN)?;
    }

    if order.ignore_open_auction {
        check_version(server_version, IGNORE_OPEN_AUCTION)?;
    }

    if order.route_marketable_to_bbo {
        check_version(server_version, ROUTE_MARKETABLE_TO_BBO)?;
    }

    if order.seek_price_improvement.is_some() {
        check_version(server_version, SEEK_PRICE_IMPROVEMENT)?;
    }

    if order.what_if_type.is_some() {
        check_version(server_version, WHAT_IF_TYPE)?;
    }

    if order.preset_stop_loss_order_id.is_some() || order.preset_profit_taker_order_id.is_some() {
        check_version(server_version, ATTACHED_ORDERS)?;
    }

    if order.conditions_include_overnight {
        check_version(server_version, CONDITIONS_INCLUDE_OVERNIGHT)?;
    }

    Ok(())
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
