use crate::builder::ValidationError;
use crate::client::id_generator::ClientIdManager;
use crate::orders::OrderId;
use crate::orders::{Order, OrderCancel};
use crate::protocol::{check_version, ProtocolFeature};
use crate::{server_versions, Error};

/// Checks `order_id` and `order` for sending, then reserves every id the order uses: ids the
/// client allocates afterwards (`next_order_id`, the order builder's `submit`,
/// `exercise_options`) start past them. Nothing is reserved when a check fails. The one entry
/// point for an order id from the caller; `place_order` and `submit_order` (sync and async) call
/// it.
pub(crate) fn verify_and_reserve_order_ids(ids: &ClientIdManager, server_version: i32, order_id: OrderId, order: &Order) -> Result<OrderId, Error> {
    let order_id = verify_order_ids(order_id, order)?;
    verify_order(server_version, order)?;
    verify_order_ext_operator(order)?;
    ids.raise_order_id(order_id_after(order_id, order));
    Ok(order_id)
}

/// The order ids `order` carries besides its own: the preset attached-order ids, which TWS
/// creates orders under, and `parent_id` when set, which names an existing order. Reserving the
/// parent is usually a no-op, since it was reserved when it was sent; when the caller chose it
/// outside the client, reserving it keeps a later client-allocated id from repeating it. Checking
/// and reserving both read this list, so a new id field goes here once.
fn carried_order_ids(order: &Order) -> impl Iterator<Item = i32> {
    [
        order.preset_stop_loss_order_id,
        order.preset_profit_taker_order_id,
        Some(order.parent_id).filter(|&id| id != 0),
    ]
    .into_iter()
    .flatten()
}

/// `order_id` as an [`OrderId`], after checking it and every id in [`carried_order_ids`] against
/// the request-id range ([`OrderId::checked`]). Status and errors for the attached orders route
/// by their ids.
fn verify_order_ids(order_id: OrderId, order: &Order) -> Result<OrderId, Error> {
    let order_id = order_id.checked()?;
    for id in carried_order_ids(order) {
        OrderId::from(id).checked()?;
    }
    Ok(order_id)
}

/// One past the highest of `order_id` and [`carried_order_ids`]: the lowest id the client may
/// still allocate after sending `order`. Call after [`verify_order_ids`], which keeps each id
/// below the request range.
fn order_id_after(order_id: OrderId, order: &Order) -> OrderId {
    OrderId::from(carried_order_ids(order).fold(order_id.value(), i32::max)).next()
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
fn verify_order(server_version: i32, order: &Order) -> Result<(), Error> {
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

    // Tri-state: `Some(false)` is sent too, so it is gated.
    if order.route_marketable_to_bbo.is_some() {
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

/// Rejects `order` when its `ext_operator` is one TWS rejects ([`verify_ext_operator`]).
fn verify_order_ext_operator(order: &Order) -> Result<(), Error> {
    verify_ext_operator("Order::ext_operator", &order.ext_operator)
}

/// Rejects `order_cancel` when its `ext_operator` is one TWS rejects ([`verify_ext_operator`]).
pub(crate) fn verify_order_cancel(order_cancel: &OrderCancel) -> Result<(), Error> {
    verify_ext_operator("OrderCancel::ext_operator", &order_cancel.ext_operator)
}

/// Longest `ext_operator` the client accepts (TWS also takes 65+; see [`verify_ext_operator`]).
const EXT_OPERATOR_MAX_LEN: usize = 16;

/// Rejects an operator ID outside the set TWS accepts: 1–16 ASCII letters or
/// digits, with [`ValidationError::InvalidField`]. Empty passes, since it is not sent.
///
/// On an order TWS rejects anything else with 201; on a cancel it never replies and
/// the order sticks in `PendingCancel`. Probed on paper, server 226 (2026-11-14):
/// see `docs/wire-facts/orders.md#ext_operator-and-manual_order_indicator-on-orders-and-cancels`.
/// TWS also accepts 65+ letters and digits, but this function rejects those too.
fn verify_ext_operator(field: &'static str, value: &str) -> Result<(), Error> {
    if value.is_empty() || (value.len() <= EXT_OPERATOR_MAX_LEN && value.bytes().all(|b| b.is_ascii_alphanumeric())) {
        return Ok(());
    }
    Err(ValidationError::InvalidField {
        field,
        value: value.to_owned(),
        reason: format!("TWS accepts 1-{EXT_OPERATOR_MAX_LEN} ASCII letters or digits"),
    }
    .into())
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
