---
id: fixture-builders
title: Response fixtures are field-minimal builders in src/testdata/builders
cluster: testing
status: active
triggers:
  - building a response test fixture
  - adding a MessageBusStub test for a new message type
  - a test fails with UnexpectedWireFormat
symbols: [ResponseProtoEncoder, encode_proto, proto_response, text_response, MessageBusStub, ordered_responses, Error::UnexpectedWireFormat]
related: [proto-only-decoding, fixture-migration]
precedents: ["#534", "#543", "#731"]
memory: [feedback_test_fixtures_placement, feedback_testdata_builder_no_new, feedback_no_speculative_test_infra, project_error_response_builder_gap, feedback_mirror_production_patterns]
---

Response fixtures live in `src/testdata/builders/<domain>.rs` as `ResponseProtoEncoder`
impls. Never roll a sibling `test_helpers.rs` under `<domain>/common/` for the same purpose.

Feed them to `MessageBusStub::with_ordered_responses` via `proto_response(...)`:

```rust
MessageBusStub::with_ordered_responses(vec![proto_response(
    IncomingMessages::OrderStatus,
    order_status().order_id(1).status(OrderStatusKind::Submitted).encode_proto(),
)])
```

Builders are **field-minimal**: work backwards from what the test validators actually assert,
not forwards from the proto definition. Entry points are free functions (`order_status()`),
not `Builder::new()`.

## Why

A text-framed response reaching a proto-only decoder used to be skip-classified, not raised —
so a fixture left in the legacy `response_messages: Vec<String>` form showed up as a **passing
test whose post-`next_data()` assertions never ran**. The test was green and asserted nothing.

**Since #731 that fails loudly.** `require_proto()` returns `Error::UnexpectedWireFormat`,
which `process_decode_result` does not skip, so the subscription yields a terminal error and
the test dies on its `expect`/`unwrap` instead of iterating zero times. Only
`Error::UnexpectedResponse` — wrong message *type* — is still skipped, which is what shared
channels need. See [proto-only decoding](../wire/proto-only-decoding.md).

The rule is unchanged: use `text_response(...)` only for message types with no proto decoder.
What changed is that getting it wrong now costs you a red test rather than a silent one.

Field-minimal scales further than it looks: PR #534's `ContractDataResponse` covers roughly
50 `proto::ContractData` fields with about 15 setters. Document any `Default` that is
load-bearing — `min_size: "1"` exists because validators assert `min_size == 1.0`, and
silently changing it breaks tests far from the builder.

`#[allow(clippy::too_many_arguments)]` on a fixture-construction helper is the canary, not
the fix. PR #543's /simplify pass caught free positional-arg helpers under `<domain>/common/`
and refactored them into named builders in `testdata/builders/market_data.rs`.

A builder needs a **current test consumer**. Matching the encoder file one-to-one "for
completeness" is not a consumer.

## Note on `server_version`

The version passed to `Client::stubbed` gates *outbound encoder* feature checks, not the
wire format — `stubbed` builds `ConnectionMetadata` directly and never runs
`require_protobuf_support`, because `MessageBusStub` sits below the dispatcher. A pre-213
constant like `SIZE_RULES` in a stub test is correct, not a leftover.

## Precedents

- #534 — field-minimal builders for deeply-nested protos.
- #543 — /simplify caught fixture helpers misplaced under `<domain>/common/`.
- #731 — made a mis-framed fixture fail its test instead of silently skipping.

See also [docs/testing-patterns.md](../../testing-patterns.md) for choosing between
`MessageBusStub`, `MemoryStream`, and `spawn_handshake_listener`.
