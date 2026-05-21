# Historical Data APIs → Builders

Migrate the `src/market_data/historical` public surface from positional-arg methods to fluent builders, mirroring the `realtime_bars` / `market_data` shape that already ships.

**Status:** in progress · refined 2026-05-21 · v3.0 scope.

## Why

`src/market_data/historical/` currently exposes 9 public `Client` methods. Five exceed CLAUDE.md rule 4 (max 3 params; builder for 4+):

| Method | Args | Notes |
|---|---:|---|
| `historical_data` | 6 | contract, end_date, duration, bar_size, what_to_show, trading_hours |
| `historical_data_streaming` | 6 | contract, duration, bar_size, what_to_show, trading_hours, keep_up_to_date |
| `historical_ticks_bid_ask` | 6 | contract, start, end, number_of_ticks, trading_hours, ignore_size |
| `historical_ticks_mid_point` | 5 | contract, start, end, number_of_ticks, trading_hours |
| `historical_ticks_trade` | 5 | contract, start, end, number_of_ticks, trading_hours |
| `historical_schedules` | 3 | contract, end_date, duration — borderline |
| `historical_schedules_ending_now` | 2 | contract, duration — pair to above |
| `head_timestamp` | 3 | OK as-is |
| `histogram_data` | 3 | OK as-is |

Three rough edges this plan resolves:

1. **High-arity positional calls** — error-prone, hard to read, hard to discover defaults.
2. **3-method `historical_ticks_*` split** — near-identical signatures differing only by tick type + an `ignore_size` flag (BidAsk-only). Classic "split into N free fns instead of typed terminals" smell.
3. **`historical_schedules` + `historical_schedules_ending_now` pair** — exist together to dodge magic-`None` API; can collapse to one builder with explicit `.ending()`.

Plus the convenience setter `.between(start, end)` for `historical_data` — atomic date-range alternative to the IBKR-native `.duration() + .ending()` pair. Reads naturally and eliminates the half-set state problem of separate `.from()`/`.to()` setters.

**Out of scope**: string-symbol entry (`historical_data("AAPL")`) — confirmed not asked; constructor takes `&Contract` per existing precedent.

## End state

Three new builders + the three already-fine direct methods unchanged:

- `client.historical_data(&contract, bar_size)` → `HistoricalDataBuilder`, terminals `.fetch()` / `.stream()`
- `client.historical_ticks(&contract, number_of_ticks)` → `HistoricalTicksBuilder`, terminals `.trade()` / `.mid_point()` / `.bid_ask(IgnoreSize)`
- `client.historical_schedules(&contract, duration)` → `HistoricalScheduleBuilder`, terminal `.fetch()`
- `client.head_timestamp(...)` / `client.histogram_data(...)` / `client.cancel_historical_ticks(...)` — keep as direct methods (rule-4 compliant)

Five existing methods are deleted (`historical_data`, `historical_data_streaming`, `historical_ticks_*`×3, `historical_schedules`, `historical_schedules_ending_now`).

## Precedent to mirror

`src/market_data/realtime/builder.rs::RealtimeBarsBuilder`:
- `pub struct ..<'a, C>` generic on lifetime + client type
- `pub(crate) fn new(client: &'a C, contract: &'a Contract) -> Self` — defaults in `new()`
- `mut self`-returning setters
- Per-feature `impl<'a> ..<'a, sync::Client>` and `impl<'a> ..<'a, async::Client>` blocks holding the terminal(s), each with its own `# Examples` doc block (CLAUDE.md rule 18)
- `#[must_use = "<Name> does nothing until you call .<terminal>()"]` on the struct
- Sibling `_tests.rs` (rule 8); smoke tests via `MessageBusStub` asserting protobuf round-trip

## Builder 1: `HistoricalDataBuilder`

Constructor: `client.historical_data(&contract, bar_size) -> HistoricalDataBuilder<'a, Self>`. Bar size in the constructor since both are genuinely-always-required.

Setters (all return `Self`):

| Setter | Type | Default | Purpose |
|---|---|---|---|
| `.what_to_show(WhatToShow)` | enum | `Trades` | Data type |
| `.trading_hours(TradingHours)` | enum | `Regular` | Session filter |
| `.duration(Duration)` | `Duration` | — | IBKR-native: amount of data going back from `end_date` (or now) |
| `.ending(OffsetDateTime)` | `OffsetDateTime` | now | IBKR-native: anchor end date; pairs with `.duration()` |
| `.between(OffsetDateTime, OffsetDateTime)` | `(start, end)` | — | Convenience: computes `duration = end - start`, sets `end_date = end` |

