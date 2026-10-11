//! Table-driven test data for orders module tests

use crate::orders::Order;
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
        name: "route_marketable_to_bbo",
        version: server_versions::ADDITIONAL_ORDER_PARAMS_2,
        set: |order| order.route_marketable_to_bbo = true,
    },
];
