# Issue #716 — Fallible, sentinel-aware decimal wire parsing

> On approval, copy this file to `plans/decimal-wire-parsing.md` in the repo (per the plans-in-repo convention) before starting work.

## Context

IBKR models market-data sizes as `Decimal` and ships them on the wire as protobuf `optional string`
(e.g. `HistoricalTick.size`, `TickSize.size`). This crate decodes them through two helpers in
`src/proto/decoders.rs` that both end in `unwrap_or_default()`:

```rust
pub(crate) fn parse_f64(opt: &Option<String>) -> f64 { opt.as_deref().and_then(|s| s.parse::<f64>().ok()).unwrap_or_default() }
pub(crate) fn parse_i32(opt: &Option<String>) -> i32 { opt.as_deref().and_then(|s| s.parse::<i32>().ok()).unwrap_or_default() }
```

Three defects follow:

1. **Confirmed data loss.** Historical tick and histogram sizes are typed `i32`. A wire value of
   `"0.5"` fails `parse::<i32>()` and silently becomes `0`. Fractional sizes are real (crypto,
   fractional shares).
2. **Silent defaults.** Any malformed decimal field decodes to `0` instead of surfacing an error —
   contrary to CLAUDE.md rule 16.
3. **Unrecognized sentinels.** C# `Util.StringToDecimal` (tws-api `Util.cs:135`) maps `""`,
   `"2147483647"`, `"9223372036854775807"`, `"-9223372036854775808"` and `"1.7976931348623157E308"`
   to `decimal.MaxValue` — upstream's "unset" marker. We handle only the `f64::MAX` case, and only
   inside `optional_string_f64`. Today `"2147483647"` leaks into a size field as a plausible-looking
   2.1-billion quantity.

This is the **first of two PRs**. It introduces no new decimal type — a shared decimal quantity type
is deliberately deferred so the data-loss fix can ship on its own.

### Decisions already taken

- Market-data size fields that change type become **`Option<f64>`**, not `f64`: `Ok(Some(0.5))` for
  `"0.5"`, `Ok(None)` for absent/empty/sentinel, `Err(Error::Parse)` for malformed.
- The parse-helper migration is **crate-wide** (all 32 string-parsing call sites), not market-data only.
- `ContractDetails.{min_size, size_increment, suggested_size_increment}` are **included** in the
  `Option<f64>` change — they are Pattern-U upstream and `size_increment = 0.0` is a nonsense value.
- Ships as **two PRs**: PR-A (helpers, no public API change) then PR-B (public field types).

### Behavioural consequence to be explicit about

`Error::Parse` is **terminal for a subscription**. `process_decode_result`
(`src/subscriptions/common.rs:112-119`) skip-classifies only `Error::UnexpectedResponse`; everything
else becomes `ProcessingResult::Error`. That is the intended rule-16 outcome, but it is user-visible
and must be changelogged and tested.

---

## Upstream semantics (the mapping table this plan rests on)

Every decimal-string site upstream falls into one of two patterns, established by reading
`EDecoder.cs` / `EDecoderUtils.cs` and the C# constructors:

- **Pattern U — absent means unset.** C# writes `Has X ? Util.StringToDecimal(..) : decimal.MaxValue`,
  or the constructor pre-initializes the field to `decimal.MaxValue`.
- **Pattern D — absent means 0.** C# writes `if (proto.HasX) obj.X = ...` and the constructor leaves
  the field at `decimal`'s default of `0`.

Pattern D is only `Order.total_quantity` (`Order.cs` has no ctor init) and `Execution.shares` /
`cumulative_quantity` (`Execution.cs:166,170` set `0`). Everything else is Pattern U.

**The two patterns do not need two helpers now.** They differ only in what the field holds when
absent, and every Pattern-D field here is a plain `f64` that cannot represent "unset" — so both
collapse to the identical observable `0.0`. Carry the distinction as a `// pattern U` / `// pattern D`
comment at each call site so the step-2 PR knows which fields become optional, and skip the second
helper (rule 25: no speculative infrastructure).

