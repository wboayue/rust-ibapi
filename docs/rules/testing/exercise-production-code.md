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

Ask of every new test: **what production code does this traverse?** If the answer is "none",
drop it. A self-loop — builder → encode → decode → assert the builder's own fields — verifies
only pass-through and `prost`.

- **Outgoing requests**: drive the real client API, then assert on the captured bytes with
  `assert_request(&message_bus, index, &builder)` — it pulls both the message id and the
  expected proto body from the builder's `RequestEncoder` impl.
- **Incoming responses**: feed builder bytes through the production decoder (`decode_*_proto`).

## Why

A self-loop test is green by construction and stays green through any bug in the code it
appears to cover. Worse, it reads as coverage — the module looks tested, so nobody adds the
test that would have caught the defect.

Assertions of the form `assert!(result.is_some())` or `is_ok()` have the same defect in
weaker form: a `Subscription` yielding `Err` items satisfies them. Running the matching
example is what surfaces those — the example prints what the test swallowed.

Pick the lightest fixture that reaches the seam under test: `MessageBusStub` sits below the
dispatcher, `MemoryStream` covers the transport, and the handshake-replay listener covers
connection setup. Reaching for a heavier one than the seam needs buys nothing and slows the
suite.

The inverse also applies: when a routing or structural change makes a bug class impossible,
delete the regression tests at the now-bypassed layer along with the dead code. A test
guarding an unreachable path traverses nothing either.

**Read a dead arm before deleting it.** It marks where someone expected a case to arrive,
which is where to check whether the real case is handled at all. Blocking `matching_symbols`
carried an unreachable `IncomingMessages::Error` arm next to `if let Some(Ok(mut message))`,
so a routed error fell through to `Ok(Vec::new())` — a rejected pattern was indistinguishable
from "no symbols matched". `OrderBuilder::analyze` had the same hole on both sides.

**"Covered at a lower layer" is not a reason to delete a test.** Transport tests prove the
dispatcher *produces* a `RoutedItem::Error`; they say nothing about whether a given public API
hands it to the caller, and two APIs did not. That question has to be asked once per public
entry point — no shared layer answers it, and
[subscription consumer idiom](../parity/subscription-consumer-idiom.md) is where the shape to
look for is written down. When a test looks deletable because only `MessageBusStub` could
produce its input, the cheaper fix is usually to make the stub behave like the wire; see
[fixture builders](fixture-builders.md).

## A mock that hosts a copy of the code under test

The strongest form of the self-loop is a mock client carrying its own `submit` / `analyze`
that mirrors the production method. `src/orders/builder/{sync,async}_impl/tests.rs` did this,
and the copy had already drifted: its `submit_all` numbered a bracket trio `base_id + i` from a
single `next_order_id()`, where production calls `next_order_id()` three times and reserves the
ids up front. Same output while ids increment by one, which is why four tests named for bracket
submission never noticed. Drive `Client::stubbed` and assert on the captured request proto.

Deleting the shadows left the mock with no callers at all: `src/orders/builder/tests.rs` went
from 292 lines to 123 (`wc -l`). **A mock that exists to host a copy of the code under test
takes the rest of its file with it when the copy dies.**

## Precedents

- #534 — field-minimal builders driven from what validators actually assert.
- #543 — /simplify caught positional-arg fixture helpers under `<domain>/common/` and moved
  them into named builders.
- #734 — deleted a batch of decoder tests as covered elsewhere; two were the only coverage of a
  public API's error path, and were restored in #735.
- #735 — moved `analyze` off its shadow and taught `MessageBusStub` to classify error frames,
  which is what let the restored tests pass unmodified.
- #750 — deleted the `submit` shadows and the mock client with them; 12 shadow tests became 14
  against `Client::stubbed`.

See [fixture-builders](fixture-builders.md) for where the response fixtures themselves live,
and [docs/testing-patterns.md](../../testing-patterns.md) for choosing among the three fixtures.
