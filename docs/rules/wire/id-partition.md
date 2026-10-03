---
id: id-partition
title: Request ids and order ids live in disjoint ranges
cluster: wire
status: active
triggers:
  - adding a public API that accepts an order id
  - adding a request that allocates a request id
  - routing an inbound frame or error by its id
  - adding an Order field that carries another order's id
symbols: [REQUEST_ID_FLOOR, RequestId, OrderId, WireId, WireId::classify, OrderId::checked, verify_order_ids, Client::mint_request_id]
related: [proto-aware-accessors]
precedents: ["#789"]
memory: [project_issue_audit_788_790, reference_tws_request_id_range]
---

An error frame carries one integer for both requests and orders, so the two
id domains must never share a number. The `i32` space is split at
`REQUEST_ID_FLOOR` (`src/client/ids.rs`): request ids are at or above it,
order ids below it. An inbound id's **range** names its domain — never which
table happens to hold the number.

**Minting a request id** → `Client::mint_request_id()` (or a `RequestBuilder`),
which yields a `RequestId`. Pass `request_id.raw()` to the encoder and the
`RequestId` to the bus. There is no public way to mint one: request ids are
crate-minted only, and callers read a live request's id from
`Subscription::request_id()` / `ContractDetailsBuilder::request_id()`.

**Accepting an order id from a caller** → take `impl Into<OrderId>`
(`orders::OrderId`, public; `i32` converts), then check at the entry point:
`verify::verify_order_ids(order_id.into(), &order)?` for anything that places
an order (it also checks `parent_id` and the preset attached-order ids, since
TWS routes frames for those orders by them), or `order_id.into().checked()?`
for a bare id (`cancel_order`). Both return `Error::OrderIdInRequestRange`.
The check runs where the id crosses into the crate, never at `OrderId`
construction. An `Order` field that names another
order's id belongs in `verify_order_ids`.

**Routing by an inbound id** → `WireId::classify(id)` and match on the arm:
`Request` looks only in the request table, `Order` only in the order table,
`None` (TWS's `-1`) is request-less. For a frame that carries both a request id
and an order id in separate fields (`ExecutionData`), convert each field with
`RequestId::from_raw` / `OrderId::from` — they are different fields, not one
ambiguous number. Do not reintroduce a "try requests, then orders" fallback or
an ownership check on the error path.

**Tests** → derive request ids from `RequestId::nth(n)` (floor + `n`) and put
`.raw()` of the same value in the frames meant for them; keep order ids small.
A frame built with a sub-floor id is an order frame.

Why the floor is 1,500,000,000: order ids span the account's lifetime
(`nextValidId` persists across sessions) plus caller numbering schemes, so
they get the larger share; request ids span one process. TWS echoes every id
except `i32::MAX` and `i32::MIN` on error frames, so the allocator stops at
`i32::MAX - 1`.

**Public id types are not typed beyond `OrderId` parameters** (decided for 5.0).
Order methods take `impl Into<OrderId>`, but `next_order_id()`,
`next_valid_order_id()`, the `order_id` fields on `Order` / `OrderStatus` /
`Execution` / …, `Subscription::request_id()` and `Notice::request_id` stay
`i32`, and `RequestId` / `WireId` stay `pub(crate)`. Typing them would break
every caller's `let id: i32`, `HashMap<i32, _>` key and id arithmetic to guard
against passing a request id to `cancel_order` — a mistake no issue has
reported, and #789 is fixed internally either way. Revisit only on user demand.