---

## PR-A — "Make decimal wire parsing fallible"

No public API change. ~32 call sites, all mechanical once the helpers land.

### A1. New helpers in `src/proto/decoders.rs`

Insert below `optional_f64` (line 27) under a banner comment separating the two helper families.

```rust
// === Decimal wire fields (protobuf `optional string`) ===
//
// IBKR ships sizes, quantities and volumes as decimal strings. These helpers are
// the only sanctioned way to read one; they are the numeric counterpart to the
// `parse_required` / `parse_optional` pair below, which handle enumerated fields.

/// Integer sentinels TWS uses to mean "no value" on a decimal-typed field.
///
/// These are the ones that must be matched **literally**, because they parse to
/// perfectly ordinary finite numbers — `"2147483647"` is a sentinel but
/// `"2147483647.0"` is a real size and must survive. The float sentinel
/// (`f64::MAX`) is deliberately *not* here; it is caught numerically after the
/// parse, which covers every spelling at once.
///
/// Mirrors `Util.StringToDecimal` in the C# reference client
/// (tws-api `source/csharpclient/client/Util.cs:135`), which maps each of these
/// to `decimal.MaxValue` — upstream's "unset" marker.
const UNSET_DECIMAL_WIRE: [&str; 3] = [
    "9223372036854775807",  // i64::MAX
    "2147483647",           // i32::MAX
    "-9223372036854775808", // i64::MIN
];

pub(crate) fn parse_optional_decimal(opt: Option<&str>) -> Result<Option<f64>, Error>;
pub(crate) fn parse_decimal_or_zero(opt: Option<&str>) -> Result<f64, Error>;
```

`parse_optional_decimal` semantics: absent / empty / literal sentinel → `Ok(None)`; parse with
`f64::from_str`; then a numeric guard `!value.is_finite() || value == f64::MAX` → `Ok(None)`;
anything else non-numeric → `Err(Error::parse_field(text, ..))`. Use `Error::parse_field` (not
`Error::parse_proto`) — we pass the offending *value*, which is what `parse_field` documents at
`src/errors.rs:166`.

**Two unset mechanisms, one job each** — this split is deliberate. The literal table handles
sentinels that parse to legitimate-looking finite values, where only string equality can distinguish
them from real data. The numeric guard handles everything that parses to `f64::MAX`, `inf` or `NaN`,
which is spelling-independent and so catches `"1.7976931348623157E308"`, `"…e308"`, `"…E+308"` and
`"1e309"` with one comparison. Listing the float sentinel in the table *as well* would be a second
mechanism for a case the guard already owns — redundant, and it would rot if C# ever changed its
round-trip formatting. Observable behaviour is identical to upstream either way; this factoring just
keeps each mechanism responsible for exactly the cases it alone can decide. The guard also preserves
the retired `optional_string_f64`'s `v == f64::MAX` filter, so the six already-`Option<f64>` sites
keep their semantics.

`parse_decimal_or_zero` is `parse_optional_decimal(..)?.unwrap_or(0.0)`, documented as **transitional**:
it exists only for fields still typed `f64`, and grepping its name yields the step-2 worklist.

Design points to defend in review:

- **Signature is `Option<&str>`, not `&Option<String>`** — matches the `parse_required`/`parse_optional`
  convention established by PRs #556/#558. Call sites pay `.as_deref()`; helpers become trivially
  unit-testable.
- **No whitespace trimming.** `Some(" ")` → `Err`. C#'s `decimal.Parse(" ")` also throws, so erroring
  is the faithful mapping and the loud option. Pin it with a test.
- **Do not rename `parse_required`/`parse_optional`.** CLAUDE.md rule 16 names them verbatim; the
  `_decimal` infix is the disambiguator and survives step 2 unchanged.

### A2. Delete `parse_f64` and `optional_string_f64`; keep `parse_i32` for now