**Date-spec semantics** (one required, mutually exclusive at terminal):
- **IBKR-native**: `.duration(D)` (defaults `end_date = None` → now) with optional `.ending(end)` to anchor a specific end
- **Range**: `.between(start, end)` (convenience; computes duration internally)

`.last(D)` was considered as sugar but rejected — it would be a redundant spelling of `.duration(D)` with identical wire output (the "one obvious way to spell each thing" rule from v3-ergonomics §7).

Internal state: `Option<DateSpec>` enum with `IbkrNative { duration, ending }` and `Range { start, end }` variants. First setter wins; conflicting setter errors at terminal with `Error::InvalidArgument("mix .between(...) with .duration()/.ending()")`.

Terminals:
- `.fetch() -> Result<HistoricalData, Error>` — one-shot bar fetch
- `.stream() -> Result<Subscription<HistoricalBarUpdate>, Error>` — `keep_up_to_date = true`

**Two-layer validation at each terminal**:
1. **Builder-internal consistency** (builder's responsibility): date-spec was set; date-spec wasn't mixed; `.stream()` wasn't called after `.ending(end)` or `.between(_, end)` (IBKR requires `end_date = None` for `keep_up_to_date`).
2. **Wire-format validation** (existing `common::validate_historical_data`'s responsibility): the existing AdjustedLast / end_date mutual-exclusion check, unchanged.

Two layers because the responsibilities differ: builder enforces *its own state machine*, the wire validator enforces *IBKR encoding rules*.

**Why no typestate**: `StockBuilder` uses typestate to enforce required fields, but `.duration` vs `.between` is content (two ways to spell the same thing), not "did the user set it yet?" — typestate would need 3 phantom states + generic juggling in callers' `let` bindings. Runtime check matches `RealtimeBarsBuilder`'s precedent.

## Builder 2: `HistoricalTicksBuilder`

Constructor: `client.historical_ticks(&contract, number_of_ticks) -> HistoricalTicksBuilder<'a, Self>`.

Setters:

| Setter | Type | Default | Purpose |
|---|---|---|---|
| `.starting(OffsetDateTime)` | `OffsetDateTime` | — | Anchor at start (fetch forward) |
| `.ending(OffsetDateTime)` | `OffsetDateTime` | — | Anchor at end (fetch backward) |
| `.trading_hours(TradingHours)` | enum | `Regular` | Session filter |

**Date semantics**: IBKR ticks API takes both `startDateTime` and `endDateTime` on the wire (empty `Option<OffsetDateTime>` encodes as empty string). At-least-one runtime check at terminal:
- Both unset → `Error::InvalidArgument("historical_ticks: must set .starting() or .ending()")`
- One or both set → allowed (existing code passes both fields)

No `.between()` here — date range isn't the API shape (it's anchor + count).

Terminals (each produces correctly-typed `TickSubscription<T>`):
- `.trade() -> Result<TickSubscription<TickLast>, Error>`
- `.mid_point() -> Result<TickSubscription<TickMidpoint>, Error>`
- `.bid_ask(IgnoreSize) -> Result<TickSubscription<TickBidAsk>, Error>` — `ignore_size` lives only on this terminal since it's BidAsk-only on the wire

**`IgnoreSize`**: new 2-variant enum (`Yes` / `No`), not `bool`. Matches v3.0 typed-enum philosophy. `#[non_exhaustive]` omitted (binary domain per issue #608's rule on `#[non_exhaustive]`).

## Builder 3: `HistoricalScheduleBuilder`

Replaces the `historical_schedules` + `historical_schedules_ending_now` pair. The PR #573 split was the documented intermediate (magic-`None` API was worse than two methods); the builder dissolves both into one explicit shape.

Constructor: `client.historical_schedules(&contract, duration) -> HistoricalScheduleBuilder<'a, Self>`.

Setters:

| Setter | Type | Default | Purpose |
|---|---|---|---|
| `.ending(OffsetDateTime)` | `OffsetDateTime` | now | Anchor end date |

Terminal: `.fetch() -> Result<Schedule, Error>`.

## Migration plan — 3 PRs

CLAUDE.md rule 23: modernize callers, *then* restrict. Each PR keeps the workspace green by adding the new builder, sweeping all callers (examples, tests, docs), then deleting the old methods.

Branch-then-PR default. Branch names:
- PR 1: `historical-schedule-builder`
- PR 2: `historical-ticks-builder`
- PR 3: `historical-data-builder`

Standard per-PR workflow: branch → implement → `cargo fmt` + clippy ×4 configs + rustdoc ×3 configs + tests + integration crate compile (rule 11) → commit → `gh pr create` → merge after CI green → sync main + prune branch.

### PR 1 — `HistoricalScheduleBuilder` (smallest)

- Add `src/market_data/historical/builder/mod.rs` + `builder/schedule.rs` + `builder/schedule_tests.rs`
- Add `Client::historical_schedules(&contract, duration)` builder entry on both sync + async
- Sweep callers: `examples/{sync,async}/historical_schedules*.rs`, integration tests
- Delete `historical_schedules` + `historical_schedules_ending_now` from `sync.rs` / `async.rs`
- **Delete the private `historical_schedule` helper** at `src/market_data/historical/sync.rs:522` and `async.rs:586` — it exists solely to back the deleted public methods; absorb its logic into the builder's `.fetch()` terminal so no dead duplicate remains
- Migration guide §entry: 2-method → 1-builder mapping

This PR partially undoes the PR #573 split (which existed to avoid magic-`None`); the builder is the documented next step in the 3-step evolution per the `feedback_magic_none_split_to_builder` memory.

### PR 2 — `HistoricalTicksBuilder`

- Add `builder/ticks.rs` + `ticks_tests.rs` + the `IgnoreSize` enum
- Add `Client::historical_ticks(&contract, number_of_ticks)` entry on sync + async
- Sweep callers: `examples/{sync,async}/historical_ticks_{trade,mid_point,bid_ask}.rs`, integration tests
- Delete `historical_ticks_trade` / `historical_ticks_mid_point` / `historical_ticks_bid_ask` from `sync.rs` / `async.rs`
- Add `IgnoreSize` to the prelude
- Migration guide §entry: 3-method → 1-builder mapping

### PR 3 — `HistoricalDataBuilder` (largest)

- Add `builder/data.rs` + `data_tests.rs` + the private `DateSpec` enum
- Add `Client::historical_data(&contract, bar_size)` entry on sync + async
- Two terminals (`.fetch()` + `.stream()`) with the stream/ending exclusion runtime check
- Sweep callers: `examples/{sync,async}/historical_data*.rs`, `examples/{sync,async}/breakout.rs`, integration tests, README snippets, `docs/api-patterns.md`
- Delete `historical_data` + `historical_data_streaming`
- Migration guide §entry: 2-method → 1-builder + the date-spec tutorial (`.duration`+`.ending` vs `.between`)

## Reuse (do not rewrite)

- `src/market_data/historical/mod.rs::Duration` (lines 207-247) + `ToDuration` trait (lines 298-332) — `.duration(7.days())` syntax for free
- `src/market_data/historical/common/encoders.rs::encode_request_historical_data` (line 48) + `encode_request_historical_ticks` (line 83) — unchanged; builders compose the args
- `src/market_data/historical/common/validate_historical_data` — call from terminals for the existing AdjustedLast/end_date mutual-exclusion (don't extend with builder-state concerns)
- `src/market_data/realtime/builder.rs` — copy the structural shape verbatim
- `src/market_data/builder/market_data_builder/tests.rs` — copy the smoke-test shape (`MessageBusStub` + `assert_proto_msg_id` + `assert_request`)

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
- Stub the client with `MessageBusStub`; build a request via the new builder; assert the captured proto bytes round-trip to the expected protobuf message + msg_id
- Round-trip every setter
- Negative test for each runtime check (`Error::InvalidArgument` shapes documented above)

## Self-review notes (lens findings)

- **Ergonomics**: dropped `.last(D)` sugar (duplicate of `.duration(D)`; violates "one obvious way" — v3-ergonomics §7).
- **SRP**: builders have single coherent responsibilities. Multiple terminals are typed-output selection, not separate responsibilities. Validation is two clearly-labelled layers (builder-state vs wire-format).
- **Composability**: shared `validate_date_range(start, end)` helper considered, rejected at ≤2 callsites. Per-feature `impl` blocks duplicated across builders, but extracting a shared trait would only save boilerplate without adding behavior (`RealtimeBarsBuilder` / `MarketDataBuilder` precedent).
- **Duplication**: overlapping setters (`what_to_show`, `trading_hours`) repeat across builders by precedent. Per-builder test fixtures inline since each tests a distinct encoder. Stale `historical_schedule` private helpers explicitly flagged for deletion in PR 1.

## Out of scope

- Builders for `head_timestamp` / `histogram_data` / `cancel_historical_ticks` (rule-4 compliant)
- Changes to `WhatToShow`, `BarSize`, `Duration`, `TradingHours` enums
- Changes to encoders/decoders
- String-symbol entry on builders
- Backport to `v2-stable` (v3.0 only)
