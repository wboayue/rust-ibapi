---
id: narrow-reexports
title: Re-export the names you need instead of widening the module
cluster: style
status: active
triggers:
  - exposing one or two items from an otherwise-private module
  - cross-domain code reaching into another domain's decoders
  - about to change mod foo to pub(crate) mod foo to fix a visibility error
symbols: [pub(crate), pub(super)]
related: [domain-module-layout, restrict-after-callers]
precedents: ["#581"]
memory: [project_domain_module_pattern]
---

When one module needs a handful of items from another, re-export exactly those names at the
parent rather than widening the module declaration:

```rust
// orders/common/mod.rs
pub(super) mod decoders;  // the module stays narrow
pub(crate) use decoders::{
    decode_commission_report, decode_completed_order, decode_execution_data, decode_open_order, decode_order_status,
};
```

`pub(crate) mod decoders;` would instead hand every `pub(crate)` item inside `decoders` — and
every one added to it later — to the whole crate.

## Why

Widening is the cheapest fix for a visibility error and the one that compounds. The next item
added to the module inherits the widened reach silently, so the module's surface grows without
any diff that looks like an API change. A `pub(crate) use` list is the opposite: adding a name
to it is a visible line in review.

Cross-domain reads are the common trigger. `connection/common.rs` needs five order decoders and
one account decoder to classify handshake-time messages; both domains expose exactly those and
keep `decoders` at `pub(super)`. Nothing else in `orders::common::decoders` is reachable from
`connection/`.

This is the same instinct as [restrict after callers](../workflow/restrict-after-callers.md)
read forwards: narrow surfaces are cheap to keep and expensive to reclaim. Narrowing an
already-`pub` item is the expensive direction — it surfaces `private_interfaces` and
`dead_code` warnings from surfaces you did not expect to be involved.

## Precedents

- #581 — the cost of the other direction. Narrowing `ResponseMessage` from `pub` to
  `pub(crate)` was planned as transparent and surfaced ten warnings across `Error` variants,
  trait impls, and now-uncalled methods.
