# Code-consistency follow-ups

Open items from the CLAUDE.md alignment audit (ran 2026-05-28). Everything else that audit
found has shipped: the inline-test sweep (#657), async `# Examples` (#657/#659), domain module
layout, the `pegged_to_benchmark` builder (#660), the builder-fed
`#[allow(clippy::too_many_arguments)]` justifications, the `wsh_event_data_*` builders (#752),
the `# Examples` backfill (#751 — every `impl Client` method that owes one has one), and the
`market_data` realtime seam opened by #729's `/simplify` (#772 relocated `MarketDataBuilder`
beside its siblings, `migration-4.0.md` §7; #780 narrowed the `sync::market_data` free fn and
unified async terminal routing through free fns, §8).
Re-run the audit before starting new follow-ups; this list dates from 2026-05-28 and only the
items below were re-verified 2026-08-08.

> **On rule numbers.** This file predates the move of `CLAUDE.md`'s numbered rules into
> [`docs/rules/`](../docs/rules/README.md) nodes.
> Audit-time numbers do not map to today's nodes — the `too_many_arguments` exception was
> recorded as "rule 19" but belongs to [param budget](../docs/rules/style/param-budget.md), and
> the audit's "rule 20" was the ratchet/cleanup split. Resolve any number found here against
> `git show <commit-before-2026-05-28>:CLAUDE.md`, never against the current file.

## [Param budget](../docs/rules/style/param-budget.md) — functions with 4+ params

Treat the rule as "4+ args with at least one optional / defaultable field needs a builder";
pure-required signatures don't benefit (receiver `&self` excluded from the budget).

One public site keeps its decision from the audit; the other shipped:

- `contracts::Client::option_chain` — **shipped as a builder** (`option_chain(symbol,
  security_type, contract_id).exchange(..).subscribe()`, `migration-4.0.md` §10). The audit's
  "type `exchange` as `Option<Exchange>`" verdict was too small: a live probe showed the field
  is TWS's `futFopExchange`, which returns an empty chain for any named exchange on a stock
  underlying, and that `contract_id = 0` — passed by two examples — is a hard rejection. The
  builder made the default the only thing a stock caller can express; the probe is the part
  worth repeating before typing a "meaningful default" (see
  [verify wire before typing](../docs/rules/wire/enum-typing.md)).
- `news::Client::historical_news(&self, contract_id, provider_codes, start_time, end_time,
  total_results)` — 5 args, none optional. **Skip the builder.** A `DateRange` newtype over
  `start_time` + `end_time` is the only remedy worth considering, and leaving it alone is
  defensible.

Internal / free-function violations — take one when you are already in the file:

- `require_range<T>(value, min, max, name)` in `src/common/error_helpers.rs` — consider a
  `Range<T>` newtype or a builder.
- `validate_bracket_prices(action, entry, take_profit, stop_loss)` in
  `src/orders/builder/validation.rs`.
- `ContractBuilder::iron_condor(self, long_put_id, short_put_id, short_call_id, long_call_id)`
  in `src/contracts/builders.rs` — consider a struct of four contract ids.
- `pegged_to_stock(action, quantity, delta, stock_reference_price, starting_price)` in
  `src/orders/common/order_builder/mod.rs`.

## Out-of-scope on the audit pass

- [Coverage floor](../docs/rules/testing/coverage-floor.md) — not audited; run `just cover` per PR.
- [Integration crate builds](../docs/rules/workflow/integration-crate-builds.md) — gates run on touch.
- Audited clean: [narrow re-exports](../docs/rules/style/narrow-reexports.md),
  [macros last resort](../docs/rules/style/macros-last-resort.md),
  [no parity wrappers](../docs/rules/parity/no-parity-wrappers.md),
  [restrict after callers](../docs/rules/workflow/restrict-after-callers.md),
  [wire enum typing](../docs/rules/wire/enum-typing.md),
  [floor-ratchet splits](../docs/rules/wire/floor-ratchet-splits.md), and
  [clock seams](../docs/rules/testing/clock-seams.md).
