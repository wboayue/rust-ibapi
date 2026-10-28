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
  - splitting a builder's sync and async impl blocks
symbols: [Client, common, proto]
related: [narrow-reexports, sibling-test-files, dual-feature-types]
precedents: ["#657", "#729", "#1078", "#1080"]
memory: [project_domain_module_pattern, project_main_flat_helpers_nested_pattern]
---

A domain owns its client surface. Its `impl Client` blocks live in `<domain>/sync.rs` and
`<domain>/async.rs`, not in `client/sync.rs` / `client/async.rs`, which hold only cross-cutting
mechanics (connect, ids, server version, disconnect).

- Shared sync/async logic goes under `<domain>/common/`.
- Protobuf decoders and encoders go in `<domain>/common/{decoders,encoders}.rs`, including
  sub-message converters a domain's frame codecs share (`decode_order`, `encode_order`).
  Converters used by more than one domain go in `src/proto/{decoders,encoders}.rs`.
- Public types go in `<domain>/mod.rs`, ungated by feature flags.
- Prefer flat `<domain>/sync.rs` over `<domain>/sync/mod.rs`.
- Builder per-feature code: a builder generic over the client (`FooBuilder<'a, C>`) keeps its
  `#[cfg(feature)] impl` blocks in its own file. A builder with distinct per-feature types
  (`ClientBuilder`, `NoticeStream`) wraps each side in inline `sync_impl` / `async_impl` modules.
  A builder with its own directory (`orders/builder`, `config/builder`) may use sibling
  `sync_impl.rs` / `async_impl.rs` files.
- Other inline per-feature submodules (`common/request_helpers.rs`, `common/retry.rs`) are also
  named `sync_impl` / `async_impl`.

Flat applies to the main file only: `<domain>/<side>.rs` with helpers in `<domain>/<side>/foo.rs`
is fine. Tests follow [sibling test files](../testing/sibling-test-files.md).
`docs/architecture.md` and `docs/extending-api.md` have the directory diagram and a worked example.

## Why

Every domain method exists twice, once per feature. Keeping the pair in `<domain>/sync.rs` +
`<domain>/async.rs` puts both implementations of one API side by side, so parity drift shows in a
two-file diff, and keeps `client/` from becoming the file every domain PR touches.

## Precedents

- #657 — established the flat layout; main file flat, helpers may nest.
- #729 — moved `Client::order` and `Client::market_data` out of `client/` into
  `orders/{sync,async}.rs` and `market_data/realtime/{sync,async}.rs`, next to the sibling entry
  points they had diverged from.
- #1080 — moved `Client::submit_oca_orders` from `orders/builder/{sync,async}_impl.rs` to
  `orders/{sync,async}.rs`: a client method in a helper file is drift even inside the right domain.
- #1078 — moved single-domain converters out of `src/proto/` (orders sub-message codecs in #1085,
  then `decode_contract_and_details`, `encode_subscription_params`). Helpers with a second user
  stayed: `decode_soft_dollar_tier` (accounts too), `encode_contract_with_order` (wrapped by the
  shared `encode_contract`).
