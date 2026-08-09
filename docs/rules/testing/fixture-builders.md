---
id: fixture-builders
title: Response fixtures are field-minimal builders in src/testdata/builders
cluster: testing
status: active
triggers:
  - building a response test fixture
  - adding a MessageBusStub test for a new message type
  - a test fails with UnexpectedWireFormat
  - asserting that a decoder rejects text framing
  - a test needs the connection to drop mid-request
symbols: [ResponseProtoEncoder, encode_proto, proto_response, text_response, MessageBusStub, ordered_responses, Error::UnexpectedWireFormat, assert_rejects_text_framing, with_connection_resets]
related: [proto-only-decoding, fixture-migration, exercise-production-code]
precedents: ["#534", "#543", "#731", "#735", "#743", "#747"]
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

Use `text_response(...)` only for message types with no proto decoder. A text-framed
response reaching a proto-only decoder fails the subscription with
`Error::UnexpectedWireFormat` — so a fixture left in the legacy
`response_messages: Vec<String>` form dies on its `expect`/`unwrap` rather than iterating zero
times and asserting nothing. No error variant is skippable; whether a message belongs to a
subscription is decided from `RESPONSE_MESSAGE_IDS` before decoding. See
[proto-only decoding](../wire/proto-only-decoding.md).

Field-minimal scales further than it looks: PR #534's `ContractDataResponse` covers roughly
50 `proto::ContractData` fields with about 15 setters. Document any `Default` that is
load-bearing — `min_size: "1"` exists because validators assert `min_size == 1.0`, and
silently changing it breaks tests far from the builder.

`#[allow(clippy::too_many_arguments)]` on a fixture-construction helper is the canary, not
the fix. PR #543's /simplify pass caught free positional-arg helpers under `<domain>/common/`
and refactored them into named builders in `testdata/builders/market_data.rs`.

A builder needs a **current test consumer**. Matching the encoder file one-to-one "for
completeness" is not a consumer.

## A fixture field no assertion reads is unverified input

It will eventually be wrong, and nothing says so. `test_decode_market_rule_rejects_text_framing`
framed itself as message id 87 (`MarketRule` is 93) and
`test_decode_news_providers_rejects_text_framing` led with the literal `"newsProviders"`, which
parses as no discriminant at all. Both passed for as long as they existed, because the only
assertion was that `require_proto()` rejects text framing and `require_proto()` never reads the
type. Two more turned up the same way in the same PR.

So use `assert_rejects_text_framing(expected, text_frame, decode)`
(`src/common/test_utils.rs`) rather than hand-rolling the `matches!` — it checks the frame's own
leading discriminant against `expected` *before* running the decoder, which a pure
extract-the-common-assertion refactor would have preserved the blind spot of. The one caller
that keeps its own shape is `connection`'s handshake reader, which takes
`&mut ResponseMessage`, so it passes a cloning closure.

## What the stub does and does not simulate

`MessageBusStub` sits below the dispatcher, so a fixture reaches the subscription without
being routed. Two consequences pull in opposite directions:

- **It does classify `Error` frames** (since #735). `routed_items()` runs each fixture through
  `determine_routing`/`classify_error`, so an error frame arrives as
  `RoutedItem::Error`/`Notice` exactly as it would on the wire. Before that it arrived as a
  `RoutedItem::Response`, which no real transport produces — that gap is what let decoders grow
  unreachable `IncomingMessages::Error` arms with passing tests to match. A warning or
  data-advisory code now becomes a `Notice` and is filtered by `iter_data()`/`filter_data`;
  assert on `SubscriptionItem::Notice` if that is the point of the test.
- **It does not route by channel.** The stub has one channel per request, so a fixture reaches
  the subscription regardless of whether `determine_routing` could have addressed it there.
  `debug_assert_request_id_routable` covers that half; see
  [proto-aware accessors](../wire/proto-aware-accessors.md).

**It can drop the connection.** `with_connection_resets(n)` answers the first `n` requests with
a bare `RoutedItem::Error(Error::ConnectionReset)` and no responses — what a dropped socket
looks like — so a retrying request path can be asserted on by resend count. Only `server_time`
uses it today; nothing checks that the other one-shot sites retry, so an API whose retry wiring
matters needs its own test rather than an assumption.

## Note on `server_version`

The version passed to `Client::stubbed` gates *outbound encoder* feature checks, not the
wire format — `stubbed` builds `ConnectionMetadata` directly and never runs
`require_protobuf_support`, because `MessageBusStub` sits below the dispatcher. A pre-213
constant like `SIZE_RULES` in a stub test is correct, not a leftover.

## Precedents

- #534 — field-minimal builders for deeply-nested protos.
- #543 — /simplify caught fixture helpers misplaced under `<domain>/common/`.
- #731 — made a mis-framed fixture fail its test instead of silently skipping.
- #735 — taught the stub to classify `Error` frames like the dispatcher.
- #743 — added `with_connection_resets`; a mutation (retry limit 0) reproduced red before the
  fix, which is the evidence a test claiming to gate something owes.
- #747 — one `assert_rejects_text_framing` replaced four spellings across 23 tests, and added
  the discriminant check none of them had.

See also [docs/testing-patterns.md](../../testing-patterns.md) for choosing between
`MessageBusStub`, `MemoryStream`, and `spawn_handshake_listener`.
