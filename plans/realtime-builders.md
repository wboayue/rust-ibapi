# Realtime API → Builders

Mirror the historical-data builder sweep (issue #615) on the `src/market_data/realtime` public surface. Collapse the 4-method `tick_by_tick_*` split and the borderline `market_depth` into fluent builders, matching the `RealtimeBarsBuilder` shape already established.

**Status:** planned · v3.0 scope.

## Why

`src/market_data/realtime/` exposes 7 public methods on `Client`:

| Method | Args | Status |
|---|---:|---|
| `realtime_bars(&c) -> RealtimeBarsBuilder` | 1 | ✓ already a builder |
| `tick_by_tick_all_last(&c, n, ignore_size)` | 3 | needs builder (4-method split smell) |
| `tick_by_tick_last(&c, n, ignore_size)` | 3 | needs builder |
| `tick_by_tick_bid_ask(&c, n, ignore_size)` | 3 | needs builder |
| `tick_by_tick_midpoint(&c, n, ignore_size)` | 3 | needs builder |
| `market_depth(&c, n_rows, is_smart_depth)` | 3 | needs builder (`bool` → typed enum) |
| `market_depth_exchanges()` | 0 | keep as-is |
| `switch_market_data_type(type)` | 1 | keep as-is |

Plus on the crate-root `Client`:

| Method | Args | Status |
|---|---:|---|
| `market_data(&c) -> MarketDataBuilder` | 1 | ✓ already a builder |

Two rough edges this plan resolves:

1. **4-method `tick_by_tick_*` split** — same anti-pattern as `historical_ticks_*` (which this sweep just collapsed in #613). Near-identical bodies differing only by the wire string (`"AllLast"` / `"Last"` / `"BidAsk"` / `"MidPoint"`) and the return type `T`. `ignore_size: bool` is only meaningful for `BidAsk` per IBKR docs.
2. **`market_depth`'s `is_smart_depth: bool`** — stringly-typed smell; should be a `SmartDepth { Yes, No }` enum (mirroring `IgnoreSize`).

Plus a cross-domain cleanup: **`IgnoreSize` was added at `historical::IgnoreSize` in #613** but the same flag applies to realtime tick-by-tick. Lift to a shared location.

## End state

| Method(s) replaced | New shape |
|---|---|
| 4× `tick_by_tick_*` | `client.tick_by_tick(&c, n)` → `TickByTickBuilder` with terminals `.last()` / `.all_last()` / `.bid_ask(IgnoreSize)` / `.mid_point()` |
| `market_depth(&c, n, bool)` | `client.market_depth(&c, n)` → `MarketDepthBuilder` with `.smart_depth(SmartDepth)` setter and `.subscribe()` terminal |
| `historical::IgnoreSize` | Lifted to `market_data::IgnoreSize`; re-exported at the prelude unchanged |

Direct methods that stay (rule-4 compliant or no ergonomic win):
- `market_depth_exchanges()` (0 args)
- `switch_market_data_type(MarketDataType)` (1 arg)
- `realtime_bars`, `market_data` (already builders)

## Precedent to mirror

Same shape as the just-merged `HistoricalTicksBuilder` (#613) and `HistoricalDataBuilder` (#614):
- `pub struct ..<'a, C>` generic on lifetime + client type
- `pub(crate) fn new(client: &'a C, contract: &'a Contract[, ...]) -> Self` — defaults in `new()`
- `mut self`-returning setters
- Per-feature `impl<'a> ..<'a, sync::Client>` / `impl<'a> ..<'a, async::Client>` blocks holding the terminal(s)
- `#[must_use = "<Name> does nothing until you call .<terminal>()"]` on the struct
- Sibling `_tests.rs` with smoke tests via `create_*_with_responses_and_version`
- For each `pub use Builder` in `realtime/mod.rs`, also added to the prelude

## Builder 1: `TickByTickBuilder`

Constructor: `client.tick_by_tick(&contract, number_of_ticks) -> TickByTickBuilder<'a, Self>`.

Setters: none (no other shared parameters; `ignore_size` is BidAsk-only — lives on the terminal).

Terminals (each produces correctly-typed `Subscription<T>`, wire string set per terminal):

| Terminal | Wire | Returns |
|---|---|---|
| `.last()` | `"Last"` | `Subscription<Trade>` |
| `.all_last()` | `"AllLast"` | `Subscription<Trade>` |
| `.bid_ask(IgnoreSize)` | `"BidAsk"` | `Subscription<BidAsk>` |
| `.mid_point()` | `"MidPoint"` | `Subscription<MidPoint>` |

`ignore_size` is hardcoded `false` for the three non-BidAsk terminals (matches IBKR semantics — they ignore the flag for those types).

Shared `pub(crate) fn tick_by_tick<T: ?>` helper extracted in sync.rs + async.rs. The helper is 5-arg (`client, contract, tick_type, number_of_ticks, ignore_size`) — within rule 4 without needing `#[allow(clippy::too_many_arguments)]`.

**Note:** the four `tick_by_tick_*` methods return different `Subscription<T>` types but the wire dispatch is unified via the `tick_type: &str` argument. The helper is generic over `T` only because the four call sites declare different return types; the actual wire/dispatch logic is identical.

## Builder 2: `MarketDepthBuilder`

Constructor: `client.market_depth(&contract, number_of_rows) -> MarketDepthBuilder<'a, Self>`.

Setters:

| Setter | Type | Default | Purpose |
|---|---|---|---|
| `.smart_depth(SmartDepth)` | enum | `No` | Aggregate across exchanges (Yes) vs single exchange (No) |

Terminal: `.subscribe() -> Result<Subscription<MarketDepths>, Error>`.

**`SmartDepth` enum** (new, in `realtime::SmartDepth`): 2-variant `Yes` / `No`. No `#[non_exhaustive]` (binary domain). Mirrors `IgnoreSize`'s precedent.

Shared `pub(crate) fn market_depth` helper extracted in sync.rs + async.rs (covers version checks for `SMART_DEPTH` and `MKT_DEPTH_PRIM_EXCHANGE` features, encoder call, decoder context with `smart_depth`).

## Cross-domain cleanup: lift `IgnoreSize`

`IgnoreSize` was added at `historical::IgnoreSize` in PR #613. The realtime `TickByTickBuilder::bid_ask(_)` terminal uses the same wire flag with identical semantics. Two options:

1. **Reuse the historical path**: realtime callers `use ibapi::market_data::historical::IgnoreSize;` — feels wrong (cross-submodule reach for a wire primitive).
2. **Lift to `market_data::IgnoreSize`**: both submodules `use ibapi::market_data::IgnoreSize;` — natural shared home.

Going with (2). The lift is non-breaking since `IgnoreSize` was added in #613 (not yet in any released version). The prelude entry stays — only the source path changes.

## Migration plan — 3 PRs

CLAUDE.md rule 23: modernize callers, *then* restrict. Each PR keeps the workspace green by adding the new builder, sweeping all callers (examples, tests, docs), then deleting the old methods.

Branch-then-PR default. Suggested branch names:
- PR 1: `lift-ignore-size-to-market-data`
- PR 2: `realtime-tick-by-tick-builder`
- PR 3: `realtime-market-depth-builder`

Standard per-PR workflow: branch → implement → `cargo fmt` + clippy ×4 configs + rustdoc ×3 configs + tests + integration crate compile (rule 11) → commit → `gh pr create` → merge after CI green → sync main + prune branch.

### PR 1 — Lift `IgnoreSize` to `market_data::IgnoreSize` (precursor, smallest)

- Move `IgnoreSize` definition from `src/market_data/historical/builder/ticks.rs` to a new `src/market_data/types.rs` (or inline in `src/market_data/mod.rs` — find precedent).
- Update `src/market_data/historical/mod.rs::pub use builder::IgnoreSize` → drop (no longer in historical's tree).
- Update `src/market_data/mod.rs` to `pub use types::IgnoreSize;` (or wherever it lands).
- Update prelude: `historical::IgnoreSize` → `market_data::IgnoreSize`.
- Sweep callers: `historical/builder/ticks.rs`, `historical/builder/ticks_tests.rs`, integration tests, examples that import the type.
- Migration guide §entry: 1-line path move.

Standalone precursor PR because PR 2 (`TickByTickBuilder`) depends on the shared home. Could also fold into PR 2 if user prefers a single PR.

### PR 2 — `TickByTickBuilder`

- Add `src/market_data/realtime/builder/mod.rs` (new directory; current `builder.rs` is the realtime-bars file — convert to subdirectory).
- Move existing `realtime/builder.rs` → `realtime/builder/bars.rs` to make room for `tick_by_tick.rs` and other future builders.
- Add `src/market_data/realtime/builder/tick_by_tick.rs` (~250 LoC) + `tick_by_tick_tests.rs`.
- Add `pub(crate) fn tick_by_tick<T>` helper in `realtime/sync/mod.rs` + `async/mod.rs` (deduplicates the 4 method bodies).
- Replace 4 deleted methods with one `Client::tick_by_tick(&contract, n)` entry.
- Sweep callers: `examples/{sync,async}/tick_by_tick_*.rs` (likely 4-6 files), integration tests, unit tests.
- Add `TickByTickBuilder` to the prelude.
- Migration guide §entry: 4-method → 1-builder mapping table.

### PR 3 — `MarketDepthBuilder` + `SmartDepth`

- Add `src/market_data/realtime/builder/market_depth.rs` + `_tests.rs`.
- Add `SmartDepth` enum in the same file (or alongside `IgnoreSize` at `market_data::SmartDepth` for symmetry — TBD during impl).
- Add `pub(crate) fn market_depth` helper in `realtime/sync/mod.rs` + `async/mod.rs` (covers SMART_DEPTH + MKT_DEPTH_PRIM_EXCHANGE version checks).
- Replace `market_depth(&c, n, bool)` with builder entry.
- Sweep callers: `examples/{sync,async}/market_depth.rs`, integration tests, unit tests, README if applicable.
- Add `MarketDepthBuilder` + `SmartDepth` to the prelude.
- Migration guide §entry: `bool` → `SmartDepth` enum migration.

## Reuse (do not rewrite)

- `src/market_data/realtime/builder.rs::RealtimeBarsBuilder` — copy the structural shape verbatim; this is the canonical precedent.
- `src/market_data/historical/builder/ticks.rs::HistoricalTicksBuilder` — 4-terminal pattern with `IgnoreSize` on the BidAsk terminal is the direct precedent for `TickByTickBuilder`.
- `src/market_data/realtime/common/encoders.rs::encode_tick_by_tick` (5-arg signature) — unchanged; builder composes the args.
- `src/market_data/realtime/common/encoders.rs::encode_request_market_depth` — unchanged.
- `src/market_data/historical/builder/data_tests.rs` — smoke-test pattern with `create_*_with_responses_and_version` shared helpers + `assert_stream_invalid_argument_{sync,async}` helpers for `Subscription<T>`'s missing `Debug` impl.

## Verification

Per CLAUDE.md "Quick Commands" + rule 11:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features --features sync -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps  # × 3 feature configs
just test
cargo build -p ibapi-integration-sync  --tests
cargo build -p ibapi-integration-async --tests
```

Per-PR sanity tests (smoke level):
- Stub the client with `MessageBusStub`; build a request via the new builder; assert captured proto bytes round-trip to the expected request.
- Round-trip every setter.
- Negative tests for runtime checks (where they exist).

## Self-review notes (lens findings)

- **Ergonomics**: `IgnoreSize` reuse is the dominant ergonomic win — same flag, same semantics, one type. Lifting to `market_data::` makes both subscriptions share it. `SmartDepth` enum replaces `bool` for the same reason (matches `IgnoreSize` precedent).
- **SRP**: each new builder owns one wire request shape; helpers stay in sync.rs/async.rs per the historical-builder precedent. `tick_by_tick` helper's `tick_type: &str` argument is stringly-typed at the wire layer — the enum-vs-string at the helper is debatable but unchanged from the existing `encode_tick_by_tick`; defer to a follow-up.
- **Composability**: realtime + historical share `IgnoreSize` after lift. Shared test helpers (`create_*_with_responses_and_version`) already in place.
- **Duplication**: 4-method `tick_by_tick_*` is the headline duplicate (4 near-identical bodies). Same precedent the historical sweep just closed (`historical_ticks_*` 3-method → 1-builder).

## Out of scope

- Builders for `market_depth_exchanges` (0 args) / `switch_market_data_type` (1 arg) — no ergonomic win.
- Changes to `MarketDataType`, `BarSize`, `WhatToShow`, or other realtime enums.
- Changes to encoders/decoders.
- `MarketDataBuilder` enhancements (generic-tick constants module — tracked separately at `plans/generic-tick-types.md`).
- Backport to `v2-stable` (v3.0 only).
