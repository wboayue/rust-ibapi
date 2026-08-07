---
id: exercise-production-code
title: A test must traverse production code
cluster: testing
status: active
triggers:
  - writing a test that builds a value and asserts on the same value
  - reviewing a new test
  - a test suite is green but a bug reached an example
symbols: [assert_request, RequestEncoder, decode_proto, MessageBusStub]
related: [fixture-builders, coverage-floor, sibling-test-files]
precedents: ["#534", "#543"]
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

## Precedents

- #534 — field-minimal builders driven from what validators actually assert.
- #543 — /simplify caught positional-arg fixture helpers under `<domain>/common/` and moved
  them into named builders.

See [fixture-builders](fixture-builders.md) for where the response fixtures themselves live,
and [docs/testing-patterns.md](../../testing-patterns.md) for choosing among the three fixtures.
