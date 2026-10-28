---
id: exercise-production-code
title: A test must traverse production code
cluster: testing
status: active
triggers:
  - writing a test that builds a value and asserts on the same value
  - reviewing a new test
  - a test suite is green but a bug reached an example
  - a mock client defines a method the real Client already has
  - deleting an unreachable match arm or the tests that covered it
symbols: [assert_request, RequestEncoder, decode_proto, MessageBusStub, Client::stubbed]
related: [fixture-builders, coverage-floor, sibling-test-files]
precedents: ["#534", "#543", "#734", "#735", "#750"]
memory: [feedback_examples_expose_test_gaps, feedback_no_speculative_test_infra, feedback_lightest_test_fixture, feedback_unreachable_regression_guards]
---

Ask of every new test: **what production code does this traverse?** If none, drop it. A
self-loop — builder → encode → decode → assert the builder's own fields — verifies only
pass-through and `prost`.

- **Outgoing requests**: drive the real client API (`Client::stubbed`), then assert on the
  captured bytes with `assert_request(&message_bus, index, &builder)` — it takes both the
  message id and the expected proto body from the builder's `RequestEncoder` impl.
- **Incoming responses**: feed builder bytes through the production decoder (`decode_*_proto`).

Pick the lightest fixture that reaches the seam: `MessageBusStub` (below the dispatcher),
`MemoryStream` (transport), handshake-replay listener (connection setup). See
[docs/testing-patterns.md](../../testing-patterns.md).

## Why

A self-loop is green by construction and stays green through any bug in the code it appears
to cover — and it reads as coverage, so nobody writes the test that would catch the defect.
`assert!(result.is_some())` / `is_ok()` is the weaker form: a `Subscription` yielding `Err`
items satisfies it. Running the matching example surfaces what the test swallowed.

**A mock that hosts a copy of the code under test** is the strongest self-loop. The orders
builder mock client carried its own `submit` / `analyze`, and the copy had drifted: its
`submit_all` numbered a bracket `base_id + i` from one `next_order_id()` call, where production
reserves three ids up front — identical output while ids increment by one, so four bracket
tests never noticed. Drive `Client::stubbed` instead. When the copy dies, the mock and the rest
of its file usually go with it (#750, then #1054 deleted `src/orders/builder/tests.rs`).

## Deleting tests and dead arms

- When a structural change makes a bug class impossible, delete the regression tests at the
  bypassed layer along with the dead code — a test of an unreachable path traverses nothing.
- **Read a dead arm before deleting it.** It marks where someone expected a case to arrive.
  Blocking `matching_symbols` had an unreachable `IncomingMessages::Error` arm, so a routed
  error fell through to `Ok(Vec::new())` — a rejected pattern looked like "no matches".
  `OrderBuilder::analyze` had the same hole.
- **"Covered at a lower layer" is not a reason to delete.** Transport tests prove the
  dispatcher *produces* `RoutedItem::Error`, not that a given public API hands it to the caller;
  ask once per public entry point (see
  [subscription consumer idiom](../parity/subscription-consumer-idiom.md)). If a test looks
  deletable because only `MessageBusStub` could produce its input, make the stub behave like the
  wire instead; see [fixture builders](fixture-builders.md).

## Precedents

- #534 — field-minimal builders driven from what validators actually assert.
- #543 — /simplify moved positional-arg fixture helpers under `<domain>/common/` into named
  builders.
- #734 — deleted decoder tests as "covered elsewhere"; two were a public API's only error-path
  coverage, restored in #735.
- #735 — moved `analyze` off its shadow and taught `MessageBusStub` to classify error frames.
- #750 — deleted the `submit` shadows and their mock client; 12 shadow tests became 14 against
  `Client::stubbed`.
