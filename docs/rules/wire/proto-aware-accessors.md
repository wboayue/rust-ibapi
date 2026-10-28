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
symbols: [ResponseMessage, peek_int, request_id, order_id, client_id, execution_id, routes_by_request_id, text_request_id_field, debug_assert_request_id_routable, first_unroutable_by_request_id]
related: [proto-only-decoding, coverage-floor]
precedents: ["#519", "#647", "#730", "#734"]
memory: [feedback_request_id_index_registration, feedback_sync_protobuf_routing, project_protobuf_only]
---

Any `&self` accessor on `ResponseMessage` (`request_id`, `order_id`, `client_id`,
`execution_id`, and any future sibling) reads `raw_bytes` first. Every production inbound
message arrives proto-framed: `fields` holds only the message id and the payload lives in
`raw_bytes`. Text-field-index reads (`peek_int`) are a test-fixture fallback only.

Don't decode the whole proto struct to read one field. Define a minimal `prost::Message`
envelope (alongside the existing ones in `src/messages.rs`) and let prost length-skip the rest: `ProtoIdEnvelope` (`int32 @ tag 1`) covers most
messages, `ExecutionDetailsMinimal` shows the nested case. A full `OpenOrder` or
`ExecutionDetails` decode costs roughly twenty String allocations, and the dispatcher calls these accessors
on every inbound frame.

**Check the tag per message type.** `request_id` and `order_id` sit at tag 1 wherever they
appear, so one envelope serves them. `contract_id` would not: the `Contract` sub-message is at
tag 2 for `ContractData` / `OpenOrder` / `ExecutionDetails` / `Position`, tag 1 for
`PortfolioValue` / `CompletedOrder`, tag 3 for `PositionMulti`. Such an accessor needs
`match self.message_type()` and one envelope per tag position; read the tags off the generated structs in
`src/proto/protobuf.rs`.

**Adding a public API on a proto inbound message type requires an entry in
`text_request_id_field` (`src/messages.rs`)** if the type correlates to a request.

**A new accessor can trip `-D warnings` before it has a caller.** `ResponseMessage` is
`pub(crate)` (#581), so an uncalled accessor is dead code, per feature too: see `is_shutdown`'s
`#[cfg_attr(not(feature = "sync"), allow(dead_code))]`.

Accessors return `Result`/`Option`; never a panicking variant (#519's bug class).

## Why

`ResponseMessage::request_id()` returns `None` for any type not in the allow-list *before*
reaching the envelope branch, so an unregistered type silently never routes — the subscription
just receives nothing.

`text_request_id_field` is a deliberate allow-list, not a sentinel: it prevents misrouting types
where `int @ tag 1` means something else (`MarketRule.market_rule_id`, `OrderBound.perm_id`).
`routes_by_request_id()` is a thin wrapper over it. The value is the text-frame field index
(1 vs 2) used by text-framed test fixtures — populate the right bucket.

`MessageBusStub` tests bypass the dispatcher, so they pass with the registration missing.

## The gate

`debug_assert_request_id_routable` (`src/subscriptions/common.rs`) runs in both subscription
constructors and panics when a `request_id`-keyed subscription is built for a decoder whose
`RESPONSE_MESSAGE_IDS` names a type the dispatcher cannot route to it. The classification is
`first_unroutable_by_request_id` (`src/transport/routing.rs`), whose two accepting arms mirror
`determine_routing`: order-scoped types, and anything with a `text_request_id_field` entry.
`Error` is deliberately not accepted; see [proto-only decoding](proto-only-decoding.md).

Stub tests bypass the dispatcher but not the constructor, so this fires in exactly the tests that
used to pass silently. It is compiled out of release builds (the invariant is over static tables).
Its reach is the set of decoders some test instantiates — one more reason for
[coverage floor](../testing/coverage-floor.md).

## Precedents

- #519 — a panicking accessor on a proto-framed message.
- #647 — shipped a missing routing registration (caught only by a live smoke test), then split
  the table into `routes_by_request_id` + `text_request_id_field`.
- #730 — replaced the prose warning with the constructor guard. First run found `MarketDataType`,
  declared by the `TickTypes` decoder since #516 but missing from the table, so
  `TickTypes::MarketDataType` never reached a `market_data` subscription.
- #734 — removed the guard's `Error` exemption along with the sixteen decoders declaring it.
