# PR #825 review follow-ups

Review of [#825](https://github.com/wboayue/rust-ibapi/pull/825) landed eight findings.
§1-3, §6, §7 shipped in #829; §4 in #827; §5 in #832/#833; §8 was no action.

## Open

- `auction_limit(action, quantity, price)` constructs exactly what
  `limit_order(action, quantity, price)` does since #832 removed its strategy parameter
  (`src/orders/common/order_builder/mod.rs`). It survives only as a documented BOX-routing
  entry point. Third occurrence of a duplicate free constructor trips the `/simplify` rule
  of three — fold it into `limit_order` then.
