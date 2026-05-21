# Typed-Status Sweep Tracker

**End goal:** finish converting `String` fields with enumerated TWS wire vocabularies on public types into strict typed enums, following the PR #518 / `OrderStatusKind` precedent (CLAUDE.md rule 16). After this sweep, every "stringly-typed" public field carrying an enumerable wire value should be a typed enum that round-trips via `Display`/`FromStr`.

**Parent:** [v3-api-ergonomics.md §2 "Continue the typed-status sweep"](v3-api-ergonomics.md).

## Status today

- `OrderStatus.status: OrderStatusKind` — shipped PR #518.
- `OrderState.status: OrderStatusKind` — shipped (same wave).
- `OrderState.completed_status: String` — verified free-form by audit, **not** typed (rule 21 caveat).
- `ComboLeg.action: LegAction` + shared `parse_required` / `parse_optional` helpers — shipped PR #556 (PR 1).
- `Contract.right: Option<OptionRight>` — shipped PR #559 (PR 2).
- `impl_wire_enum!` macro + `parse_required(Option<&str>, ...)` precursor — shipped PR #564, #558 (PR 3a).
- `Contract.security_id_type: Option<SecurityIdType>` — shipped PR #568 (PR 3b).
- `impl_wire_enum!` crate-wide promotion + `some_display` + `OrderStatusKind` retrofit + shared `wire_enum` test helpers — shipped PR #569 (PR 4a).
- `ExecutionFilter.side: Option<ExecutionFilterSide>` — shipped PR #570 (PR 4b).
- `Execution.side: ExecutionSide` — shipped (PR 5b). C# `Execution.cs:83` is authoritative: documented vocabulary is two values, `"BOT"` and `"SLD"`. The original "wait for short-sale capture" gate was over-conservative — short-sale fills emit `"SLD"` (the SSHORT designation lives on the originating `Action`, not on `Execution.side`). Exhaustive enum (no `#[non_exhaustive]`) since the field is binary; strict `FromStr` fails loudly on unknown values so a hypothetical IBKR addition surfaces in the decoder before any `match` fires. Diagnostic branch `typed-status-sweep-pr5a-diagnostic` (PR 5a) was not merged; its Phase-1 capture (`{BOT, SLD}`) matched the spec.

## Audit summary

| Field | Wire vocab | Verified via | Status |
|-------|-----------|---|---|
| `ComboLeg.action: String` | `BUY`/`SELL`/`SSHORT` (no `SLONG` — combo-specific subset) | IBKR samples `ContractSamples.cs:459-588`; `EClient.cs:1289` SSHORT gating | **shipped #556** |
| `Contract.right: String` | `C`/`P` (and `CALL`/`PUT` historically) | C# `Contract.cs:Right`; IBKR option docs | **shipped #559** |
| `Contract.security_id_type: String` | `CUSIP`/`ISIN`/`SEDOL`/`RIC`/`FIGI` | C# `Contract.cs:104-109`; IBKR secIdType reference | **shipped #564, #568** |
| `ExecutionFilter.side: String` | `BUY`/`SELL` only (filter; empty = no filter) | doc comment `src/orders/mod.rs:1614`; `proto/encoders.rs:612` fixture | **shipped #569, #570** |
| `Execution.side: String` | `BOT`/`SLD` (binary; short-sale fills emit `SLD`) | C# `Execution.cs:83`; PR 5a Phase-1 capture matched | **shipped PR 5b** |

### Plan-text correction

The parent item in `v3-api-ergonomics.md` originally said `Execution.side` is `"Buy / Sell / SShort / SLng"`. That vocabulary belongs to **`Action`** (outgoing order side), not `Execution.side` (incoming fill side). The C# reference and our captured-wire fixtures show `BOT`/`SLD` for `Execution.side`. Short-sale variants are plausible (`SS`/`SSE`) but unverified — hence the live-diagnostic split (PR 5).

`ExecutionFilter.side` (outgoing filter, separate field on the same struct) is the one that takes `BUY`/`SELL`, typed as PR 4 (`ExecutionFilterSide`).

## Shared infrastructure (shipped)

Decoder helpers `parse_required<T>(opt: Option<&str>, label: &str) -> Result<T, Error>` and `parse_optional<T>(opt: Option<&str>) -> Result<Option<T>, Error>` live in `src/proto/decoders.rs`. Shipped in PR #556 (PR 1) with `&Option<String>` signature; refactored to `Option<&str>` in PR #558 (PR 3a) so text-path callers can pass `Some(s.as_str())` without re-wrapping. `parse_required` returns `Err(Error::Parse)` on empty/missing wire — never falls back to `T::default()` (rule 16).

