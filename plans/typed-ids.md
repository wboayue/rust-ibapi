# Typed ids and a partitioned id space — design for #789

Issue #789: request ids (minted from 9000 in every process) and order ids
(seeded from the server's `NextValidId`, growing across sessions) share one
`i32` space with nothing keeping them apart. An inbound error frame carries a
single integer, so once a live request and an order hold the same number, no
routing policy can recover the destination: the order's cancellation
confirmation or 201 rejection lands on an unrelated data subscription, or a
late error for a finished request is published on the order-update stream as
if it were order-bound.

The fix has to make collision impossible before the frame arrives. This plan
does that in two layers: a **partition** of the `i32` space that makes the two
id domains disjoint by construction, and **typed ids** that make the partition
a property the compiler checks rather than a convention the routing code
remembers.

## 1. Constraints

- **Order ids are external input.** `place_order`, `submit_order`, and
  `cancel_order` take a caller-supplied `i32`. Callers get it from
  `next_order_id()` (local generator), from `next_valid_order_id()` (a server
  round-trip, returns whatever TWS considers next), or from their own scheme.
  The crate cannot assume it minted the order id.
- **Request ids are internal.** Every request id is minted by
  `ClientIdManager::next_request_id`. The only public exposure is
  `Client::next_request_id()` and read-only accessors
  (`Subscription::request_id()`, `Notice::request_id`). No public entry point
  accepts a caller-chosen request id; the `with_id` builders are `pub(crate)`.
- **The wire is one integer.** `ErrorMessage.id` is the same field for both
  domains. `ResponseMessage::request_id()` and `::order_id()` are already
  distinct accessors, but for error frames both read the same value.
- **TWS chooses the order floor.** `NextValidId` on connect and reconnect
  seeds `ClientIdManager` (`client/{sync,async}.rs`); #802 and #803 made that
  a monotonic raise. The plan must not fight that.

This rules out the ib_insync approach (one shared counter for both domains):
it closes the `next_order_id()` path but a caller who sources an id from
`next_valid_order_id()` can still land on a live request id, because the
server only knows about orders it has seen.

## 2. The partition

```rust
/// First request id. Every request id is >= this; every order id is < this.
pub(crate) const REQUEST_ID_FLOOR: i32 = 1 << 30; // 1_073_741_824
```

- Request ids ascend from `REQUEST_ID_FLOOR` up to and including
  `i32::MAX - 1`. Headroom is ~1.07e9 ids per process. `i32::MAX` itself is
  excluded: TWS omits the id on error frames at that value (§4 step 7). On
  reaching it the allocator (`fetch_update`, not `fetch_add`) logs at `error`
  and panics — a billion requests in one process is a bug, not a workload.
- Order ids stay below the floor. Enforced at every point an order id enters
  the crate (§3.2). A server `NextValidId` at or above the floor is treated as
  a connection error; IB's per-account sequence is in the millions for heavy
  users, so this is a "cannot happen" guard with a clear message rather than a
  real branch.
- Inbound classification is by range, not by table lookup:

  ```rust
  pub(crate) enum WireId { Request(RequestId), Order(OrderId) }
  impl WireId { pub(crate) fn classify(id: i32) -> Option<WireId> } // None for -1
  ```

  This replaces `id_owned_by_data_request` in `transport/routing.rs` and the
  request-then-order fallback in both transports' `deliver_to_request_id`.

Ascending from the floor rather than descending from `i32::MAX` keeps ids
sortable in logs and captures and makes the predicate a single comparison.

Rejected partitions: parity or a high-bit tag on order ids — the server picks
order ids, so nothing can be imposed on their shape. Tracking "live" order ids
to steer the request allocator around them — open orders survive sessions and
receive error frames indefinitely, so there is no liveness bound to track.

## 3. Typed ids

```rust
// src/client/ids.rs  (new; sibling tests in ids_tests.rs)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct RequestId(i32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct OrderId(i32);
```

- `RequestId` has no public constructor. `ClientIdManager::next_request_id`
  is the only minting site; `WireId::classify` is the only inbound site.
- `OrderId` converts from and to `i32` infallibly, the shape `ContractId`
  already uses (`accounts/types.rs`):

  ```rust
  impl From<i32> for OrderId { .. }        // caller-held ids enter without ceremony
  impl From<OrderId> for i32 { .. }        // persistence, logging, arithmetic on the way out
  impl PartialEq<i32> for OrderId { .. }   // `status.order_id == my_id` keeps compiling
  impl PartialEq<OrderId> for i32 { .. }
  ```

  Every public entry point that takes an order id takes `impl Into<OrderId>`,
  so an existing `i32` argument compiles unchanged and a typed `OrderId`
  passes through. The conversion has to be infallible for this to work: a
  `TryFrom<i32>` bridges to `Into<Result<OrderId, _>>`, not `Into<OrderId>`,
  and would not satisfy the bound.
- The range guard therefore sits after conversion, not in it:
  `OrderId::checked(self) -> Result<OrderId, Error>` (`pub(crate)`) is the
  single home. `Error::InvalidArgument` with a message naming the floor is
  enough; no new variant. It runs at every point an order id crosses into the
  crate (§4 step 4) — never at construction, so a caller can build, compare,
  and log an `OrderId` freely and only pays the check when it is sent.
- `RequestId` gets `From<RequestId> for i32` and `PartialEq<i32>` only. No
  `From<i32>`: request ids are internal (§1), and an inbound `i32` becomes a
  `RequestId` solely through `WireId::classify`.
- Both expose `.raw() -> i32` for encoders and `Display` for logs. No `Deref`
  to `i32` and no `Add`/`Sub`; the point is that arithmetic and cross-domain
  comparison stop compiling inside the crate. `OrderId == RequestId` is a
  type error; `OrderId == i32` is allowed because callers already hold `i32`s.

What typing buys beyond the partition:

- `transport/async.rs` `request_channels: HashMap<RequestId, _>` and
  `order_channels: HashMap<OrderId, _>`; `transport/sync.rs`
  `requests: SenderHash<RequestId, _>` and `orders: SenderHash<OrderId, _>`.
  Looking an `OrderId` up in the request table stops compiling. The
  `ExecutionData` strategy (order table, then request table) becomes explicit
  about which id it is holding at each step.
- `OrderRequestBuilder::with_id` takes `OrderId`; `RequestBuilder::with_id`
  takes `RequestId`. A domain module cannot hand an order id to a request
  builder.
- `ResponseMessage::request_id()` and `::order_id()` return the typed ids, so
  the accessors' existing separation becomes load-bearing.

## 4. Scope split

### PR 1 — partition + internal typing (closes #789)

Everything above, kept inside the crate. Public signatures unchanged; public
*values* change (`next_request_id()` returns numbers above 2^30).

1. `src/client/ids.rs`: `RequestId`, `OrderId`, `WireId`, `REQUEST_ID_FLOOR`.
   `pub(crate)` for now (see PR 2).
2. `ClientIdManager`: request generator starts at the floor; `next_request_id`
   returns `RequestId`; `next_order_id` returns `OrderId`;
   `raise_order_id(OrderId)`. `new(initial_order_id)` validates the seed.
   Drop `INITIAL_REQUEST_ID`.
3. Transports: table key types; `deliver_to_request_id` becomes
   `deliver(WireId, RoutedItem)` with one lookup per arm; `route_error_message`
   classifies with `WireId::classify` and the order-update copy is
   `matches!(id, WireId::Order(_))`. Delete `order_update_notice`'s
   `id_owned_by_data_request` parameter.
4. Order entry points: `place_order`, `submit_order`, `cancel_order`,
   `OrderRequestBuilder::with_id` take `impl Into<OrderId>`, then
   `.into().checked()?` before the id reaches an encoder or a routing table.
   Public signatures gain the `impl Into` bound in PR 1 already — it is
   source-compatible for every `i32` caller, and doing it here means PR 2
   changes only return types and struct fields. `next_valid_order_id()`
   result is `checked()` the same way before it raises the generator.
5. Connection seed: `ConnectionMetadata.next_order_id` validated at
   `ClientIdManager::new`; a floor-or-above value is `Error::ConnectionRejected`
   with the value in the message.
6. Tests. In order:
   - `ids_tests.rs`: classify at floor, floor − 1, −1, `i32::MAX`;
     `OrderId::from(REQUEST_ID_FLOOR).checked()` rejects, `floor − 1` passes;
     `i32` round-trips through `OrderId` unchanged; `OrderId == i32` and
     `i32 == OrderId` both directions.
   - `id_generator_tests.rs`: request and order sequences never intersect
     across a reconnect reseed; allocator panics on wrap.
   - `transport/routing_tests.rs`: the two scenarios from #789 — order id
     equal to a live request id, and a late request error while an order with
     that number is live — each land on the right side.
   - Test fixtures currently hardcoding 9000-based ids: 176 literals in 23
     files (`grep -rlE '\b900[0-9]\b' src --include='*_tests.rs'`). Migrate to
     `REQUEST_ID_FLOOR + n` per
     `docs/rules/testing/derive-from-constants.md`; do not re-hardcode.
   - Stubbed clients (`Client::stubbed` seeds `ClientIdManager::new(9000)`)
     keep a sub-floor order seed; the value is arbitrary and should become a
     named test constant.
7. Wire verification — **done 2026-09-11** against the paper gateway with a
   temporary in-crate probe (`send_request` with a chosen id +
   `encode_request_contract_data`, AAPL for the data path and a bogus symbol
   for the error-200 path), recorded with `IBAPI_RECORDING_DIR`:

   | request id | ContractData / End echo | error 200 echo |
   |---|---|---|
   | 0, 9000, `1<<24`, `1<<30`, `i32::MAX-1` | yes | yes |
   | `-2`, `-1000`, `i32::MIN+1` | yes | yes |
   | `i32::MAX`, `i32::MIN` | yes | **no — frame omits field 1, decodes as `-1`** |

   TWS echoes the full `i32` range on data frames, negatives included. On
   error frames it drops the id at exactly `i32::MAX` and `i32::MIN`, so an
   error for a request at either value arrives request-less and is
   unroutable. Consequences: the floor at `1 << 30` stands; the request
   allocator's last valid id is `i32::MAX - 1` and it must stop (panic, per
   §2) before handing out `i32::MAX`. Negative ids would also work but stay
   rejected (§1: `-1` is reserved, and the positive floor keeps the
   predicate a single comparison). PR 1 adds the `#[ignore]`d live test
   asserting error-frame echo at `REQUEST_ID_FLOOR` and `i32::MAX - 1`, in
   the sync integration crate rather than under `src/` (no in-crate live
   tests exist; `send_request` and the contract-data encoder are crate-private,
   so the test drives `contract_details` and reads the request id off the
   `Notice`). The probe itself was scratch and is not committed; the recipe
   above is enough to revive it.
