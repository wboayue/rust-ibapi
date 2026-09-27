# Attached SL/PT orders (`PlaceOrderRequest.attached_orders`)

Issue #842. Raised as an aside in #841.

**Implemented** on branch `orders/attached-preset-orders`. Phase 0 harness deleted. Integration
tests `preset_attached_orders_accepted` (sync + async) pass live with outcome "no preset".
Example is async-only (`examples/async/preset_attached_orders.rs`): the outcome is a notice,
and watching the update stream alongside is the async idiom. Integration tests are not
version-gated — `server_versions` is crate-private; a gateway below 218 fails loudly with
`Error::ServerVersion`.

## Problem

`encode_place_order` (`src/orders/common/encoders.rs`) hard-codes `attached_orders: None`, so
the crate cannot ask TWS to create server-side stop-loss / profit-taker children from a parent.

## Wire (verified at plan time)

- `AttachedOrders.proto`: `optional int32 slOrderId`, `optional string slOrderType`,
  `optional int32 ptOrderId`, `optional string ptOrderType`. No price fields.
- Carried on `PlaceOrderRequest`, not on `Order` proto. Nothing inbound echoes it:
  `grep -rni slorder src/` hits only the generated proto.
- C# / Java / Python keep the four as `Order` fields (id sentinel `int.MaxValue`, type `""`);
  `createAttachedOrdersProto` sets each only when valid / non-empty.
- Gate: `MIN_SERVER_VER_ATTACHED_ORDERS = 218` (`server_versions::ATTACHED_ORDERS` exists).
  Below it C# rejects any populated field with `UPDATE_TWS`. Our floor is 213, so the check is
  live.
- Only documented type value: `"PRESET"` — `samples/Python/Testbed/OrderSamples.py`
  `LimitOrderWithStopLossAndProfitTaker`. Children's prices come from TWS order presets.
- Ids are client-allocated: Java `ApiController.placeOrModifyOrder` does `slOrderId(m_orderId++)`
  / `ptOrderId(m_orderId++)` when the type is set; the Python testbed passes two
  `nextOrderId()`s.
- Routing: `OpenOrder` / `OrderStatus` are `OrderOrShared` (`src/transport/routing.rs`). Child
  ids have no registered channel, so today their frames reach only `order_update_stream`, not
  the parent's `place_order` subscription.

## Phase 0 — live discovery (paper gateway, before any API)

Per verify-wire-before-typing (memory). Temporary `#[ignore]` tests / a scratch example, raw
`Order` + hand-built `PlaceOrderRequest`, deleted before the PR.

1. Confirm the paper gateway's `server_version() >= 218`. If not, the whole issue waits.
2. Type values: `PRESET` (expected OK), then `STP`, `LMT`, `TRAIL`, `""`, garbage. Record
   which are accepted vs rejected (error code + text). Decides the enum.
3. Type set, id absent: does TWS allocate, reject, or ignore? Decides whether ids are required.
4. Id set, type absent: same question.
5. Frames for child ids: `OpenOrder` / `OrderStatus` shape, `parent_id` on the child, ordering
   vs parent, and whether they arrive before parent fill (held) or only after.
6. `transmit = false` on the parent: are children created held too; does a later transmit of
   the parent release them?
7. No presets configured for the contract/account: error or silent no-op?
8. Cancel the parent: do children cancel (OCA-like), and which frames arrive?

Record findings in this file (and a `reference_*` memory) before Phase 1.

### Findings — run 2026-09-27 00:06 ET, paper gateway, server 220

Harness: `src/orders/attached_discovery_tests.rs` (temporary, since deleted). Every frame below carried the
**parent** id; no frame for a child id ever arrived.

| # | Case | Result |
|---|------|--------|
| 1 | server version | 220 ≥ 218 ✅ |
| 2 | type `STP`, `LMT`, `TRAIL`, `""`, `BOGUS` | code **320** `Error reading request. Invalid value for Stop Loss order-id or order-type`. Request rejected whole: no status frames, later cancel → 10147 not found |
| 2 | type `PRESET` (AAPL, both legs) | code **10355** `Cannot auto-attach Profit Taker. Preset is not defined.`, then `OrderStatus Cancelled` (perm id assigned) + **202** `Order was discarded` |
| 3 | type `PRESET`, ids absent | **320** (same text) — ids are required |
| 4 | ids set, type absent | **320** (same text) — type is required |
| 5 | SL only / PT only, `PRESET` | 10355 naming that leg (`Stop Loss` / `Profit Taker`), parent discarded |
| 6 | parent `transmit=false`, `PRESET` | same 10355 + discard: presets are resolved at placement, `transmit` doesn't defer it. Re-send of the id → **103** `Duplicate order id` (a discarded id stays consumed) |
| 7 | EUR.USD IDEALPRO, `PRESET` | 10355 — no presets for forex either |
| 8 | cancel parent | n/a — parent never lived |

