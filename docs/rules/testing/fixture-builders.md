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
symbols: [ResponseProtoEncoder, encode_proto, proto_response, text_response, MessageBusStub, with_ordered_responses, Error::UnexpectedWireFormat, assert_rejects_text_framing, with_connection_resets]
related: [proto-only-decoding, fixture-migration, exercise-production-code]
precedents: ["#534", "#543", "#731", "#735", "#743", "#747"]
memory: [feedback_test_fixtures_placement, feedback_testdata_builder_no_new, feedback_no_speculative_test_infra, feedback_mirror_production_patterns]
---

Response fixtures live in `src/testdata/builders/<domain>.rs` as `ResponseProtoEncoder` impls —
never a `test_helpers.rs` under `<domain>/common/`. Feed them to
`MessageBusStub::with_ordered_responses` via `proto_response(...)`
(`common::test_utils::helpers`):

```rust
MessageBusStub::with_ordered_responses(vec![proto_response(
    IncomingMessages::OrderStatus,
    order_status().order_id(1).status(OrderStatusKind::Submitted).encode_proto(),
)])
```

Builders are **field-minimal** — work back from what the test validators assert, not forward
from the proto definition. Entry points are free functions (`order_status()`), not
`Builder::new()`. Every builder needs a current test consumer; mirroring the encoder file
"for completeness" is not one.

## Why

Use `text_response(...)` only for message types with no proto decoder. A text-framed response
reaching a proto-only decoder fails with `Error::UnexpectedWireFormat`, so a text fixture for a
proto type fails its test instead of silently iterating zero times. Subscription membership is
decided from `RESPONSE_MESSAGE_IDS` before decoding. See
[proto-only decoding](../wire/proto-only-decoding.md).

Field-minimal scales: #534's `ContractDataResponse` covers a ~50-field proto with about 15
setters. Document any load-bearing `Default` — `min_size: "1"` exists because validators
assert `min_size == 1.0`.

`#[allow(clippy::too_many_arguments)]` on a fixture helper is the canary, not the fix: convert
positional helpers into named builders (#543).

## A fixture field no assertion reads is unverified input

It will eventually be wrong, and nothing says so. #738 found four text-framing fixtures framed
as the wrong message (`87` for `MarketRule`, which is 93; a literal `"newsProviders"` that
parses as no discriminant). They passed because `require_proto()` rejects text framing without
reading the type.

So use `assert_rejects_text_framing(expected, text_frame, decode)` (`src/common/test_utils.rs`)
rather than a hand-rolled `matches!` — it checks the frame's leading discriminant against
`expected` before running the decoder. The connection handshake reader takes
`&mut ResponseMessage`, so its caller passes a cloning closure.

## What the stub does and does not simulate

`MessageBusStub` sits below the dispatcher:

- **It classifies `Error` frames** (#735). `routed_items()` runs fixtures through
  `determine_routing` / `classify_error`, so an error frame arrives as `RoutedItem::Error` or
  `Notice` as on the wire. Warning / advisory codes become `Notice`s, filtered by
  `iter_data()` / `filter_data`; assert on `SubscriptionItem::Notice` if that is the point.
- **It does not route by channel.** A fixture reaches the subscription whether or not
  `determine_routing` could address it there; `debug_assert_request_id_routable` covers that
  half — see [proto-aware accessors](../wire/proto-aware-accessors.md).
- **It can drop the connection.** `with_connection_resets(n)` answers the first `n` requests
  with a bare `RoutedItem::Error(Error::ConnectionReset)`, so a retry path is asserted by resend
  count. `server_time` (standing in for the shared one-shot helper, `request_helpers_tests.rs`)
  and `historical_data` assert retry; `wait_for_fill` asserts failure on reset. An API whose
  retry wiring matters beyond the shared helper needs its own test.

## Note on `server_version`

The version passed to `Client::stubbed` gates only *outbound encoder* feature checks.
`stubbed` constructs the `Client` directly and never runs the handshake's
`require_protobuf_support`, so a pre-213 constant like `SIZE_RULES` in a stub test is correct,
not a leftover.

## Precedents

- #534 — field-minimal builders for deeply nested protos.
- #543 — /simplify caught fixture helpers misplaced under `<domain>/common/`.
- #731 — a mis-framed fixture fails its test instead of silently skipping.
- #735 — the stub classifies `Error` frames like the dispatcher.
- #743 — added `with_connection_resets`; a mutation (retry limit 0) went red before the fix,
  the evidence a gating test owes.
- #747 — one `assert_rejects_text_framing` replaced four spellings across ~two dozen tests and
  added the discriminant check.
