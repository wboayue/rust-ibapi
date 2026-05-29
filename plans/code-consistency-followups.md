# Code-consistency follow-ups

Catalogue of CLAUDE.md alignment work that landed partially on the `code-consistency` branch (PR forthcoming) and the remaining items deferred to follow-up PRs. The audit that produced this list ran 2026-05-28; re-run it before starting any follow-up to catch new drift.

## Already landed on `code-consistency`

- **Rule 27** — `Client::matching_symbols` sync side now returns `Result<Vec<ContractDescription>, Error>` (was `impl Iterator`), matching async. Migration §32 added.
- **Rule 17** — `ResponseMessage::peek_int` adds a `raw_bytes()`-first guard returning `Err(UnexpectedResponse)` on proto-framed input. Unit test exercises the guard.
- **Rule 8 (partial)** — 16 of 22 inline `#[cfg(test)] mod tests { ... }` blocks extracted to sibling `_tests.rs` files. The 7 smallest landed first (≤30 lines, to establish the pattern); the 9 medium files (47–194 lines) followed in a second commit. The remaining 6 large files (>200 lines) are deferred to a follow-up PR.
  - Small batch (commit 668b079):
    - `market_data/realtime/mod.rs` → `mod_tests.rs`
    - `transport/common.rs` → `common_tests.rs`
    - `contracts/common/stream_decoders.rs` → `stream_decoders_tests.rs`
    - `display_groups/common/encoders.rs` → `encoders_tests.rs`
    - `wsh/common/stream_decoders.rs` → `stream_decoders_tests.rs`
    - `market_data/historical/common/encoders.rs` → `encoders_tests.rs`
    - `scanner/common/stream_decoders.rs` → `stream_decoders_tests.rs`
  - Medium batch:
    - `orders/builder/condition_helpers.rs` → `condition_helpers_tests.rs`
    - `trace/sync.rs` → `sync_tests.rs`
    - `trace/async.rs` → `async_tests.rs`
    - `client/id_generator.rs` → `id_generator_tests.rs`
    - `wsh/common/decoders.rs` → `decoders_tests.rs`
    - `orders/conditions.rs` → `conditions_tests.rs`
    - `transport/recorder.rs` → `recorder_tests.rs`
    - `common/error_helpers.rs` → `error_helpers_tests.rs`
    - `common/retry.rs` → `retry_tests.rs` (preserves the inner `#[cfg(feature = "sync")] mod sync_tests` / `#[cfg(feature = "async")] mod async_tests` structure)

## Deferred — separate follow-up PRs

### Rule 8 (rest of the inline-test sweep) — **DONE in PR #657**

All 6 remaining large files were swept in PR #657 via the sed recipe (see `feedback_sed_inline_test_extraction.md`). Total: 22/22 files migrated.

### Rule 19 / Rule 4 — `#[allow(clippy::too_many_arguments)]` on production code

**On `code-consistency`:** justification comments added to all 4 pub(crate)
helper sites + the public `pegged_to_benchmark`, marking the helpers as
builder-fed and `pegged_to_benchmark` as a tracked open issue.

- `src/market_data/historical/sync.rs:325` — `historical_ticks` (pub(crate))
- `src/market_data/historical/async.rs:345` — `historical_ticks` (pub(crate))
- `src/market_data/historical/common/encoders.rs:47` —
  `encode_request_historical_data` (pub(crate) encoder)
- `src/market_data/historical/common/encoders.rs:82` —
  `encode_request_historical_ticks` (pub(crate) encoder)
- `src/orders/common/order_builder/mod.rs:752` — `pegged_to_benchmark`
  (public, 11 params; needs builder)

The 4 `pub(crate)` helpers are internal plumbing called from the
`HistoricalDataBuilder` / `HistoricalTicksBuilder` finalisers; the public
API is already a builder, so flat args at the wire seam are the documented
exception (rule 19 canary acceptable for builder-fed helpers).

**`pegged_to_benchmark` builder migration — DONE** in `pegged-to-benchmark-builder`
branch: 11-param free function removed and replaced by `PeggedToBenchmark`
fluent builder (5 setters, `reference_contract` required and validated via
`ValidationError::MissingRequiredField`). Migration §33 added; unit tests
migrated + missing-required-field assertion added; sync + async integration
tests added (place + cancel against AAPL reference contract).

### Rule 4 — public functions with 4+ params

**Receiver exclusion clarified** in CLAUDE.md rule 4 + `docs/code-style.md` — `&self` does not count toward the 3-param budget. `pnl_single` (3 args after `&self`) is compliant under the clarified rule and dropped from this list. The clarification surfaced 3 new violations (each sync + async); they replace `pnl_single` below.

Internal / free-function violations:

- `src/common/error_helpers.rs:31` — `require_range<T>(value, min, max, name)` — internal helper; consider `Range<T>` newtype or a builder.
- `src/orders/builder/validation.rs:5` — `validate_bracket_prices(action, entry, take_profit, stop_loss)` — internal validation helper.
- `src/contracts/builders.rs:497` — `iron_condor(self, long_put_id, short_put_id, short_call_id, long_call_id)` — 4 leg ids; consider a struct of 4 contract ids.
- `src/orders/common/order_builder/mod.rs:181` — `pegged_to_stock(action, quantity, delta, stock_reference_price, starting_price)` — 5 params; builder.
- ~~`src/orders/common/order_builder/mod.rs:752` — `pegged_to_benchmark(...)`~~ — DONE in PR #660; see Rule 19 entry above.