`parse_i32` stays only so the crate is green between PRs — its five call sites all move in PR-B.
`optional_f64` (line 27) takes `Option<f64>` from a proto `double`, does no string parsing, and is
**not** in the bug class — leave it alone.

### A3. Migrate 32 call sites

| File | Sites | Helper |
|---|---|---|
| `src/proto/decoders.rs` | `:148` total_quantity (**D**), `:305` filled_quantity, `:438`/`:442` shares/cum_qty (**D**), `:502-504` contract-details sizes, **`:460` inline `min_tick`** | `parse_decimal_or_zero` |
| `src/proto/decoders.rs` | `:408` suggested_size, `:420-424` OrderAllocation ×5 | `parse_optional_decimal` (already `Option<f64>`) |
| `src/market_data/realtime/common/decoders/mod.rs` | `:80`/`:81` volume/wap, `:100` Trade.size, `:124`/`:125` bid/ask size, `:170` min_tick, `:221` TickSize.size, `:272`/`:289` MarketDepth | `parse_decimal_or_zero` |
| `src/market_data/realtime/common/decoders/mod.rs` | `:181` tick-price size | `parse_optional_decimal` — direct swap for `optional_string_f64` |
| `src/market_data/historical/common/decoders/mod.rs` | `:121`/`:122` Bar.volume/wap | `parse_decimal_or_zero` |
| `src/accounts/common/decoders/mod.rs` | `:99`, `:119`, `:141`, `:172` position | `parse_decimal_or_zero` |
| `src/orders/common/decoders/mod.rs` | `:62`/`:63` filled/remaining | `parse_decimal_or_zero` |

`src/proto/decoders.rs:460` is an **inline** silent parse the original inventory missed —
`min_tick.as_deref().and_then(|s| s.parse().ok()).unwrap_or_default()`. A crate-wide grep for
`parse().ok()` in non-test `src/` returns exactly this plus the helper bodies.

Both `min_tick` sites are `Util.StringToDoubleMax` upstream, not `StringToDecimal`. Using the decimal
helper is a harmless superset (a `min_tick` of literally `2147483647` is physically impossible); add a
one-line comment saying so at each.

### A4. Signature ripples (infallible → `Result`)

| Function | Callers to fix |
|---|---|
| `decode_order` (`src/proto/decoders.rs:139`) | `src/orders/common/decoders/mod.rs:40,102` — `.map(decode_order).unwrap_or_default()` → `.map(decode_order).transpose()?.unwrap_or_default()`; 4 test callers at `src/proto/decoders_tests.rs:128,134,146,152` gain `.unwrap()` |
| `decode_order_allocation` (`:417`) | one caller at `:410`, inside the already-`Result` `decode_order_state` → `.collect::<Result<Vec<_>, Error>>()?` |
| `decode_historical_data_bar` (`src/market_data/historical/common/decoders/mod.rs:109`) | `:106` → `.collect::<Result<Vec<_>, Error>>()`; `:243` gains `?` |

Verified **not** affected (no string parse in their bodies): `decode_delta_neutral_contract` (`:123`),
`decode_soft_dollar_tier` (`:131`), `decode_order_condition` (`:332`). `src/proto` is `pub(crate)`
(`src/lib.rs:208`), so none of this is a public-API break.

### A5. Tests

Two tiers with distinct jobs, so neither repeats the other's work:

**Tier 1 — the helper's semantics, exhaustively, once.** In `src/proto/decoders_tests.rs`, matching
the existing table-driven style at `:7-63`. One `#[test]` per behaviour class, each looping a
`for (input, expected) in [...]` table. Derive the sentinel rows from `UNSET_DECIMAL_WIRE` itself, not
re-typed literals (rule 21).