Trait-impl macro `impl_wire_enum!` (Display + FromStr + ToField from `as_str` + `from_wire`) and the test helpers `check_wire_enum_round_trip<T>` / `check_wire_enum_rejects_unknown<T>` live in `src/macros.rs` and `src/common/test_utils.rs::wire_enum` respectively. Crate-wide reach via `#[macro_use] mod macros;` in `lib.rs`. Shipped in PR #569 (PR 4a). Every subsequent typed-status enum implements `as_str` + `from_wire` and calls `impl_wire_enum!(EnumT);` — no per-field plumbing.

## Implementation pattern

For every field (CLAUDE.md rule 16, mirroring PR #518):

1. **Define enum** next to the struct in its domain module. Annotate `#[non_exhaustive]` (CLAUDE.md §7 rule), derive `Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize`; derive `Default` only where the field has a sensible default.
2. **`impl Display`** — returns the canonical wire string (round-trips back into IBKR).
3. **`impl FromStr<Err = crate::Error>`** — unknown string → `Error::Parse`. This is the *only* per-field parse code; the generic `parse_required` / `parse_optional` helpers from "Shared infrastructure" do the `Option<String>` plumbing.
4. **Wire the decoder call site** in `src/proto/decoders.rs` to call `parse_required::<EnumT>(&proto.field, "EnumT")?` or `parse_optional::<EnumT>(&proto.field)?` depending on whether empty wire is valid (see PR sections below for which shape each field uses).
5. **Update encoder call site** — use `Display` via `some_str` / `ToField`.
6. **Update builder setters** to accept the enum directly (do not add `From<&str>` — silent defaults mask bugs per rule 21).
7. **Sibling `_tests.rs`** (CLAUDE.md rule 8):
   - Round-trip table over every variant: `for (v, s) in TABLE { assert_eq!(v.to_string(), s); assert_eq!(s.parse::<X>().unwrap(), v); }`.
   - For required fields: assert `parse_required::<X>(&None, "X")` and `parse_required::<X>(&Some(String::new()), "X")` both return `Err(Error::Parse)`.
   - For optional fields: assert both return `Ok(None)`.
   - Unknown wire value: `"NOTAVARIANT".parse::<X>()` returns `Err`.
8. **Sweep docs** per CLAUDE.md rule 16 preamble:
   - grep `README.md`, `docs/*.md`, module rustdoc for the field name and old `String` references.
   - manually verify markdown fenced blocks would compile (`docs/*.md` is **not** rustdoc-checked — `feedback_md_doc_snippets_rot_silently.md`).
   - update `docs/migration-3.0.md` §8 (or appropriate section) with the new type, default, and a one-line migration snippet.
9. **Run all three feature configs** (sync, default-async, all-features) for clippy + `cargo test` (CLAUDE.md "Quick Commands"); also `cargo build -p ibapi-integration-{sync,async} --tests` per rule 11.

## Per-PR scope

### PRs 1–4 — shipped

| PR | Field | Commits | Notes |
|---|---|---|---|
| 1 | `ComboLeg.action: LegAction` + `parse_required`/`parse_optional` helpers | #556 | Extended existing `LegAction` (not `ComboAction`); added `SellShort` variant. |
| 2 | `Contract.right: Option<OptionRight>` | #559 | Strict `"C"`/`"P"` only — `"CALL"`/`"PUT"` rejected as wire-cruft (PR #559 found these were VB-sample display fallbacks, not real wire). |
| 3a | `parse_required(Option<&str>, ...)` precursor; `impl_wire_enum!` macro | #558, #564 | Refactored `parse_required` signature; introduced `impl_wire_enum!` (module-local in `contracts/types.rs`). |
| 3b | `Contract.security_id_type: Option<SecurityIdType>` | #568 | 5-variant `#[non_exhaustive]` enum (CUSIP/ISIN/SEDOL/RIC/FIGI). |
| 4a | crate-wide `impl_wire_enum!` + `some_display` helper + `OrderStatusKind` retrofit | #569 | Promoted macro to `src/macros.rs` via `#[macro_use] mod macros;`; shared `wire_enum` test helpers at `src/common/test_utils.rs`. |
| 4b | `ExecutionFilter.side: Option<ExecutionFilterSide>` | #570 | Subset enum (`Buy`/`Sell` only — distinct from `Action` which has `SShort`/`SLong`). |

### PR 5 — `Execution.side: ExecutionSide` (split: 5a diagnostic, 5b typed)

#### PR 5a — Live diagnostic

Diagnostic test lives on branch `typed-status-sweep-pr5a-diagnostic` (not merged). Full plan + captured data: `plans/typed-status-sweep-pr5a.md` on that branch.

- **Status (2026-05-12)**: Phase-1 ran against a paper account with futures + long-equity fills only. 512 rows captured; distinct `side` values: `{BOT, SLD}`. Matches the documented vocab but **does not** cover short-sale fills (futures don't have `SSHORT`/`SLONG` order vocabulary; equity short-sale fills weren't in the slice).
- **Blocked on**: a stock-RTH short-sale capture. Path B selected — wait for a paper short-stock fill (manual via TWS, or via the diagnostic's Phase-2 active-submission test `execution_side_short_sale_capture` which submits `SPY sell_short(1) + buy(1)` market orders) before designing PR 5b's enum. Strictest rule-16 interpretation.
- **Approach**: `#[ignore]`-d sync integration test at `integration/sync/tests/executions_side_diagnostic.rs`. Both phases write to stdout AND `/tmp/exec-side-{diagnostic,short-diagnostic}.txt` per `feedback_buffered_stdio_diagnostics`.
- **Cleanup after PR 5b**: drop the diagnostic file if no short-sale variants surface; archive on the branch otherwise.

#### PR 5b — Strict enum

Gated on PR 5a's stock-short capture. Likely shape:

- **New enum**: `pub enum ExecutionSide { Bought, Sold }` (or more variants if short-sale fills emit something distinct). `#[non_exhaustive]` mandatory.
- **Wire vocab**: at minimum `"BOT"` → `Bought`, `"SLD"` → `Sold`. Display canonicalizes back to these strings via `impl_wire_enum!`.
- **Files**: `src/orders/mod.rs:1473` (field); `src/proto/decoders.rs:416` (`decode_execution`); fixtures at `orders/sync/tests.rs:538`, `orders/common/decoders/tests.rs:183`/`:354`, `testdata/builders/orders.rs:282`; examples printing `execution.side` (`examples/async/order_update_stream.rs:60`, `examples/async/place_order.rs:53`, `examples/sync/{readme_place_order.rs:34, submit_order.rs:63}`). These call `Display`, so they continue to print the wire string — no semantic break.
- **Migration**: `if exec.side == "BOT"` → `if exec.side == ExecutionSide::Bought`. Consider `PartialEq<&str>` for ergonomics (`Symbol`/`Exchange` macro precedent from PR #548).

## Cross-cutting checks (every PR)

- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo clippy --all-targets --features sync -- -D warnings`
- [ ] `cargo clippy --all-features`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` (× 3 feature configs)
- [ ] `just test`
- [ ] `cargo build -p ibapi-integration-sync --tests` + async equivalent (rule 11)
- [ ] Grep `README.md` + `docs/*.md` + module rustdoc for renamed identifiers; visually verify each snippet still compiles (CLAUDE.md rule 16 preamble).
- [ ] Update `docs/migration-3.0.md` entry in the same PR.

## Cross-cutting follow-ups

- **`Error::Parse(usize, String, String)` index slot — decide shape before PR 5b.** Every `FromStr<Err = Error>` impl in this sweep passes `0` as the `field_index` (legacy from text-protocol days; proto fields are name-keyed). Current count of fake-`0` sites after PR 4b: ~5 (covering `LegAction`, `OptionRight`, `SecurityIdType`, `ExecutionFilterSide`, and the `parse_required` / `impl_wire_enum!` macro `Err` arms). PR 5b adds one more. Past the inflection — three options to weigh: (1) make the index `Option<usize>`; (2) restructure as `Parse { index: Option<usize>, value, reason }`; (3) leave as-is + document the `0`-as-placeholder convention. Tracked in [`v3-api-ergonomics.md` §5](v3-api-ergonomics.md).

## Rule references

- CLAUDE.md rule 8 — separate `_tests.rs` files.
- CLAUDE.md rule 16 — typed-status migration pattern; reject empty/missing as `Error::Parse`; verify wire before typing.
- CLAUDE.md rule 21 — derive test expectations from the constant under test.
- CLAUDE.md rule 23 — restrictive API additions split into modernize-callers + restrict; one field per PR.
- `feedback_live_diagnostic_tests.md` — diagnostic-first when typed enum has unverified wire vocab.
- `feedback_verify_wire_before_typing.md` — grep fixtures + C# before typing `String` → enum.
- `feedback_md_doc_snippets_rot_silently.md` — manually verify markdown snippets; rustdoc doesn't check `.md` files.
