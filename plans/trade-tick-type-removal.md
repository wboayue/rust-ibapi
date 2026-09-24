# Remove `Trade.tick_type`

**Implemented** on branch `market-data/remove-trade-tick-type`.

Follow-up from the PR #831 review. #831 corrected the doc comment (the field holds the wire
code `"1"`/`"2"`, not `"Last"`/`"AllLast"`). Typing it as an enum was considered and dropped:
the field carries no information the caller doesn't already have.

## Problem

- `Trade.tick_type: String` (`src/market_data/realtime/mod.rs`) holds `"1"` or `"2"`, a
  stringified wire code set by `tick_type.to_string()` in `decode_trade_tick_proto`
  (`src/market_data/realtime/common/decoders/mod.rs`). Magic strings, and the natural guess
  (`"Last"`) is wrong.
- It is constant per stream. `Trade` is reachable only as the item of
  `tick_by_tick(..).last()` (always 1) and `tick_by_tick(..).all_last()` (always 2), sync and
  async (`src/market_data/realtime/builder/tick_by_tick.rs`). The method the caller picked
  already says which feed it is.
- It is residue of the C# callback shape: there both feeds share one callback,
  `tickByTickAllLast(reqId, tickType, ...)`, so `tickType` is how the callback tells them
  apart. Our API gives each feed its own typed stream.

## Why not an enum

A `TradeTickType { Last, AllLast }` only helps a caller who merges both feeds into one
stream. That case is weak:

- Merging several tick-by-tick streams already means tagging at merge time — `Trade` carries
  no contract or request id either — so the feed tag costs nothing extra there.
- `AllLast` is a superset of `Last`; subscribing to both on one contract spends two scarce
  tick-by-tick slots for data one of them already has. Subscribe `AllLast` and filter on
  `special_conditions` instead.
- Serialized `Trade` records already need a wrapper for the contract; the feed belongs there.

Removal is also smaller: no new public type, no serde form change, no exception to the
enum-typing node's "no closed integer enum" rule.

## Consumers (verified at plan time)

`grep -rn "tick_type" examples/ integration/ docs/ README.md`: no reads of `Trade.tick_type`.
The tick-by-tick examples (`examples/{sync,async}/tick_by_tick*.rs`) and
`integration/{sync,async}/tests/realtime_data.rs` print `{:?}` or read price/size/exchange.
Re-run before the PR.

## Changes

1. `src/market_data/realtime/mod.rs` — delete the `tick_type` field from `Trade`. The struct
   doc (`Represents Last or AllLast ...`) stays.
2. `src/market_data/realtime/common/decoders/mod.rs` — drop `tick_type: tick_type.to_string()`
   from the `Trade` literal. **Keep** the `1 || 2` guard: it still catches a misrouted frame
   (e.g. a BidAsk code on a trade subscription), and the rejection tests depend on it.
3. `src/market_data/realtime/common/decoders/tests.rs`:
   - `test_decode_trade_tick_proto_last` — drop the `tick_type` assert; the rest stays.
   - `test_decode_trade_tick_proto_all_last` — its only assert was `tick_type == "2"`. Keep it
     as "code 2 is accepted" by asserting a payload field (e.g. price), so the guard's second
     arm stays covered.
   - Add: absent `tick_type` (proto `None` → 0) is rejected with `Unexpected tick_type`, if not
     already covered (at plan time only code 3 is tested).
   - The testdata builder's `tick_type` (`src/testdata/builders/market_data.rs`) is wire input
     and stays.
4. `CHANGELOG.md` `## [Unreleased]` → `### Removed`: `market_data::realtime::Trade::tick_type`
   — it held the wire code `"1"`/`"2"` and is constant per stream; the method that opened the
   stream (`.last()` / `.all_last()`) names the feed. See `docs/migration-4.0.md` §18.
5. `docs/migration-4.0.md` §18 — before/after for a caller that merges feeds:

   ```rust
   // before
   if trade.tick_type == "2" { ... }

   // after: tag at merge time
   let last = client.tick_by_tick(&contract, 0).last().await?.filter_data().map(|t| (Feed::Last, t));
   let all = client.tick_by_tick(&contract, 0).all_last().await?.filter_data().map(|t| (Feed::AllLast, t));
   let merged = futures::stream::select(last, all);
   ```

   plus the `AllLast` + `special_conditions` note. Add the item to the quick-migration
   checklist.
6. `grep -rn "tick_type" docs/rules/ plans/` for anything citing the field (user-docs-sync /
   rule-graph maintenance).

## Checks

Full pre-PR gate (three clippy legs, rustdoc trio, `just test`, examples both ways), plus the
integration crates (`cargo build -p ibapi-integration-sync --tests`,
`-p ibapi-integration-async --tests`) since a public field is removed. `just rules-check`
(this file lives in `plans/`).

## Out of scope

The request side passes `&str` (`"Last"`, `"AllLast"`, `"BidAsk"`, `"MidPoint"`) into
`tick_by_tick` / `encode_tick_by_tick`. All `pub(crate)` and fed only by fixed builder methods,
so no user can pass a bad value. Leave it.