Classes: absent · empty · each literal sentinel · float-sentinel spellings (`E308`, `e308`, `E+308`)
· non-finite (`inf`, `NaN`, `1e309`) · **fractional** (`"0.5"`, `"0.001"`, `"-0.25"`) · integral
(`"0"` is a real zero, distinct from `None`) · **sentinel near-miss** (`"2147483647.0"`,
`"9223372036854775806"` → `Some`, which is what proves the literal/numeric split of A1 is doing its
job) · **more digits than f64 round-trips** (`"0.1234567890123456789"` → assert the exact nearest f64
`0.12345678901234568`, with a comment that this precision loss is accepted here and motivates step 2)
· malformed (`"abc"`, `" "`, `"1,000"`, `"1.2.3"`) → `Err(Error::Parse(0, value, _))`, asserting the
payload round-trips the offending string.

`parse_decimal_or_zero` is `parse_optional_decimal(..)?.unwrap_or(0.0)`, so re-running all of the
above through it would test `unwrap_or` twenty times. It gets **three** cases proving the delegation:
an unset-class input → `Ok(0.0)`, a value → passes through, a malformed input → `Err` propagates.

**Tier 2 — that each decoder is actually wired to the helper.** Tier 1 owns "does the parsing work";
Tier 2 owns "does this decoder route its field through it", which is one assertion per field, not
three. Default to a single malformed→`Err` case per decoder, and add a fractional case *only* where
it guards a real prior defect or a real user report:

| Decoder | Cases |
|---|---|
| historical ticks ×3, histogram (PR-B) | malformed→`Err` **+ fractional** — these are the `i32` truncation sites, the regression the issue reports |
| accounts `decode_position_proto` | malformed→`Err` **+ fractional** — crypto positions are the field users actually hit |
| `decode_contract_details` | malformed→`Err` **+ sentinel→`None`** (PR-B) — `size_increment` is why the trio changed type |
| realtime `decode_tick_size_proto`, `_market_depth_proto`/`_l2_proto`, `_trade_tick_proto`, `_bid_ask_tick_proto`, `_realtime_bar_proto`; `decode_order`, `decode_execution`, `decode_order_status_proto`; the 3 remaining accounts decoders | malformed→`Err` only |

Extract one shared `assert_decimal_parse_error(result)` (asserts `Err(Error::Parse(..))` and that the
payload names the offending value) rather than open-coding the same `matches!` in ~16 places. Do
**not** try to generify across the (builder, decoder, accessor) triples — the proto types are
heterogeneous, so a generic would need HRTB gymnastics to save three lines a site; rule 25's test is
"would ordinary Rust be simpler", and here the plain helper plus per-decoder `#[test]` fns is simpler.

Two named behaviour-change tests, because they encode contract changes rather than field wiring:
`decode_tick_price_proto` with a **malformed** size currently degrades silently to `TickTypes::Price`
— assert it now returns `Err`, and keep a sentinel case asserting it *still* degrades to
`TickTypes::Price` (the `Ok(None)` path is unchanged); and `decode_order_state` / `decode_order_allocation`
with a sentinel → `None`, guarding that `optional_string_f64`'s semantics survived the swap.

### A6. Changelog

`## [Unreleased]` → `### Fixed` only. Two bullets: malformed decimal fields now surface `Error::Parse`
instead of decoding to `0`; TWS unset sentinels now recognized on every decimal field instead of
leaking as a literal 2.1-billion size. No migration-guide entry — no public API change.

---

## PR-B — "Historical, histogram and contract-details sizes as `Option<f64>`"

Closes #716.

### B1. Public field type changes

| File | Fields | Change |
|---|---|---|
| `src/market_data/historical/mod.rs` | `:449` `HistogramEntry.size`, `:550` `TickMidpoint.size`, `:566`/`:568` `TickBidAsk.size_bid`/`size_ask`, `:592` `TickLast.size` | `i32` → `Option<f64>` |
| `src/contracts/mod.rs` | `:724` `min_size`, `:726` `size_increment`, `:728` `suggested_size_increment` | `f64` → `Option<f64>` |

Rewrite each field's doc comment to state what `None` means (absent / empty / sentinel) and that
`Some(0.0)` is a real zero.

### B2. Decoder migration

