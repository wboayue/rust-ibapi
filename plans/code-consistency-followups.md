# Code-consistency follow-ups

Remaining open items from the CLAUDE.md alignment audit (ran 2026-05-28). All other
tracks from that audit have shipped — the [sibling test files](../docs/rules/testing/sibling-test-files.md)
inline-test sweep (PR #657), rule 18 async `# Examples` (PRs #657/#659),
[domain module layout](../docs/rules/style/domain-module-layout.md) (audit-time rule 2),
`pegged_to_benchmark` builder (PR #660), and the builder-fed
`#[allow(clippy::too_many_arguments)]` justification comments. Re-run the audit before
starting new follow-ups to catch fresh drift.

> **On rule numbers.** This file predates the [knowledge-graph migration](claude-md-knowledge-graph.md);
> migrated rules are cited by node path below, and only rules still inline in `CLAUDE.md` keep
> a number. Two audit-time numbers did not mean what they look like today: the
> `too_many_arguments` track above was recorded as "rule 19", but rule 19 was then the proto
> fixture sweep — that exception belongs to
> [param budget](../docs/rules/style/param-budget.md). And the audit's "rule 20"
> was the ratchet/cleanup split, not proto-only decoding. Resolve any number found here
> against `git show <commit-before-2026-05-28>:CLAUDE.md`, never against the current file.

## [Param budget](../docs/rules/style/param-budget.md) (audit-time rule 4) — functions with 4+ params

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

## [Public API examples](../docs/rules/docs/public-api-examples.md) — `impl Client` methods with no `# Examples`

**Closed in #751.** Every `impl Client` method that owes an example has one.

Re-counted 2026-08-08 with a script that walks each `impl Client` block and reads each method's
*own* doc comment (`scratchpad/examples_audit.py` in that PR's write-up; the rule is one line of
`re.match(r'\s{4}pub (?:async )?fn')` inside a brace-tracked block):

| | 2026-08-07 | 2026-08-08, before | after |
|---|---|---|---|
| `impl Client` methods | 152 | 154 | 154 |
| carrying `# Examples` | 132 | 135 | 143 |
| missing, exempt accessors | 11 | 11 | 11 |
| missing, genuine | 9 | 8 | **0** |

**The list was right and the denominator was stale**, which is the usual split — the eight
methods named here were exactly the eight still missing a year-tick later, but "132 of 152" was
already wrong when written down, because the method count moves with every PR that adds an API.
A count in prose dates from the moment it was typed; only the command survives.

Written in #751: `server_time_millis` (sync + async), `family_codes` (sync), `market_rule`,
`cancel_contract_details`, `cancel_historical_ticks` (sync + async), `exercise_options` (sync).
The three cancel examples say where the `request_id` comes from, which was the reason they had
no example: it is not a value the method itself hands you.

The remaining eleven sites are the six accessors the rule exempts — `client_id`,
`next_request_id`, `next_order_id`, `connection_time`, `time_zone`, and async `server_version`.
Leave them, with one caveat: `next_request_id` / `next_order_id` allocate from an atomic rather
than reading a field, so they are exempt by size, not by category. If the manual-request-id
workflow ever needs documenting, they are where it goes.

`check_server_version` was on this list until it turned out to be `pub` on async and
`pub(crate)` on sync — a visibility asymmetry, not a doc gap. Narrowed to `pub(crate)` on both
in the same PR.

## `market_data` realtime seam — opened by #729's `/simplify`

Both are restructuring, deferred out of #729 rather than landed in a cleanup pass.

- **`MarketDataBuilder` sits a directory above its three siblings.** It lives in
  `src/market_data/builder/` — a three-line module whose sole content is that one type — while
  `RealtimeBarsBuilder`, `MarketDepthBuilder`, and `TickByTickBuilder` live in
  `src/market_data/realtime/builder/`. After #729 moved the entry point into
  `market_data/realtime/{sync,async}.rs`, the new `impl Client` blocks have to reach *upward*
  out of `realtime/` for it, on lines adjacent to siblings that resolve via `super::`. Its only
  consumers in the tree are realtime. Fix: move it to `realtime/builder/market_data.rs` and add
  it to that module's `pub use`. Breaking — `crate::market_data::builder::MarketDataBuilder` is
  a public path, referenced from `realtime/generic_tick.rs` and doc links in
  `subscriptions/{sync,async}.rs` — so it needs its own PR with a `migration-3.0.md` note.
- **`market_data::realtime::sync::market_data` is `pub` where its siblings are `pub(crate)`.**
  The free fn at `realtime/sync.rs` is public; `realtime_bars`, `market_depth`, and
  `tick_by_tick` beside it are `pub(crate)`. Under `#[cfg(all(feature = "sync", not(feature =
  "async")))] pub use sync::*` that makes it reachable as
  `ibapi::market_data::realtime::market_data` in sync-only builds. Async has no public
  counterpart at all — `subscribe_market_data` is `pub(crate)` on `impl Client`. Pre-existing,
  but co-locating the builder entry point in #729 turned it into a same-file inconsistency.
  Narrowing is a public-API change: audit callers first per
  [restrict after callers](../docs/rules/workflow/restrict-after-callers.md), and it needs a
  `CHANGELOG.md` entry.

## Out-of-scope on the audit pass

- [Coverage floor](../docs/rules/testing/coverage-floor.md) (90% target, audit-time rule 6) — not audited; run `just cover` per PR.
- [Integration crate builds](../docs/rules/workflow/integration-crate-builds.md) (audit-time
  rule 11) — gates run on touch.
- Audited clean: [narrow re-exports](../docs/rules/style/narrow-reexports.md) (audit-time
  rule 14), [macros last resort](../docs/rules/style/macros-last-resort.md) (audit-time
  rule 25),
  [no parity wrappers](../docs/rules/parity/no-parity-wrappers.md) (audit-time rule 13),
  [restrict after callers](../docs/rules/workflow/restrict-after-callers.md) (audit-time
  rule 23), plus
  [wire enum typing](../docs/rules/wire/enum-typing.md) (audit-time rule 16),
  [floor-ratchet splits](../docs/rules/wire/floor-ratchet-splits.md) (audit-time rule 20), and
  [clock seams](../docs/rules/testing/clock-seams.md) (audit-time rule 26).