8. Integration crates: `cargo build -p ibapi-integration-sync -p
   ibapi-integration-async --tests`. Any test asserting a literal request id
   in captured bytes moves to the constant.
9. Changelog under `## [Unreleased]` → `### Fixed`, citing #789: request ids
   now start at 2^30 and order ids above that floor are rejected at
   `place_order` / `submit_order` / `cancel_order` with `InvalidArgument`.
10. Rule node: `docs/rules/wire/id-partition.md` (new) — "Adding a public API
    that accepts an order id → take `impl Into<OrderId>` and call `.checked()?`
    before the id crosses the domain boundary; one that accepts a request id
    → take `RequestId`, never `i32`. Inbound `i32`s become typed only through
    `WireId::classify`." Index line under *Wire protocol* in `CLAUDE.md`;
    `just rules-check`.

### PR 2 — public typed ids (separate decision)

Promote `RequestId` and `OrderId` to `pub` and change signatures:

- `Client::next_request_id() -> RequestId`, `next_order_id() -> OrderId`,
  `next_valid_order_id() -> Result<OrderId, _>`.
- `place_order(OrderId, …)`, `submit_order(OrderId, …)`,
  `cancel_order(OrderId, …)`, `cancel_historical_ticks(RequestId)`,
  `cancel_contract_details(RequestId)`.
