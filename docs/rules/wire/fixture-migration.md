---
id: fixture-migration
title: Sweeping test fixtures when a decoder went proto-only
cluster: wire
status: historical
triggers:
  - deleting a text branch from a domain decoder
  - IBKR ships a newly proto-gated message family
symbols: [text_response, proto_response, encode_proto, ordered_responses]
related: [fixture-builders, floor-ratchet-splits, proto-only-decoding]
precedents: ["#529", "#531", "#532", "#534", "#543"]
memory: [project_protobuf_only, project_tickefp_unobserved]
---

**Concluded** when the connection floor reached `PROTOBUF_REST_MESSAGES_3` (213); no text
decoders remain in production. Kept for the procedure, should IBKR ship a new gated family.

After deleting a text branch from a domain decoder, find every `MessageBusStub` fixture still
feeding text-framed responses for that type — `text_response(...)` in
`<domain>/{sync,async}_tests.rs` and in test tables' `ordered_responses` — and convert each to
`proto_response(IncomingMessages::Foo, builder.encode_proto())`. Prerequisite: a
`ResponseProtoEncoder::encode_proto()` impl on the builder.

## Why the sweep couldn't be skipped

During the arc, decoders skip-classified unreadable frames, so a missed conversion showed up as a
**passing test whose assertions never ran**; only sync, async and all-features sweeps caught it.
Since #731/#732 a text-framed arrival at a proto decoder raises `Error::UnexpectedWireFormat` and
fails the test (see [proto-only decoding](proto-only-decoding.md)), so a future sweep fails loudly.

## TickEFP

`TickTypes::EFP` was removed in v3.0 with its decoder and tests. `IncomingMessages::TickEFP = 47`
stays so `From<i32>` maps id 47 to a known variant; no decoder claims it, so it lands on a shared
channel nobody subscribes to and the dispatcher reports it as an unroutable frame. Revisit only if
IBKR ships `TickEFP.proto` plus a `MIN_SERVER_VER_*TICK_EFP*` gate.

## Precedents

- #529, #531, #532 — per-family text-branch deletions.
- #534 — field-minimal builder for a deeply nested proto.
- #543 — /simplify caught fixture helpers in the wrong module.

The builder conventions from this arc remain active — see
[fixture builders](../testing/fixture-builders.md).
