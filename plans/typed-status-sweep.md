# Typed-Status Sweep Tracker

**End goal:** finish converting `String` fields with enumerated TWS wire vocabularies on public types into strict typed enums, following the PR #518 / `OrderStatusKind` precedent (CLAUDE.md rule 16). After this sweep, every "stringly-typed" public field carrying an enumerable wire value should be a typed enum that round-trips via `Display`/`FromStr`.

**Parent:** [v3-api-ergonomics.md §2 "Continue the typed-status sweep"](v3-api-ergonomics.md).

## Status today

- `OrderStatus.status: OrderStatusKind` — shipped PR #518.
- `OrderState.status: OrderStatusKind` — shipped (same wave).
- `OrderState.completed_status: String` — verified free-form by audit, **not** typed (rule 21 caveat).
- `ComboLeg.action: LegAction` + shared `parse_required` / `parse_optional` helpers — shipped PR #556 (PR 1).
- Remaining typed-status work tracked below.

## Audit summary

| Field | Wire vocab | Verified via | Plan PR |
|-------|-----------|---|---|
| `ComboLeg.action: String` | `BUY`/`SELL`/`SSHORT` (no `SLONG` — combo-specific subset) | IBKR samples `ContractSamples.cs:459-588`; `EClient.cs:1289` SSHORT gating | **PR 1** |
| `Contract.right: String` | `C`/`P` (and `CALL`/`PUT` historically) | C# `Contract.cs:Right`; IBKR option docs | **PR 2** |
| `Contract.security_id_type: String` | `CUSIP`/`ISIN`/`SEDOL`/`RIC`/`FIGI` | C# `Contract.cs:104-109`; IBKR secIdType reference | **PR 3** |
| `ExecutionFilter.side: String` | `BUY`/`SELL` only (filter; empty = no filter) | doc comment `src/orders/mod.rs:1614`; `proto/encoders.rs:612` fixture | **PR 4** |
| `Execution.side: String` | `BOT`/`SLD` confirmed; short-sale variants unverified | C# `Execution.cs:83`; fixtures `orders/{sync/tests.rs:538, common/decoders/tests.rs:183}`, `testdata/builders/orders.rs:282` | **PR 5a** (live diagnostic) → **PR 5b** (typed) |

### Plan-text correction

The parent item in `v3-api-ergonomics.md` says `Execution.side` is `"Buy / Sell / SShort / SLng"`. That vocabulary belongs to **`Action`** (outgoing order side), not `Execution.side` (incoming fill side). The C# reference and our captured-wire fixtures consistently show `BOT`/`SLD` for `Execution.side`. Short-sale variants are plausible (`SS`/`SSE`) but unverified — hence the live-diagnostic split (PR 5).

`ExecutionFilter.side` (outgoing filter, separate field on the same struct) is the one that takes `BUY`/`SELL`, and we're typing it as PR 4.

## Shared infrastructure (landed in PR 1)

Five hand-rolled `parse_X` helpers in `src/proto/decoders.rs` would duplicate the same `Option<String> → Result<T, Error>` shape. Extract two generics up front (in PR 1, before defining `ComboAction`):

```rust
/// Shape for required wire enums (the OrderStatusKind / ComboAction / ExecutionSide pattern).
/// `None` and `Some("")` are treated as malformed: empty wire on a required field is a TWS
/// protocol bug, not a default — surface it as `Error::Parse` instead of silently picking
/// `T::default()` (CLAUDE.md rule 16).
pub(crate) fn parse_required<T: FromStr<Err = Error>>(opt: &Option<String>, label: &str) -> Result<T, Error> {
    match opt.as_deref() {
        Some(s) if !s.is_empty() => s.parse(),
        _ => Err(Error::Parse(0, String::new(), format!("missing {label}"))),
    }
}

/// Shape for optional / filter wire enums (the SecurityIdType / OptionRight / ExecutionFilterSide pattern).
/// `None` and `Some("")` both mean "no value" (a valid wire state — e.g. non-option contracts
/// emit empty `right`); only an unknown non-empty string is an error.
pub(crate) fn parse_optional<T: FromStr<Err = Error>>(opt: &Option<String>) -> Result<Option<T>, Error> {
    match opt.as_deref() {
        Some(s) if !s.is_empty() => s.parse().map(Some),
        _ => Ok(None),
    }
}
```