- `Subscription::request_id() -> Option<RequestId>`; `Notice.request_id`
  becomes `Option<WireId>` (an error notice can be either); `OrderStatus`,
  `OpenOrder`, `ExecutionData.order_id: OrderId`.
- `order_builder::market_f_hedge(parent_order_id: OrderId, …)`.

Allowed under the crate's breaking-release stance, but it touches every order
example and both integration crates, so it is worth its own review. The
conversions in §3 are chosen to keep most caller code compiling across it:

| Caller pattern today | After PR 2 |
| --- | --- |
| `client.place_order(my_i32, &c, &o)` | unchanged (`impl Into<OrderId>`, landed in PR 1) |
| `let id = client.next_order_id(); client.place_order(id, ..)` | unchanged |
| `if status.order_id == my_i32` | unchanged (`PartialEq<i32>`) |
| `let id: i32 = client.next_order_id()` | `i32::from(..)` / `.into()` |
| `next_order_id() + 1`, `id as i64` | `.raw()` first — arithmetic on ids is what the type exists to stop |
| `HashMap<i32, _>` keyed by `status.order_id` | key type becomes `OrderId`, or `.raw()` at insert |
| `Subscription::request_id() == Some(9001)` | `PartialEq<i32>` on `RequestId`; the value moved to 2^30+ in PR 1 anyway |

