# Code-consistency follow-ups

Remaining open items from the CLAUDE.md alignment audit (ran 2026-05-28). All other
tracks from that audit have shipped — Rule 8 inline-test sweep (PR #657), Rule 18 async
`# Examples` (PRs #657/#659), Rule 2 flat layout, `pegged_to_benchmark` builder (PR #660),
and the Rule 19 `#[allow(too_many_arguments)]` justification comments. Re-run the audit
before starting new follow-ups to catch fresh drift.

## Rule 4 — public functions with 4+ params

Treat the rule as "4+ args with at least one optional / defaultable field needs a builder";
pure-required signatures don't benefit (receiver `&self` excluded from the budget).

Internal / free-function violations:

- `src/common/error_helpers.rs:31` — `require_range<T>(value, min, max, name)` — internal helper; consider `Range<T>` newtype or a builder.
- `src/orders/builder/validation.rs:5` — `validate_bracket_prices(action, entry, take_profit, stop_loss)` — internal validation helper.
- `src/contracts/builders.rs:550` — `iron_condor(self, long_put_id, short_put_id, short_call_id, long_call_id)` — 4 leg ids; consider a struct of 4 contract ids.
- `src/orders/common/order_builder/mod.rs:182` — `pegged_to_stock(action, quantity, delta, stock_reference_price, starting_price)` — 5 params; builder.

Client-method violations exposed by the receiver clarification (each appears in `<domain>/sync.rs` + `<domain>/async.rs`):

- **`wsh::Client::wsh_event_data_by_contract(&self, contract_id, start_date, end_date, limit, auto_fill)`** — 1 required + 4 `Option`. Doc example calls `(id, None, None, None, None)`; canonical happy-path is "just events for this contract id." **Strong builder candidate** (the clear win of the three): `WshEventDataBuilder` on `Client::wsh_event_data_by_contract(id) -> WshEventDataBuilder` with `.date_range(start, end)`, `.limit(n)`, `.auto_fill(spec)` setters. Clean standalone PR (sync + async together).
- **`contracts::Client::option_chain(&self, symbol, exchange, security_type, contract_id)`** — 4 args all required, but `exchange` documents `""` as a meaningful default. Marginal. **Defer / decide case-by-case;** if revisiting, consider typing `exchange` as `Option<Exchange>` and dropping the magic empty string.
- **`news::Client::historical_news(&self, contract_id, provider_codes, start_time, end_time, total_results)`** — 5 args all required, no defaults. **Skip the builder.** Better remedy if any: group `start_time` + `end_time` into a `DateRange` type. Leaving as-is is also defensible.

## Out-of-scope on the audit pass

- Rule 6 (90% coverage target) — not audited; run `just cover` per PR.
- Rule 11 (integration crate builds) — gates run on touch.
- Rules 13, 14, 16, 20, 23, 25, 26 — audited clean.
