---
id: proto-aware-accessors
title: ResponseMessage accessors must be proto-aware
cluster: wire
status: active
triggers:
  - adding a &self accessor on ResponseMessage
  - adding a public API on a proto inbound message type
  - subscription routing silently returns no data
  - a new IncomingMessages variant correlates to a request
symbols: [ResponseMessage, peek_int, request_id, order_id, execution_id, routes_by_request_id, text_request_id_field]
related: [proto-only-decoding]
precedents: ["#519", "#647"]
memory: [feedback_request_id_index_registration, feedback_sync_protobuf_routing, project_protobuf_only]
---

Any `&self` accessor on `ResponseMessage` that reads by text-field index — `request_id`,
`order_id`, `execution_id`, `peek_int`, and any future sibling — needs a `raw_bytes`-first
branch. Every production inbound message arrives proto-framed: `fields` holds only the
message id and the payload lives in `raw_bytes`.

Don't decode the whole proto struct to read one field. Define a minimal `prost::Message`
envelope and let prost length-skip the rest.

**Adding a public API on a proto inbound message type also requires an entry in
`text_request_id_field` (`src/messages.rs`).**

## Why

`ResponseMessage::request_id()` short-circuits on a missing `text_request_id_field` entry
*before* it reaches the protobuf-envelope branch, so an unregistered message type silently
never routes — the subscription just receives nothing.

That table is a deliberate allow-list, not a sentinel. It prevents misrouting messages where
`int @ tag 1` means something other than a request id (`MarketRule.market_rule_id`,
`OrderBound.perm_id`). `routes_by_request_id()` is a thin wrapper over it;
`text_request_id_field` is the single source of truth and carries the text-frame field index
(1 vs 2) — populate the right bucket. No production message reaches the text path now, but
tests still construct text-framed fixtures.

**`MessageBusStub` tests structurally bypass the dispatcher and pass with the registration
missing.** Only a live-gateway smoke test surfaces the gap. PR #647 shipped exactly this bug,
then refactored the table.

On the minimal-envelope point: the dispatcher calls these accessors three or four times per
inbound message. A full decode of `OpenOrder` or `ExecutionDetails` costs roughly twenty
String allocations each; an envelope holding just `id @ tag 1` is essentially free.

Cursor primitives return `Result<T, Error>`. Never reintroduce a panicking variant — a
panicking accessor was the root of PR #519's bug class.

## Precedents

- #519 — a panicking accessor on a proto-framed message.
- #647 — shipped a missing routing registration, then split the table into
  `routes_by_request_id` + `text_request_id_field`.
