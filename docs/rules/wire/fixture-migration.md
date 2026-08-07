---
id: fixture-migration
title: Sweeping test fixtures when a decoder went proto-only
cluster: wire
status: historical
triggers:
  - deleting a text branch from a domain decoder
  - IBKR ships a newly proto-gated message family
symbols: [text_response, proto_response, encode_pipe, encode_proto, ordered_responses]
related: [fixture-builders, floor-ratchet-splits, proto-only-decoding]
precedents: ["#529", "#531", "#532", "#534", "#543"]
memory: [project_protobuf_only, project_tickefp_unobserved]
---

**Concluded.** The per-family text-cleanup arc finished when the connection floor reached
`PROTOBUF_REST_MESSAGES_3` (213). No text decoders remain in production. This node is kept
for the procedure, which applies again if IBKR ships a new gated family.

After deleting a text branch from a domain decoder, find every `MessageBusStub` fixture still
feeding text-framed responses for that message type — `text_response(builder.encode_pipe())`
in `<domain>/{sync,async}_tests.rs` — and convert each to
`proto_response(IncomingMessages::Foo, builder.encode_proto())`.

Prerequisite: a `ResponseProtoEncoder::encode_proto()` impl on the builder.

## Why the sweep can't be skipped

Silent skip-classification means a missed conversion shows up as a **passing test whose
assertions never run**, not as a failure. Verify with sync, async, and all-features sweeps —
there is no other signal.

## Table-driven fixtures

Rename the test-case struct field `response_messages: Vec<String>` to
`ordered_responses: Vec<ResponseMessage>`. Use `proto_response(...)` for migrated types and
`text_response(...)` for end markers, errors, and cross-domain shared decoders that were
never migrated.

## TickEFP

The `TickTypes::EFP` public API was removed in v3.0 along with its decoder and tests. The
`IncomingMessages::TickEFP = 47` variant stays so the `From<i32>` arm remains exhaustive:
incoming message id 47 maps to a known variant that no decoder claims, and the dispatcher
catch-all skip-classifies it. Revisit only if IBKR ships `TickEFP.proto` plus a
`MIN_SERVER_VER_*TICK_EFP*` gate.

## Precedents

- #529, #531, #532 — per-family text-branch deletions.
- #534 — field-minimal builder for a deeply-nested proto.
- #543 — /simplify caught fixture helpers in the wrong module.

The builder conventions from this arc remain active — see
[fixture builders](../testing/fixture-builders.md).
