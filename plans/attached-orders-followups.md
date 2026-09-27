# Preset attached orders — follow-ups

Follow-up to #842 (shipped in #849; its plan, `plans/attached-orders.md`, is in that PR's history). The encoding shipped; these Phase 0
items were never observed because the paper account has no order presets, so every attach
ended in 10355 + parent discarded.

## Unblock first

Define stock presets with an attached stop-loss and profit-taker in TWS (Global
Configuration → Presets) for the paper user, with settings stored on server so IB Gateway
picks them up. Confirm with `preset_attached_orders_accepted` (sync or async): its printed
outcome flips from `no preset` to `attached`.

## Open questions

1. **Child frames** (Phase 0 item 5). For the SL / PT ids: `OpenOrder` / `OrderStatus` shape,
   `order.parent_id`, `order_type` / prices TWS chose, ordering relative to the parent, and
   whether they arrive at placement (held) or only after the parent fills. Also any
   `OrderBound` frames for the child ids.
2. **`transmit = false`** (item 6). Are the children created held with the parent, and does a
   later re-send of the parent with `transmit = true` release them? Phase 0 showed a re-send
   after a discard gets 103 Duplicate order id, so test against a *working* held parent.
3. **Parent cancel** (item 8). Do the children cancel with the parent (OCA-like), and which
   frames/notices arrive for each id?
4. **Modify.** Re-sending the parent (same id, new limit) with the same preset ids: does TWS
   keep, re-create, or reject the children? Without the preset ids?

## Decisions that hang on the answers

- **Routing** (#849 plan, § Routing).
  Shipped as Option A: child frames reach only `order_update_stream`. If item 1 shows callers
  need the family on one subscription, Option B registers the child ids onto the parent's order
  channel in `send_order_request` (both transports; run the integration crate builds).
- **Integration test.** Once presets exist, tighten `preset_attached_orders_accepted` to require
  `attached`, assert both children's `parent_id`, and add a parent-cancel assertion from item 3.
- **Docs.** `Order::preset_*_order_id`, `OrderBuilder::preset_*`, `docs/order-types.md` and
  `examples/async/preset_attached_orders.rs` describe only the no-preset failure; add the
  observed child lifecycle and `transmit` behavior.

## Harness

The Phase 0 harness was deleted before the PR. Rebuild it as a temporary `#[ignore]`d
in-crate async test (`#[cfg(all(test, feature = "async"))]`): open `order_update_stream()`,
place a far-from-market parent through the builder
(`.preset_stop_loss().preset_profit_taker().submit()`), print every item for ~6s, cancel the
parent, print again, then cancel the child ids. The public API now sets `attached_orders`, so
no hand-built `PlaceOrderRequest` is needed. Record findings in this file and in the
`reference_attached_orders_wire` memory.
