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

### Rule 8 (rest of the inline-test sweep) — 6 files, ~1,720 lines

Same mechanical pattern as the 16 above. The remaining files are large (200+ lines each); sized for one focused PR.

| File | Inline block lines |
| --- | --- |
| `accounts/common/encoders.rs` | 244 |
| `accounts/types.rs` | 350 |
| `client/builders/async.rs` | 351 |
| `client/builders/sync.rs` | 300 |
| `market_data/historical/mod.rs` | 248 |
| `proto/encoders.rs` | 228 |

**Pattern:**
1. Move body of `#[cfg(test)] mod tests { ... }` to sibling `<stem>_tests.rs` (or `mod_tests.rs` for the one mod.rs case).
2. Lift `use super::*;` to the top of the new file (already there in every block).
3. Replace the inline block with `#[cfg(test)] #[path = "<stem>_tests.rs"] mod tests;`.
4. Run `cargo build --tests --all-features` and `cargo test --features sync` to catch any unresolved imports.

### Rule 19 / Rule 4 — `#[allow(clippy::too_many_arguments)]` on production code (4 sites)

Each is a 4–6-param function that needs a builder per rule 4. Rule 19 cites these as the canary for "the canary is not the fix."

- `src/orders/common/order_builder/mod.rs:752` — `pegged_to_benchmark` (≥7 params)
- `src/market_data/historical/async.rs:345` — `historical_ticks` (pub(crate))
- `src/market_data/historical/sync.rs:325` — `historical_ticks` (pub(crate))
- `src/market_data/historical/common/encoders.rs:47` — `encode_request_historical_data` (pub(crate) encoder)

`historical_ticks` and `encode_request_historical_data` are pub(crate) plumbing called from the `HistoricalTicksBuilder` / `HistoricalDataBuilder`; the builders already exist and these helpers are not on the public API surface. The `#[allow]` is *defensible* here (helper-function exception), but per rule 19 the project still prefers a struct-of-args or named-tuple seam. Lower urgency.

`pegged_to_benchmark` is genuinely public and needs a builder migration similar to `BracketOrderBuilder`.

### Rule 4 — public functions with 4+ params (6 sites)

- `src/accounts/sync/mod.rs:159` — `pnl_single(&self, account, contract_id, model_code)` — 3 args after `&self`, technically right at the limit. (Re-audit: rule says "max 3 params"; `&self` counts. If `&self` is excluded by convention here, this is compliant. Clarify in CLAUDE.md.)
- `src/common/error_helpers.rs:31` — `require_range<T>(value, min, max, name)` — internal helper; consider `Range<T>` newtype or a builder.
- `src/orders/builder/validation.rs:5` — `validate_bracket_prices(action, entry, take_profit, stop_loss)` — internal validation helper.
- `src/contracts/builders.rs:497` — `iron_condor(self, long_put_id, short_put_id, short_call_id, long_call_id)` — 4 leg ids; consider a struct of 4 contract ids.
- `src/orders/common/order_builder/mod.rs:181` — `pegged_to_stock(action, quantity, delta, stock_reference_price, starting_price)` — 5 params; builder.
- `src/orders/common/order_builder/mod.rs:752` — `pegged_to_benchmark(...)` — see Rule 19 entry above.

Best opened as one PR per function so each migration can be reviewed for the right signature shape.

### Rule 18 — async public methods missing `# Examples` (≈30 sites)

The async siblings of well-documented sync methods systematically lack `# Examples` blocks. Found in:

- `orders/async.rs` — 11 of 12 `pub async fn` (sync counterparts all documented)
- `news/async.rs` — 0 of 6
- `scanner/async.rs` — 0 of 2
- `display_groups/async.rs` — 0 of 2
- `wsh/async.rs` — 0 of 3
- `market_data/realtime/async/mod.rs` — 3 methods (`switch_market_data_type`, `market_depth_exchanges`, `realtime_bars`)
- `contracts/async/mod.rs` — `calculate_option_price`, `calculate_implied_volatility`, `cancel_contract_details`, `option_chain`

Pattern: copy the sync method's `# Examples` block, switch to `#[tokio::main]` + `.await`, switch `use ibapi::client::blocking::Client;` to `use ibapi::prelude::*;` (per `feedback_per_method_sync_async_doc_pairing.md`). One PR per domain is the most reviewable shape.

### Rule 2 — 5 domains still use `<domain>/{sync,async}/mod.rs`

The project minimises `mod.rs` files; these five domains haven't been migrated:

- `accounts/{sync,async}/mod.rs`
- `contracts/{sync,async}/mod.rs`
- `market_data/realtime/{sync,async}/mod.rs`
- `orders/{sync,async}/mod.rs`
- `transport/{sync,async}/mod.rs`

Each migration is a `git mv` plus updating the parent `mod.rs` from `mod sync;` (resolves to `sync/mod.rs`) to `#[path = "sync.rs"] mod sync;` (or just `mod sync;` if the directory is removed entirely). One PR per domain to keep the diff focused; do not try to flatten all 5 in one PR.

### Rule 12 sub-rule — `Subscription` doesn't use the `sync_impl`/`async_impl` naming

Pre-existing. `Subscription` lives in flat `subscriptions/{sync,async}.rs`. The audit clarified CLAUDE.md rule 12 to note this divergence (`Subscription` predates the convention). Migrating `Subscription` to `sync_impl`/`async_impl` is a larger reshape and not required for alignment — the rule now documents the divergence rather than demanding conformity.

## Out-of-scope on the audit pass

- Rule 6 (90% coverage target) — not audited; `just cover` should be run on every PR per the rule.
- Rule 11 (integration crate builds) — not audited; gates run on touch.
- Rules 13, 14, 16, 20, 23, 25, 26 — audited clean (no current violations).