Decision: `impl Into<OrderId>` on parameters, infallible `From<i32>`, guard at
the call site via `checked()`. The earlier alternative — a fallible
`OrderId::new(id)?` at every call site — made the floor visible to callers but
forced a rewrite of every order example for a check that fails only on a
misconfigured id; the loud `InvalidArgument` from `place_order` carries the
same information at the moment it matters. Migration guide §N lists the
right-hand column above; README and `docs/` snippets are grepped for
`next_order_id()` arithmetic (`grep -rn "next_order_id() *[+-]" README.md docs
examples`).

Do not fold PR 2 into PR 1. PR 1 is a routing correctness fix with a
mechanical test migration; PR 2 is an API-shape decision.

## 5. Behaviour changes visible to callers (PR 1)

- `next_request_id()` and `Subscription::request_id()` return values at or
  above 1_073_741_824 instead of 9000-based ones. Anyone logging or
  persisting them sees larger numbers; nothing else changes.
- An order id at or above the floor is rejected up front with
  `Error::InvalidArgument` from `place_order` / `submit_order` /
  `cancel_order`. Previously it would have been sent and, if it collided,
  misrouted.
- Order entry points take `impl Into<OrderId>`. Existing `i32` arguments
  compile unchanged; the typed value is opt-in until PR 2 changes returns.
- A late error frame for a dropped request no longer reaches the
  order-update stream, and an order's error frame no longer reaches a data
  subscription. Both were the #789 symptoms.

## 6. Open items

- Wire check in §4 step 7 gates the floor value. Record the outcome here.
- `messages::routes_by_request_id` allow-list: unchanged by this plan, but
  once `request_id()` returns `RequestId` the allow-list is the only thing
  keeping non-request messages from being classified — worth a comment on
  the accessor pointing at `WireId`.
- Related follow-up in `plans/transport-cleanup-followups.md` if the
  `deliver` refactor exposes further duplication between the sync and async
  `route_error_message` shells.