Client-method violations exposed by the receiver clarification (each appears in `<domain>/sync.rs` + `<domain>/async.rs`). Treat the rule as "4+ args with at least one optional / defaultable field needs a builder"; pure-required signatures don't benefit:

- **`wsh::Client::wsh_event_data_by_contract(&self, contract_id, start_date, end_date, limit, auto_fill)`** — 1 required + 4 `Option`. Doc example currently calls `(id, None, None, None, None)` — the canonical happy-path is "I just want events for this contract id." **Strong builder candidate** (the only one of the three that's clearly worth refactoring): `WshEventDataBuilder` on `Client::wsh_event_data_by_contract(id) -> WshEventDataBuilder` with `.date_range(start, end)`, `.limit(n)`, `.auto_fill(spec)` setters.
- **`contracts::Client::option_chain(&self, symbol, exchange, security_type, contract_id)`** — 4 args all required, but `exchange` documents `""` as a meaningful default ("all exchanges"). Marginal — the builder would only let callers skip `exchange`, saving one `""` literal per callsite. **Defer / decide case-by-case;** if revisiting, consider also typing `exchange` as `Option<Exchange>` and dropping the magic empty string.
- **`news::Client::historical_news(&self, contract_id, provider_codes, start_time, end_time, total_results)`** — 5 args all required, no defaults. Builder would only add ceremony. **Skip the builder.** Better remedy if any: group `start_time` + `end_time` into a `DateRange` type (drops to 4 args, still required, but the type carries intent). Leaving as-is is also defensible.

The first is a clean PR; the other two are documentation work (note the rule exception for all-required signatures). Best opened as one PR per function (sync + async migrated together).

### Rule 18 — async public methods missing `# Examples` — **DONE**

Closed across two PRs:

- PR #657 — `orders/async.rs` (11/12 → 12/12), `news/async.rs` (0/6 → 6/6), `scanner/async.rs` (0/2 → 2/2), `wsh/async.rs` (0/3 → 3/3), `contracts/async.rs` (4 methods).
- PR #659 — `display_groups::{sync,async}::update` (sync gap was pre-existing; mirrored alongside), `market_data::realtime::async::{switch_market_data_type, realtime_bars, market_depth_exchanges}`.

Pattern established by `feedback_per_method_sync_async_doc_pairing.md`: async examples mirror their sync sibling, switch to `#[tokio::main]` + `.await`, and use `ibapi::prelude::*` (noting that `WhatToShow` is renamed to `RealtimeWhatToShow` / `HistoricalWhatToShow` in the prelude).

Broader Rule 18 sweep across infrastructure (`subscriptions/`, `transport/`, `connection/`, `client/builders/`, `client/{sync,async}.rs`) is out of scope for this audit thread — those files are dominated by `pub(crate)` items.

### Rule 2 — flat `<domain>/{sync,async}.rs` layout

**Complete on `code-consistency`.** All 5 nested-domain layouts migrated:

- `accounts/{sync,async}/{mod,tests}.rs` → `accounts/{sync,async}{,_tests}.rs`
- `contracts/{sync,async}/{mod,tests}.rs` → `contracts/{sync,async}{,_tests}.rs`
- `orders/{sync,async}/{mod,tests}.rs` → `orders/{sync,async}{,_tests}.rs`
- `market_data/realtime/{sync,async}/{mod,tests}.rs` →
  `market_data/realtime/{sync,async}{,_tests}.rs`
- `transport/{sync/mod.rs,sync/tests.rs}` → `transport/{sync.rs,sync_tests.rs}`
  (helpers `memory.rs`, `memory_tests.rs`, `test_listener.rs` stay nested in
  `transport/sync/`, resolved via Rust's normal lookup from `sync.rs`)
- `transport/{async_io.rs,async_memory.rs,async_memory_tests.rs,async_test_listener.rs}`
  → `transport/async/{io,memory,memory_tests,test_listener}.rs` (symmetric
  with sync; helpers nested, main `async.rs` and `async_tests.rs` flat)

The pattern across all 5 domains:
- Main entry-point file flat: `<domain>/sync.rs`, `<domain>/async.rs`
- Main tests flat sibling per rule 8: `<domain>/sync_tests.rs`,
  `<domain>/async_tests.rs`
- Helper submodules allowed to stay nested in `<domain>/<side>/` for
  multi-file domains (only `transport` applies)

### Rule 12 sub-rule — `Subscription` doesn't use the `sync_impl`/`async_impl` naming

Pre-existing. `Subscription` lives in flat `subscriptions/{sync,async}.rs`. The audit clarified CLAUDE.md rule 12 to note this divergence (`Subscription` predates the convention). Migrating `Subscription` to `sync_impl`/`async_impl` is a larger reshape and not required for alignment — the rule now documents the divergence rather than demanding conformity.

## Out-of-scope on the audit pass

- Rule 6 (90% coverage target) — not audited; `just cover` should be run on every PR per the rule.
- Rule 11 (integration crate builds) — not audited; gates run on touch.
- Rules 13, 14, 16, 20, 23, 25, 26 — audited clean (no current violations).