Conclusions:

- **`PRESET` is the only accepted type value.** Everything else fails TWS's request validation.
  The 320 text always names "Stop Loss", even with both legs bad — the SL leg is checked
  first, so the text doesn't say which leg is wrong.
- **Each leg needs both id and type.** Ids stay client-allocated. (Design: the type is
  always `PRESET`, so the public API carries only the id.)
- **Attach failure kills the parent.** 320 → the order is never created. 10355 → created, then
  discarded (Cancelled + 202). There is no "parent placed, children skipped" outcome. Doc this
  on the builder.
- **Items 5 (child frames), 6 (transmit semantics with working children) and 8 (parent cancel)
  are unobserved**: this paper account has no order presets. IB Gateway has no presets UI;
  defining stock presets with an attached SL/PT in TWS (Global Configuration → Presets), for
  the same paper user with settings stored on server, then re-running `type_values` /
  `transmit_false_then_true` should unblock them.

## Design

Decided after Phase 0: no order-type enum. `PRESET` is the only value TWS accepts, so the
encoder always sends it and the public API carries only the child ids.

### Types (`src/orders/mod.rs`)

```rust
// on Order — request-only; never set on decoded orders:
/// Some(id): ask TWS to attach its preset stop-loss to this order, under `id`.
pub preset_stop_loss_order_id: Option<i32>,
/// Some(id): ask TWS to attach its preset profit-taker to this order, under `id`.
pub preset_profit_taker_order_id: Option<i32>,
```

- No `AttachedOrderType` / `AttachedOrder`: with the type fixed, an id is the whole leg.
  Phase 0 items 3/4 (320 on either half alone) can't happen — the encoder always sends the
  pair, so a type without an id can't be expressed.
- `preset_` in the names: the children's prices come from TWS presets, not from the caller.
  Keeps them apart from `BracketOrderBuilder`'s price-taking `stop_loss` / `take_profit`.
- If IBKR ever accepts other types: add the enum then, as a new field or builder argument.
  Until then [wire enum typing](../docs/rules/wire/enum-typing.md) has nothing to type.
- Doc both fields as request-only: `Order` is also the decoded `OpenOrder` payload, and these
  stay `None` there. Doc that a missing preset discards the parent (10355 + 202).
- `Order` has no `#[non_exhaustive]`; the new fields break exhaustive struct literals →
  changelog `### Changed` + `docs/migration-4.0.md` section.
- `i32`, matching `Order.order_id` / `parent_id`. Typed ids (#789, `plans/typed-ids.md`, not
  landed) must sweep these two fields along with the rest.
- Field docs show the raw path: `next_order_id()` per leg, set the fields, `place_order` —
  the only way to get a subscription for an order with attached legs.

Alternative considered: keep them off `Order`, pass the ids to the encoder through a new
`place_order_with_attached`. Rejected — widens the client surface, and every reference client
models them on `Order`.

### Encoder

New `proto::encoders::encode_attached_orders(&Order) -> Option<proto::AttachedOrders>`, next
to `encode_order`: each `Some(id)` sets `*_order_id = id` and `*_order_type = "PRESET"` (one
private `const PRESET: &str`); `None` when both absent (keeps existing bytes identical —
assert that in a test). `encode_place_order` stays assembly-only and calls it.

### Version check

`verify_order` (`src/orders/common/verify.rs`):
`if order.preset_stop_loss_order_id.is_some() || order.preset_profit_taker_order_id.is_some()`
→ `check_version(server_versions::ATTACHED_ORDERS, "It does not support attached orders.")`.
Covers `place_order`, `submit_order`, and the builders, sync and async.

### Builder entry point

Like `bracket()` → `BracketOrderBuilder` → `BracketOrderIds`, but the first leg is the
transition — no empty `.attached()` step:

```rust
client.order(&contract).buy(100).limit(50.0)
    .preset_stop_loss()               // OrderBuilder -> AttachedOrdersBuilder
    .preset_profit_taker()            // AttachedOrdersBuilder -> Self
    .submit()                         // -> AttachedOrderIds { parent, stop_loss: Option<OrderId>, profit_taker: Option<OrderId> }
```