- `src/market_data/historical/common/decoders/mod.rs:141,163,189,190,211` → `parse_optional_decimal`.
  The enclosing `.map(..)` closures now yield `Result`, so each becomes
  `.collect::<Result<Vec<_>, Error>>()?`.
- `src/proto/decoders.rs:502-504` → switch from `parse_decimal_or_zero` (PR-A) to
  `parse_optional_decimal`.
- **Delete `parse_i32`.** A crate-wide grep confirms no other `String`→`i32` proto field exists —
  every other integer arrives as a proto `int32`/`int64` read via `.unwrap_or_default()`.

### B3. Test fixture builders — `src/testdata/builders/market_data.rs` (rule 19: they stay here)

Four fixture structs store `i32` and stringify in `to_proto`: `HistoricalTickMidFields.size` (`:926`),
`HistoricalTickLastFields.size` (`:994`), `HistoricalTickBidAskFields.size_bid`/`size_ask`
(`:1094-1095`), `HistogramDataEntryFields.size` (`:1185`).

Reshape each to hold the **raw wire form** so fractional / sentinel / empty / absent fixtures are all
expressible:

```rust
pub struct HistoricalTickMidFields {
    pub time: i64,
    pub price: f64,
    /// Raw `optional string size` wire value. `None` = field absent.
    pub size: Option<String>,
}
// free entry-point fn keeps the ergonomic path:
pub fn historical_tick_mid(time: i64, price: f64, size: f64) -> HistoricalTickMidFields
```

**No new setters.** The obvious move is a `.size_wire(..)` / `.size_absent()` pair on each of the four
structs — ~10 near-identical three-line methods whose only difference is a field name. They aren't
needed: the fields are already `pub` and these fixtures are already built as free fn + struct literal
(the documented testdata-builder convention), so struct-update syntax expresses every edge case with
no new API surface:

```rust
HistogramDataEntryFields { size: Some("2147483647".into()), ..histogram_entry(125.50, 1000.0) }
HistogramDataEntryFields { size: None,                      ..histogram_entry(125.50, 1000.0) }
```

Entry-point params go `i32` → `f64`, so ~30 existing call sites need `100` → `100.0`
(`src/market_data/historical/sync_tests.rs`, `async_tests.rs`, `common/tick_tests.rs`).
`to_proto` just clones the `Option<String>` through — no `.to_string()`.

**Deferred, not bundled:** `historical_tick_bid_ask` takes 5 positional args against rule 4's limit of
3. Fixing it is restructuring, not part of this fix, and it would churn 10 call sites for reasons
unrelated to the bug — so it goes in the follow-ups list, not this PR.

### B4. Assertion updates

`src/market_data/historical/common/decoders/tests.rs:96,103,142,171,172,194,196` → `Some(100.0)` etc.
`sync_tests.rs:139,142,145,695,696`; `async_tests.rs:155,159,163,417,418,497,498,540,577`.
ContractDetails: `src/contracts/async_tests.rs:388-390` and
`src/contracts/common/test_tables.rs:171-173` → `Some(1.0)` / `Some(100.0)`.
Note `src/testdata/builders/contracts.rs:351-356` documents `min_size: "1"` as load-bearing for those
validators — keep the default, change only the expectation.

### B5. Examples (async only — sync examples do not touch these fields)

- **`examples/async/histogram_data.rs`** — `:120` `max_by_key(|e| e.size)` will not compile
  (`f64: !Ord`); use `.filter_map(|e| e.size.map(|s| (e, s))).max_by(|a, b| a.1.total_cmp(&b.1))`.
  `:69`/`:82` sums and max become `filter_map` + `fold(1.0_f64, f64::max)`; `:74`, `:121-122`,
  `:139-140`, `:143` drop their `as i64` / `as f64` casts; `print_histogram_entry`'s
  `total_count`/`max_count` params become `f64` (stays at 3 params, rule 4 limit).
