# Historical Data APIs → Builders

Design plan for migrating the `src/market_data/historical` public surface from
positional-arg methods to fluent builders. Goal: the same simplification that
landed for `realtime_bars` (PR #X) and `market_data` — fewer args at call
sites, sensible defaults, type-safe terminal selection, sync/async parity per
the project precedent.

**Status:** open · designed 2026-05-12 · v3.0 scope.

## Why

After PR #573 (sync/async reconciliation, issue #210) the historical surface
is consistent but five of nine methods exceed the project's "max 3 params —
use a builder for 4+" rule (CLAUDE.md rule 4):

| Method | Arity | Notes |
| --- | --- | --- |
| `historical_data` | 6 | contract, end_date, duration, bar_size, what_to_show, trading_hours |
| `historical_data_streaming` | 6 | contract, duration, bar_size, what_to_show, trading_hours, keep_up_to_date |
| `historical_ticks_bid_ask` | 6 | contract, start, end, number_of_ticks, trading_hours, ignore_size |
| `historical_ticks_mid_point` | 5 | contract, start, end, number_of_ticks, trading_hours |
| `historical_ticks_trade` | 5 | contract, start, end, number_of_ticks, trading_hours |
| `historical_schedules` | 3 | contract, end_date, duration — borderline |
| `historical_schedules_ending_now` | 2 | contract, duration |
| `head_timestamp` | 3 | OK as-is |
| `histogram_data` | 3 | OK as-is |
| `cancel_historical_ticks` | 1 | OK as-is |

The three `historical_ticks_*` variants also share a near-identical signature
that differs only by `WhatToShow` + the `ignore_size` flag (BidAsk-only) — a
classic "split into N free fns instead of typed terminals" smell.

`historical_schedules` + `historical_schedules_ending_now` are independently
simple but conceptually one operation — they exist as a pair because the
magic-`None` API (the pre-PR-#573 async shape) was rejected. A builder with an
explicit `.ending()` method dissolves that concern.

## Precedents in this codebase

1. **`RealtimeBarsBuilder`** (`src/market_data/realtime/builder.rs`) —
   canonical shape: `XxxBuilder<'a, C>` parameterized on the client type, one
   sync `impl` + one async `impl`, terminal `.subscribe()` on each. Defaults
   applied in `new()`. Tests in a sibling `_tests.rs`.
2. **`MarketDataBuilder`** (`src/market_data/builder/market_data_builder.rs`) —
   slightly larger surface with an accumulator setter (`add_generic_tick`).
   Same sync/async dual-impl pattern.
3. **`Client::builder()`** — shipped per v3-api-ergonomics §1 (connect
   variants folded into a builder).

Every new builder here should mirror these three down to:
- `pub(crate) fn new(client: &'a C, contract: &'a Contract)` constructor.
- Per-field setter methods consuming and returning `Self`.
- Terminal methods (`fetch` / `stream` / `subscribe`) on per-feature impl
  blocks, each with its own `# Examples` doc block per CLAUDE.md rule 18.
- Defaults set in `new()` (e.g. `TradingHours::Regular`).
- `#[must_use]` per v3-api-ergonomics §plumbing-and-misc (so a forgotten
  terminal warns at compile time).

## Proposed shape

### 1. `client.historical_data(&contract)` — unified bar fetcher

Replaces both `historical_data` and `historical_data_streaming`.

```rust
// One-shot
let bars = client.historical_data(&contract)
    .duration(7.days())
    .bar_size(BarSize::Hour)
    .what_to_show(WhatToShow::Trades)         // default: Trades
    .trading_hours(TradingHours::Regular)     // default: Regular
    .ending(datetime!(2023-04-15 0:00 UTC))   // optional; absent = now
    .fetch().await?;

// Streaming with keep_up_to_date=true
let subscription = client.historical_data(&contract)
    .duration(1.days())
    .bar_size(BarSize::Min15)
    .what_to_show(WhatToShow::Trades)
    .stream().await?;
```

**Required**: `duration`, `bar_size`. **Defaults**: `WhatToShow::Trades`,
`TradingHours::Regular`. **Terminal selection**: `.fetch()` (one-shot,
returns `HistoricalData`) vs `.stream()` (returns
`Subscription<HistoricalBarUpdate>`, sets `keep_up_to_date=true`).

**Constraint**: `.stream()` rejects builders that called `.ending(date)` —
IBKR requires `end_date=None` for `keepUpToDate=true`. Enforce at terminal as
`Error::InvalidArgument("ending() is incompatible with stream() — IBKR
requires no end_date for streaming updates")`. Same shape as the existing
AdjustedLast / end_date mutual-exclusion check in
`common::validate_historical_data`.

**Why not typestate** to make the constraint compile-time? The single
runtime check matches the project's existing precedent (the AdjustedLast
runtime check), is simpler to read, and pairs with a clear error message.
Typestate would inflate the builder type to 4 phantom states and require
generic juggling in the user's `let` bindings. Skip.

**Why required `duration` / `bar_size`**: every existing caller specifies
both; defaults would be misleading (no obvious "default duration"). Required
positional in `new()` is wrong (mixes positional + fluent), so they become
required-by-runtime-check on terminal: terminal returns an error if either
is unset. Better: take them as constructor args
(`client.historical_data(&contract, 7.days(), BarSize::Hour)`) — keeps the
contract analogy that `client.market_data(&contract)` is already required.

**Decision**: take `duration` and `bar_size` as constructor args. Reduces
runtime errors, keeps the fluent setters for genuinely optional fields. The
constructor is then 3 args (rule-4 compliant).

```rust
client.historical_data(&contract, 7.days(), BarSize::Hour)
    .what_to_show(WhatToShow::Trades)         // optional, defaults to Trades
    .ending(datetime!(2023-04-15 0:00 UTC))   // optional
    .fetch().await?;
```

### 2. `client.historical_ticks(&contract)` — unified tick fetcher

Replaces the three `historical_ticks_{trade,bid_ask,mid_point}` methods.

```rust
let trades = client.historical_ticks(&contract)
    .number_of_ticks(100)
    .start(datetime!(2023-04-15 0:00 UTC))    // .start XOR .end
    .trading_hours(TradingHours::Regular)
    .trade().await?;                          // returns TickSubscription<TickLast>

let mids = client.historical_ticks(&contract)
    .number_of_ticks(100)
    .end(datetime!(2023-04-15 0:00 UTC))
    .mid_point().await?;                      // TickSubscription<TickMidpoint>

let quotes = client.historical_ticks(&contract)
    .number_of_ticks(100)
    .end(datetime!(2023-04-15 0:00 UTC))
    .bid_ask(IgnoreSize::Yes).await?;         // TickSubscription<TickBidAsk>
```

**Terminal selects type**: `.trade()`, `.mid_point()`, `.bid_ask(IgnoreSize)`
each produce the correctly-typed `TickSubscription<T>`. `ignore_size` lives
only on the `.bid_ask()` terminal — it's BidAsk-only on the wire.

**Required**: `number_of_ticks` (no sensible default). **Defaults**:
`TradingHours::Regular`. **Constraint**: at least one of `.start()` /
`.end()` must be set per IBKR ("Either start time or end time is
specified"); runtime check at terminal.

**`IgnoreSize` newtype** instead of `bool` — follows the typed-enum
philosophy from v3-api-ergonomics. Two variants (`Yes`, `No`); a plain bool
is fine if a Yes/No enum is judged ceremonial — verify against precedent.

### 3. `client.historical_schedules(&contract)` — unified schedule fetcher

Replaces `historical_schedules` + `historical_schedules_ending_now`. The
PR #573 split exists to avoid the magic-`None` async API; a fluent
`.ending()` is explicit by name and dissolves the concern.

```rust
let schedule = client.historical_schedules(&contract, 30.days())
    .fetch().await?;                                // ends at current time

let schedule = client.historical_schedules(&contract, 30.days())
    .ending(datetime!(2023-04-15 0:00 UTC))
    .fetch().await?;                                // ends at given date
```

**Constructor**: takes `(contract, duration)` — both genuinely required.
**Optional**: `.ending(end_date)`.

This collapses two methods into one builder. The original split was a
reaction to one bad shape (`Option<OffsetDateTime>` as a positional arg);
naming the optional via `.ending(...)` keeps the explicitness and unifies
the API.

### 4. Keep as direct methods (no builder)

- `head_timestamp(contract, what_to_show, trading_hours)` — 3 args, rule-4
  compliant. Could take `trading_hours` as default → 2 args + `.with_trading_hours()`
  variant, but no real ergonomic win; current shape is clearer.
- `histogram_data(contract, trading_hours, period)` — 3 args. Same logic.
- `cancel_historical_ticks(request_id)` — single id; never a builder target.

## Migration plan

Ship in **3 PRs**. Each PR keeps the workspace green per CLAUDE.md rule 23
(modernize callers first, restrict / replace second).

### PR 1: `HistoricalTicksBuilder`

Smallest scope, three functions collapse to one builder.

- Add `src/market_data/historical/builder/ticks.rs` with `HistoricalTicksBuilder`.
- Add `Client::historical_ticks(&contract) -> HistoricalTicksBuilder` on
  both sync and async.
- Delete `historical_ticks_trade` / `historical_ticks_mid_point` /
  `historical_ticks_bid_ask` from sync.rs and async.rs.
- Sweep callers: examples (`historical_ticks*.rs` sync + async),
  integration tests, unit tests.
- Migration guide §entry: 3-method → 1-builder mapping table.

### PR 2: `HistoricalScheduleBuilder`

- Add `src/market_data/historical/builder/schedule.rs`.
- Add `Client::historical_schedules(&contract, duration) -> HistoricalScheduleBuilder`.
- Delete `historical_schedules` (positional `end_date`) and
  `historical_schedules_ending_now`.
- Sweep callers + migration §entry.

This PR partially undoes the PR #573 split, but lands the better shape — the
split was the correct intermediate step (magic `None` was worse than two
methods; the builder is better than both).

### PR 3: `HistoricalDataBuilder` (fetch + stream terminals)

Largest scope.

- Add `src/market_data/historical/builder/data.rs`.
- Add `Client::historical_data(&contract, duration, bar_size) -> HistoricalDataBuilder`.
- Two terminals: `.fetch()` and `.stream()`.
- Delete `historical_data` + `historical_data_streaming`.
- Sweep callers: `historical_data.rs` examples (sync + async),
  integration tests (`historical_data_*` and `historical_data_streaming`
  cases), unit tests, README, migration guide §entry.

## Open questions

1. **Constructor vs setter for "required" fields.** This plan takes
   `duration`/`bar_size` (data) and `duration` (schedule) and
   `number_of_ticks` (ticks) as constructor args. Alternative: setter +
   runtime check at terminal. Constructor args are more discoverable but
   mix positional + fluent. Project precedent (`market_data(&contract)`,
   `realtime_bars(&contract)`) takes only `contract` in the constructor.
   **Recommendation**: pass `duration`/`bar_size` as constructor args for
   `historical_data` (genuinely never optional, two of them is fine), but
   keep `number_of_ticks` on `historical_ticks` as a setter (since the
   start/end shape already needs a runtime XOR check, one more terminal
   check is small).

2. **`IgnoreSize` enum vs `bool`.** v3-api-ergonomics generally favors
   typed enums over `bool`, but `IgnoreSize::{Yes, No}` looks ceremonial.
   Defer to /simplify on the first draft.

3. **`#[must_use]` on builders.** Already tracked in
   v3-api-ergonomics §plumbing-and-misc; apply to these three at the same
   time.

4. **Async streaming consume-form** for `.stream()` doc-examples — use the
   `filter_data` + `next().await` form per CLAUDE.md rule 24 (a) and the
   precedent set by PR #573 for `historical_data_streaming`.

5. **`compile_fail` doc-tests** for the `.stream()` + `.ending()` mutual
   exclusion. Worth the maintenance? CLAUDE.md rule 22 — only if pinned to
   the specific error code. Likely not worth it; the runtime error message
   is sufficient.

## Out of scope

- Builders for `head_timestamp` / `histogram_data` / `cancel_historical_ticks`
  (rule-4 compliant; no ergonomic win).
- Changes to `WhatToShow`, `BarSize`, `Duration`, `TradingHours` enums.
- Changes to the underlying encoder/decoder.
- Backport to `v2-stable` — v3.0 only per CLAUDE.md Branches policy.
