---
id: domain-module-layout
title: Client methods live in their domain module, in flat sync.rs / async.rs files
cluster: style
status: active
triggers:
  - adding a client method for an existing or new domain
  - creating a new domain module
  - choosing between <domain>/sync.rs and <domain>/sync/mod.rs
  - placing a protobuf decoder or a shared proto converter
symbols: [Client, common, proto]
related: [narrow-reexports, sibling-test-files, dual-feature-types]
precedents: ["#657"]
memory: [project_domain_module_pattern, project_main_flat_helpers_nested_pattern]
---

A domain owns its client surface. Its `impl Client` blocks live in `<domain>/sync.rs` and
`<domain>/async.rs` — not in `client/sync.rs` or `client/async.rs`, which hold only
cross-cutting client mechanics (connect, ids, server version, disconnect).

- Shared sync/async logic goes under `<domain>/common/`.
- Protobuf decoders go in `<domain>/common/decoders.rs`; converters shared across domains go in
  `src/proto/decoders.rs`.
- Public types go in `<domain>/mod.rs`, ungated by feature flags.
- Prefer the flat `<domain>/sync.rs` over `<domain>/sync/mod.rs` — the project minimizes
  `mod.rs` files.

Flat applies to the *main* file, not to everything beneath it. `<domain>/<side>.rs` with
`<domain>/<side>/foo.rs` helpers underneath is canonical Rust and stays. Tests are governed
separately by [sibling test files](../testing/sibling-test-files.md).

## Why

Every domain method is feature-gated twice over — once for `sync`, once for `async`. Keeping
the pair in `<domain>/sync.rs` + `<domain>/async.rs` puts the two implementations of one API
next to each other, so parity drift is visible in a two-file diff rather than spread across
`client/`. It also keeps `client/` from growing into the file that every domain PR touches.

Ten domains follow this today — `accounts`, `config`, `contracts`, `display_groups`,
`market_data/historical`, `market_data/realtime`, `news`, `orders`, `scanner`, `wsh` — which
with `client`'s own mechanics makes eleven `impl Client` sites in the tree.
`docs/architecture.md` and `docs/extending-api.md` carry the directory diagram and a worked
example of adding a module.

## Precedents

- #657 — the sweep that established the flat layout and drew the line at helper modules: main
  file flat, helpers may nest.
- #729 — closed the last drift: `Client::order` and `Client::market_data` moved out of
  `client/` into `orders/{sync,async}.rs` and `market_data/realtime/{sync,async}.rs`, next to
  the sibling entry points (`realtime_bars`, `tick_by_tick`, `market_depth`) they had diverged
  from. No exceptions remain — every `impl Client` block outside `client/` is a domain's.
