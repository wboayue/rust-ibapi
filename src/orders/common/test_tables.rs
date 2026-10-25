//! Table-driven test data for orders module tests

use crate::orders::{Order, WhatIfType};
use crate::server_versions;

/// An order flag gated above the connection floor, and the server version it needs.
pub(crate) struct ParamGate {
    pub name: &'static str,
    pub version: i32,
    pub set: fn(&mut Order),
}

/// Mirrors the `ADDITIONAL_ORDER_PARAMS_*` checks in C# `ValidateOrderParameters`.
pub(crate) const ADDITIONAL_PARAM_GATES: &[ParamGate] = &[
    ParamGate {
        name: "deactivate",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        set: |order| order.deactivate = true,
    },
    ParamGate {
        name: "post_only",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        set: |order| order.post_only = true,
    },
    ParamGate {
        name: "allow_pre_open",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        set: |order| order.allow_pre_open = true,
    },
    ParamGate {
        name: "ignore_open_auction",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_1,
        set: |order| order.ignore_open_auction = true,
    },
    ParamGate {
        name: "route_marketable_to_bbo",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_2,
        set: |order| order.route_marketable_to_bbo = true,
    },
    ParamGate {
        name: "seek_price_improvement",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_2,
        set: |order| order.seek_price_improvement = Some(false),
    },
    ParamGate {
        name: "what_if_type",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_2,
        set: |order| order.what_if_type = Some(WhatIfType::FullOrderCheck),
    },
];