- **`examples/async/historical_ticks_trade.rs`** — `:65`/`:66` `tick.size as f64` becomes
  `unnecessary_cast` (CI runs `-D warnings`); `:83`/`:126` print `Option<f64>` with `{}`/`{:5.0}`,
  a hard compile error.
- **`examples/async/historical_ticks.rs`** — `:56`, `:120`, `:122` same display break; `:85`
  `total_volume += tick.size` where `total_volume` is inferred `i32` at `:78`.
- **`examples/async/historical_data.rs:200`** — prints `HistogramEntry.size` with `{}`. Missed by the
  first sweep; same hard compile error.

Examples are the canonical teaching surface (rule 18, and the doc-example user-trace lesson), so show
the `None` case rather than hiding it: a two-line `fn fmt_size(size: Option<f64>) -> String` using
`map_or_else(|| "n/a".into(), |s| format!("{s:.0}"))` for display columns, and `.unwrap_or(0.0)` only
inside accumulators where "unset contributes nothing" is the correct arithmetic.

`fmt_size` gets copied into each of the four examples rather than hoisted into a shared module. That
is intentional and worth a line in the PR description so a reviewer doesn't flag it: each example must
compile and read standalone, since users copy one file — the same reasoning that keeps `ibapi-test`
free of an `ibapi` dependency. A shared example-helper module would trade four duplicated lines for a
cross-file indirection in the one place where self-containment *is* the feature.

### B6. Docs

- `CHANGELOG.md` `### Changed` — the five historical/histogram fields and the three ContractDetails
  fields, with the reason (wire is decimal; `i32` truncated `0.5` to `0`) and what `None` means.
- `docs/migration-3.0.md` — new section after `### 34` (`:849`): before/after field table, what `None`
  means, the **`max_by_key` → `max_by` + `total_cmp` gotcha** (the most likely downstream compile
  break), accumulator and display idioms, and a forward pointer that a dedicated decimal quantity type
  is planned so `Option<f64>` is an intermediate.
- Grep sweep over `README.md`, `docs/*.md` and module rustdoc for `HistogramEntry`, `TickMidpoint`,
  `TickLast`, `TickBidAsk`, `size_bid`, `size_ask`, `min_size`, `size_increment`, `parse_f64`,
  `parse_i32`, `optional_string_f64` — markdown fences are never compile-checked, so *read* each hit.
  Pre-check found no prose snippet touching the changing fields (`docs/quick-start.md:187,195` hit
  `bar.volume`, which stays `f64`), but re-run and re-read after the edit rather than trusting it.

### B7. Subscription-contract test

One sync and one async test feeding a malformed-size fixture through the real client, asserting the
consumer sees `Some(Err(Error::Parse(..)))` and then `None` — `Error::Parse` is terminal per
`src/subscriptions/common.rs:117`, and that is a user-visible change. Use
`proto_response(IncomingMessages::HistogramData, builder.encode_proto())` over a malformed-size
fixture built with struct-update syntax (per B3):
`HistogramDataEntryFields { size: Some("abc".into()), ..histogram_entry(125.50, 1000.0) }`.

---

## Deliberately out of scope

- **A decimal quantity type** — the step-2 PR. Every `parse_decimal_or_zero` call site is its worklist.
- **Remaining Pattern-U `f64` fields** (`OrderStatus.filled`/`remaining`, `Position.position`,
  `Trade.size`, `TickSize.size`, `MarketDepth.size`, `Bar.volume`/`wap`, both `min_tick`s) — always
  present on real wire, so `0.0` is indistinguishable from today. Defer.
- **Rule-8 module flattening** (`common/decoders/mod.rs` + `tests.rs` → flat `decoders.rs` +
  `decoders_tests.rs`, four module pairs) — a large mechanical sweep unrelated to the fix; rule 9's own
  carve-out sends it to its own PR. Say so in the PR description so it does not read as an oversight.
- **`src/proto/encoders.rs:83,382`** — outbound `filter_map(|d| d.parse::<i32>().ok())` silently drops
  malformed dates. Same bug class, opposite direction; file a separate issue.