PR 1 also refactors the existing `parse_order_status` (`src/proto/decoders.rs:43`) to call `parse_required(..., "OrderStatus")`. Every subsequent PR just implements `FromStr` on its enum and calls `parse_required` / `parse_optional` at the decode site — no new per-field helper.

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

### PR 1 — Shared infrastructure + `ComboLeg.action: ComboAction`

- **Why first**: small blast radius validates the sweep tooling; bundles the shared `parse_required` / `parse_optional` helpers (see "Shared infrastructure" above) with their first new consumer. Subsequent PRs only define an enum + `FromStr` and call the existing helpers.
- **Why a new enum, not `Action`**: the combo-leg wire vocabulary is a strict subset of `Action`. IBKR samples (`tws-api/samples/CSharp/Testbed/ContractSamples.cs:459-588`, 10 cases) use only `BUY`/`SELL`; `SSHORT` is gated for newer servers via `MIN_SERVER_VER_SSHORT_COMBO_LEGS` (`EClient.cs:1289`) — well below our floor of 210, so in scope. **`SLONG` is not accepted on combo legs** (no SLONG gate exists in `EClient.cs` for combo legs). Reusing `Action` would let callers write `combo_leg.action = Action::SellLong` — compiles fine, fails at the server. A narrow `ComboAction { Buy, Sell, SellShort }` makes that unrepresentable.
- **New enum**: `pub enum ComboAction { Buy, Sell, SellShort }`. `#[non_exhaustive]`; `Default = Buy`; `Display` → `"BUY"`/`"SELL"`/`"SSHORT"`; `FromStr` accepts those exact strings (case-sensitive — match `Action`'s precedent).
- **Shape**: required (no empty wire — combo legs always have an action). Decoder uses `parse_required::<ComboAction>(&proto.action, "ComboAction")?`.
- **Files**:
  - `src/proto/decoders.rs` — new `parse_required` / `parse_optional` helpers; refactor `parse_order_status` (line 43) to call `parse_required(opt, "OrderStatus")`.
  - `src/contracts/mod.rs:572` — `ComboLeg.action` field + new `ComboAction` enum.
  - `src/proto/decoders.rs` `decode_combo_leg` — call site update.
  - `src/proto/encoders.rs` — combo_leg encoder, `Display` via `some_str` / `ToField`.
  - `src/contracts/common/contract_builder/` — if combo legs flow through the builder.
  - Sibling `_tests.rs` for both the helpers (round-trip / boundary / unknown via a representative test enum) and `ComboAction`.
- **Migration note**: `combo_leg.action = "BUY".to_string()` → `combo_leg.action = ComboAction::Buy`. Document that this is **not** `Action` even though the wire strings overlap — different valid sets.

### PR 2 — `Contract.right: Option<OptionRight>`

- **Why second**: new enum but tiny (2 variants); empty wire is meaningful (non-option contracts).
- **New enum**: `pub enum OptionRight { Call, Put }`. `Display` → `"C"`/`"P"` (canonical); `FromStr` accepts `"C"`, `"CALL"`, `"P"`, `"PUT"` (case-insensitive — verify against C# `EClient.cs` encoder).
- **Empty wire = `None`**: non-option contracts emit `""` for `right`. Field becomes `Option<OptionRight>`.
- **Files**: `src/contracts/mod.rs` (struct + enum + `Contract::option()` constructor at `:544`); `src/proto/decoders.rs:72` and `decode_contract`; `src/proto/encoders.rs:right`; `src/contracts/common/contract_builder/mod.rs` (`right()` setter); sibling tests in `src/contracts/tests.rs` and `src/contracts/common/contract_builder/tests.rs` (currently at lines 30/49/415/453).
- **Migration**: `Contract::option("AAPL", "20260619", 200.0, "C")` keeps `&str` parameter (parsed via `FromStr`) for ergonomics; struct field exposes `Option<OptionRight>`.
- **Sweep targets**: every `Contract::option(...)` call in examples; `README.md` snippets; `docs/contract-builder.md`; `docs/order-types.md`.
- **Scope add (test-shape modernization)**: rewrite the existing `OptionRight` per-variant asserts at `src/contracts/types_tests.rs:107-114` into the table-driven loop shape PR 1 used for `LegAction` (`:117-149`). CLAUDE.md rule 21 — derive expectations from the constant. Drop the hand-rolled `assert_eq!(OptionRight::Call.as_str(), "C")` block, replace with a loop over `[OptionRight::Call, OptionRight::Put]` asserting `Display`/`FromStr` round-trip from `as_str()`. Rule 9 ("modernize touched modules") makes this in-scope. Tracked from [`v3-api-ergonomics.md` §2](v3-api-ergonomics.md).

### PR 3 — `Contract.security_id_type: Option<SecurityIdType>`

- **Why third**: explicitly named in the parent plan; largest API surface (both filter on outbound `placeOrder`/`reqContractDetails` and field on inbound `contractDetails`).
- **New enum**: `pub enum SecurityIdType { Cusip, Isin, Sedol, Ric, Figi }`. Verify FIGI / CONID against IBKR's secIdType reference doc before including. `#[non_exhaustive]` is mandatory (catalogue grows).
- **Empty wire = `None`**: no security-id filter set. Field becomes `Option<SecurityIdType>`.
- **Files**: `src/contracts/mod.rs:202` (field); `src/proto/decoders.rs:80` (`decode_contract`); `src/proto/encoders.rs:82` (contract encoder); `src/contracts/common/contract_builder/mod.rs:264` (setter — change from `impl Into<String>` to `impl Into<SecurityIdType>` or pair with explicit setter taking the enum); sibling tests.
- **Migration**: `ContractBuilder::security_id_type("CUSIP")` becomes `.security_id_type(SecurityIdType::Cusip)`. Worth adding a `From<&str>` for ergonomics? **No** — rule 21 says decoders should reject unknown wire; a `From<&str>` that silently defaults masks bugs. Use `TryFrom<&str>` or require `FromStr` + `?`.
- **Sweep targets**: `src/contracts/mod.rs:398, 431, 456, 477` (existing string literals in tests); `src/contracts/common/test_tables.rs:607`; `docs/migration-3.0.md` §8 (Contract section).

### PR 4 — `ExecutionFilter.side: Option<ExecutionFilterSide>` (split 4a + 4b)

Split on the PR 3a/3b precedent: infrastructure ships first, typing migration consumes it. Plan-time distillation pass (rule 4, rule 25, `feedback_distillation_cadence`) surfaced three duplication classes that warrant a focused infra PR:

1. `impl_wire_enum!` is module-local in `contracts/types.rs`; PR 4b's new `ExecutionFilterSide` would hand-roll `Display`/`FromStr`/`ToField` (and PR 5b's `ExecutionSide` again), deepening the existing `OrderStatusKind` hand-roll. 6 consumers across two modules — past rule 25's earned-cost threshold.
2. `check_wire_enum_round_trip<T>` / `check_wire_enum_rejects_unknown<T>` test helpers are private to `contracts/types_tests.rs`. `orders/tests.rs` hand-rolls the same shape for `OrderStatusKind`; PR 4b would add a third hand-roll.
3. PR 3b's /simplify pass tracked `some_display(Option<&impl Display>)` as a deferred follow-up. PR 4b makes it three call sites — the rule-of-three tripwire.

- **[PR 4a](typed-status-sweep-pr4a.md)** — promote `impl_wire_enum!` (and `impl_str_partial_eq!`) to crate-wide reach via `#[macro_use] mod macros;`; move test helpers to `src/common/test_utils.rs::wire_enum`; introduce `some_display` in `proto/encoders.rs` and convert the two existing PR 2/PR 3b sites; retrofit `OrderStatusKind` to use the macro. Zero behavior change.
- **[PR 4b](typed-status-sweep-pr4b.md)** — add `ExecutionFilterSide` (uses macro from 4a), promote field to `Option<ExecutionFilterSide>`, encoder uses `some_display`, tests use shared helpers, doc-examples for both sync + async (closes rule 18 gap on async `executions`), `prelude` re-export, migration-3.0 §12.

### PR 5 — `Execution.side: ExecutionSide` (split: 5a diagnostic, 5b typed)

#### PR 5a — Live diagnostic

- **Goal**: observe every distinct `side` value that IBKR's TWS emits across a representative slice of executions. Without this, typing strictly is the rule-21 trap.
- **Approach**: temp integration test under `integration/ibapi-integration-{sync,async}/` per `feedback_live_diagnostic_tests.md`. Submits a representative mix (long market buy, long market sell, simulated short sale if account permits), waits for `ExecutionData`, prints `execution.side` strings to stdout. Run against the user's paper account during US RTH (per `project_market_hours_retry_tests.md`).
- **Output**: a captured table of `(scenario, exec.side wire string)`. Add the table to this plan as a "PR 5b prerequisite — vocabulary" subsection.
- **Cleanup**: rip the diagnostic out after observation; do not merge.

#### PR 5b — Strict enum

- **New enum**: `pub enum ExecutionSide { ... }`. Variants come from PR 5a output. Likely 2 (`Bought`/`Sold`) — possibly more (short-sale forms, partials).
- **Wire vocab**: at minimum `"BOT"` → `Bought`, `"SLD"` → `Sold`. Display canonicalizes back to these strings.
- **`#[non_exhaustive]`** — even with confirmed vocab, leave headroom for IBKR additions.
- **Files**: `src/orders/mod.rs:1473` (field); `src/proto/decoders.rs:416` (`decode_execution`); fixtures at `orders/sync/tests.rs:538`, `orders/common/decoders/tests.rs:183`/`:354`, `testdata/builders/orders.rs:282`; examples printing `execution.side` (`examples/async/order_update_stream.rs:60`, `examples/async/place_order.rs:53`, `examples/sync/{readme_place_order.rs:34, submit_order.rs:63}`). These call `Display`, so they continue to print the wire string — no semantic break.
- **Migration**: `if exec.side == "BOT"` → `if exec.side == ExecutionSide::Bought`. Document `PartialEq<&str>` for ergonomics? Match the `Symbol`/`Exchange` precedent from PR #548 (macro-generated `PartialEq` impls) — consider but not required for this PR.

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

- **`Error::Parse(usize, String, String)` index slot — decide shape before PR 5.** Every new `FromStr<Err = Error>` impl in this sweep passes `0` as the `field_index` (legacy from text-protocol days; proto fields are name-keyed). After PR 1 shipped, the count of fake-`0` sites is 2 (`LegAction::from_str` + `parse_required`'s `Err` arm). PR 2/3/4/5 add 4 more. Five accumulated fakes is the inflection — three options to weigh: (1) make the index `Option<usize>`; (2) restructure as `Parse { index: Option<usize>, value, reason }`; (3) leave as-is + document the `0`-as-placeholder convention. Tracked in [`v3-api-ergonomics.md` §5](v3-api-ergonomics.md).

## Rule references

- CLAUDE.md rule 8 — separate `_tests.rs` files.
- CLAUDE.md rule 16 — typed-status migration pattern; reject empty/missing as `Error::Parse`; verify wire before typing.
- CLAUDE.md rule 21 — derive test expectations from the constant under test.
- CLAUDE.md rule 23 — restrictive API additions split into modernize-callers + restrict; one field per PR.
- `feedback_live_diagnostic_tests.md` — diagnostic-first when typed enum has unverified wire vocab.
- `feedback_verify_wire_before_typing.md` — grep fixtures + C# before typing `String` → enum.
- `feedback_md_doc_snippets_rot_silently.md` — manually verify markdown snippets; rustdoc doesn't check `.md` files.
