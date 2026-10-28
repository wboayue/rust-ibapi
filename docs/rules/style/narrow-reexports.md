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
precedents: ["#581", "#1080"]
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

`pub(crate) mod decoders;` would hand every `pub(crate)` item inside it, including ones added
later, to the whole crate. Domain `decoders` / `encoders` modules are `pub(super)`; only `proto/`'s
are crate-wide. Check with `grep -rn '^pub(crate) mod \(decoders\|encoders\)' src`.

## Why

Widening is the cheapest fix for a visibility error and the one that compounds: the next item
added inherits the wider reach with no diff that looks like an API change. Adding a name to a
`pub(crate) use` list is a visible line in review.

Cross-domain reads are the usual trigger. `connection/common.rs` needs five order decoders and one
account decoder to classify handshake-time messages; each domain re-exports exactly those.

Same instinct as [restrict after callers](../workflow/restrict-after-callers.md): narrow surfaces
are cheap to keep and expensive to reclaim. Narrowing an already-wide item surfaces
`private_interfaces` and `dead_code` warnings from places you did not expect.

## Precedents

- #1080 — `orders::common::decoders` had drifted back to `pub(crate)` (#1031); seven other
  domains' `decoders`/`encoders` were crate-wide with no outside consumer. Restored `pub(super)`,
  with named re-exports for the few outside users (realtime, accounts, orders).
- #581 — the other direction's cost: narrowing `ResponseMessage` from `pub` to `pub(crate)` was
  planned as transparent and surfaced ten warnings across `Error` variants, trait impls, and
  now-uncalled methods.