- **`historical_tick_bid_ask`'s 5 positional args** (rule 4 limit is 3) — restructuring, unrelated to
  the bug; see B3.

## Distillation review (duplication / SRP / composability)

What the three lenses changed, so the reasoning survives into review:

| Lens | Finding | Resolution |
|---|---|---|
| SRP | Sentinel detection had **two mechanisms for one concern** — a literal table that included the float sentinel *and* a post-parse `== f64::MAX` guard that also caught it | Split by what each can uniquely decide: literal table keeps the 3 integer sentinels (only string equality separates them from real data), numeric guard owns `f64::MAX`/`inf`/`NaN` (spelling-independent). Redundant table entry removed |
| Duplication | Test plan ran fractional + malformed + sentinel across ~16 decoders, and re-ran the full class table through `parse_decimal_or_zero` | Two tiers with distinct jobs: helper semantics exhaustively **once**; per-decoder tests reduced to one wiring assertion, with fractional cases kept only where they guard a real prior defect. `parse_decimal_or_zero` gets 3 delegation cases, not 20 |
| Duplication | ~16 open-coded `matches!(.., Err(Error::Parse(..)))` assertions | One shared `assert_decimal_parse_error(result)` |
| Composability | ~10 near-identical `.size_wire()` / `.size_absent()` fixture setters across 4 builder structs | Deleted before being written — fields are already `pub` and the convention is free fn + struct literal, so struct-update syntax covers every edge case with zero new API |
| SRP | B3 bundled the storage reshape with an unrelated arg-count cleanup | Cleanup demoted to follow-ups |
| Duplication | `fmt_size` copied across 4 examples | Kept deliberately — self-containment is the feature for examples; noted so a reviewer doesn't "fix" it |

Two duplications examined and **kept**: `parse_decimal_or_zero` is a composition of
`parse_optional_decimal`, not a parallel implementation, and it earns its name by marking the step-2
worklist; and CHANGELOG-vs-migration-guide overlap is the documented house style, not drift.

One honest cost of the PR split: `src/market_data/historical/common/decoders/mod.rs` and its tests get
touched twice — PR-A for `Bar.volume`/`wap`, PR-B for the tick sizes. Accepted, because the two PRs
have genuinely different review surfaces (helper semantics vs. downstream API break).

## Verification (run for each PR)

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features

RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

just test                 # sync then async
cargo test --all-features # NOT covered by `just test`; needed for the utoipa derive
                          # on the reshaped Option<f64> fields

# Rule 11 — this touches proto decoding, so the integration crates
# (not in default-members) must be compile-checked
cargo build  -p ibapi-integration-sync  --tests
cargo build  -p ibapi-integration-async --tests
cargo clippy -p ibapi-integration-sync  --tests -- -D warnings
cargo clippy -p ibapi-integration-async --tests -- -D warnings

just cover                # touched modules stay >= 90%
```

Integration crates touch only `data.bars[0].volume`
(`integration/{sync,async}/tests/historical_data.rs:60,62`), which keeps its `f64` type — the check
should pass unchanged, but run it; it is the contract.

Expect these clippy lints specifically: `unnecessary_cast` (examples, once `as f64` on an already-`f64`
value goes redundant), `unused_mut` on accumulator bindings whose type changes, and `manual_let_else`
if the helper body is written with `match` rather than `let ... else`.

Coverage watch: the `!value.is_finite()` and `value == f64::MAX` arms of `parse_optional_decimal` are
the most likely to show uncovered — the non-finite and sentinel-spelling test rows exist to cover them.

### End-to-end check against a live gateway (optional but valuable)

Run `cargo run --example historical_ticks_trade` and `--example histogram_data` against IB Gateway
paper (127.0.0.1:4002) on a crypto contract, which is where fractional sizes actually appear. Watch for
`Some(0.5)`-style values where the old build printed `0`. Note the gateway reset windows (00:15-01:45
ET daily) before blaming a silent hang on the code.