- `OrderBuilder::preset_stop_loss()` / `preset_profit_taker()` return an
  `AttachedOrdersBuilder` holding the `OrderBuilder` plus two `bool`s; the same two methods on
  it set the other flag. "No legs" is unrepresentable, so no runtime validation.
- Separate type rather than flags on `OrderBuilder`: plain `submit()` returns one `OrderId`
  and would silently drop the child ids.
- No `build()` on `AttachedOrdersBuilder`: child ids come from the client at submit, so a
  built `Order` couldn't carry them. Raw-path callers set the fields themselves.
- Id reservation (parent → SL → PT) lives once, in the generic impl in `order_builder.rs`:
  `fn build_with_ids(self, next_id: impl FnMut() -> i32) -> Result<(Order, AttachedOrderIds),
  ValidationError>`. `sync_impl.rs` / `async_impl.rs` `submit()` = `build_with_ids(||
  client.next_order_id())` + one `submit_order`. (`submit_all` already duplicates its
  reservation across both files — not touched here.)
- `submit()` is fire-and-forget, like `OrderBuilder::submit()`: the likely failure — no
  preset, 10355 then Cancelled/202 — shows up only on `order_update_stream`. The doc says
  so and the example watches that stream.
- No `analyze()` (what-if) on `AttachedOrdersBuilder`: out of scope.
- No enum, so [builder enum coverage](../docs/rules/style/builder-enum-coverage.md) doesn't
  apply.
- `sync_impl.rs` + `async_impl.rs` both, per [feature matrix](../docs/rules/parity/feature-matrix.md).
- `BracketOrderBuilder::stop_loss(price)` / `take_profit(price)` are client-side and
  price-taking. Doc on both builders points at the other.

### Routing (decide after Phase 0 item 5)

Option A (minimal): document that child frames arrive on `order_update_stream` only.
Option B: register child ids onto the parent's order channel in `send_order_request` so a
`place_order` subscription sees the whole family. B touches both transports and
`Subscription` → [integration crate builds](../docs/rules/workflow/integration-crate-builds.md). Default to A in this issue; open a follow-up for
B if Phase 0 shows callers need it.

`place_order` with attached legs (non-builder path) keeps its current subscription shape.

## Tests

- Encoder (`src/orders/common/encoders_tests.rs` or sibling): both legs, SL only, PT only,
  neither → `attached_orders` absent; each set leg carries `"PRESET"`. Decode captured bytes back to `PlaceOrderRequest`, per
  [exercise production code](../docs/rules/testing/exercise-production-code.md).
- `verify_order`: server 217 + leg → `Err`; 218 → ok; no legs at 213 → ok. Derive versions from
  `server_versions::ATTACHED_ORDERS`, per [derive from constants](../docs/rules/testing/derive-from-constants.md).
- Builder: `build_with_ids` directly (reservation order, SL-only / PT-only / both, flags idempotent)
  with a counter closure — no client needed; then one sync and one async `submit()` via `create_test_client` / `create_blocking_test_client`.
- Integration (`integration/{sync,async}/tests/orders.rs`), gated on server ≥ 218: paper LMT
  far from market with both legs. The paper account has no presets today, so assert the
  observed outcome — 10355 then Cancelled/202 for the parent. Once presets exist (items 5/8),
  switch to asserting child `OpenOrder` frames with `parent_id == parent` and children going
  on parent cancel.

## Docs / housekeeping

- `# Examples` on every new `pub fn` ([public API examples](../docs/rules/docs/public-api-examples.md)).
- Example `examples/{sync,async}/attached_orders.rs` if the bracket example has siblings there.
- `CHANGELOG.md` `[Unreleased]`: `### Added` (`preset_*` builder methods, `AttachedOrdersBuilder`, `AttachedOrderIds`), `### Changed` (`Order` fields).
- `docs/migration-4.0.md` new section; `README.md` if it lists order builders.
- Full pre-PR gate incl. `cargo build -p ibapi-integration-{sync,async} --tests`,
  `just rules-check` (this file is under `plans/`).

## Open after Phase 0

Resolved: no type enum (`PRESET` only, always sent), ids required, attach failure discards
the parent.

Still open — need a paper account with order presets defined (see Findings). Tracked in
[attached-orders-followups](attached-orders-followups.md):

- Item 5: child frame shape / timing → decides the routing option.
- Item 6: whether `transmit=false` holds working children.
- Item 8: whether cancelling the parent cancels the children.
