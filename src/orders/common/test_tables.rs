//! Table-driven test data for orders module tests

use crate::orders::{Order, WhatIfType};
use crate::server_versions;

/// An order field gated above the connection floor: the server version it needs and the
/// feature name its `Error::ServerVersion` carries.
pub(crate) struct ParamGate {
    pub name: &'static str,
    pub version: i32,
    pub feature: &'static str,
    pub set: fn(&mut Order),
}

/// Every field `verify_order` gates above the connection floor, mirroring C#
/// `ValidateOrderParameters`. Versions come from `server_versions`, not `verify.rs`, so the
/// table checks the production gates rather than restating them.
pub(crate) const ORDER_PARAM_GATES: &[ParamGate] = &[
    ParamGate {
        name: "hedge_max_size",
        version: server_versions::HEDGE_MAX_SIZE,
        feature: "hedge max size",
        set: |order| order.hedge_max_size = Some(1),
    },
    ParamGate {
        name: "deactivate",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        feature: "deactivate",
        set: |order| order.deactivate = true,
    },
    ParamGate {
        name: "post_only",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        feature: "post only",
        set: |order| order.post_only = true,
    },
    ParamGate {
        name: "allow_pre_open",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        feature: "allow pre-open",
        set: |order| order.allow_pre_open = true,
    },
    ParamGate {
        name: "ignore_open_auction",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        feature: "ignore open auction",
        set: |order| order.ignore_open_auction = true,
    },
    // Gated on presence: an explicit `false` is sent too.
    ParamGate {
        name: "route_marketable_to_bbo",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_2,
        feature: "route marketable to BBO",
        set: |order| order.route_marketable_to_bbo = Some(false),
    },
    ParamGate {
        name: "seek_price_improvement",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_2,
        feature: "seek price improvement",
        set: |order| order.seek_price_improvement = Some(false),
    },
    ParamGate {
        name: "what_if_type",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_2,
        feature: "what-if type",
        set: |order| order.what_if_type = Some(WhatIfType::FullOrderCheck),
    },
    ParamGate {
        name: "preset_stop_loss_order_id",
        version: server_versions::ATTACHED_ORDERS,
        feature: "attached orders",
        set: |order| order.preset_stop_loss_order_id = Some(8),
    },
    ParamGate {
        name: "preset_profit_taker_order_id",
        version: server_versions::ATTACHED_ORDERS,
        feature: "attached orders",
        set: |order| order.preset_profit_taker_order_id = Some(9),
    },
    ParamGate {
        name: "conditions_include_overnight",
        version: server_versions::CONDITIONS_INCLUDE_OVERNIGHT,
        feature: "conditions include overnight",
        set: |order| order.conditions_include_overnight = true,
    },
];
