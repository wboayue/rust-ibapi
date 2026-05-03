# Issue #441 — `OrderStatus` doubles → `Option<f64>` (v3 breaking change)

Branch target: **`main`** only. The reported parse error is integer-parse misalignment, not double parsing — this PR fixes the latent API/spec mismatch raised in the issue but does **not** patch the symptom the reporter reproduced. Reporter has been silent for a month with no follow-up message/backtrace, so we close #441 on merge of this PR. If the underlying misalignment resurfaces from another reporter, treat it as a fresh issue.

## Problem

`OrderStatus.last_fill_price`, `market_cap_price`, and `average_fill_price` are currently `f64` and decoded with strict `next_double()` (text) / `unwrap_or_default()` (proto). But:

- The protobuf spec marks all three as `optional double` (`src/proto/protobuf.rs:1536, 1542, 1548`).
- The C# reference client uses `double.MaxValue` as the unset sentinel for all three on both wire formats (`EDecoder.cs:2528, 2542-2552`).
- IBKR routinely sends UNSET_DOUBLE (`1.7976931348623157E308`) for these fields. Today Rust silently surfaces `f64::MAX` to callers, conflating unset with a real numeric value.

`Option<f64>` is the idiomatic Rust translation of the C#/protobuf "may be unset" contract.

## Scope

**Three fields:**
- `OrderStatus::average_fill_price: f64` → `Option<f64>`
- `OrderStatus::last_fill_price: f64` → `Option<f64>`
- `OrderStatus::market_cap_price: f64` → `Option<f64>`

**Out of scope** (deliberately — keeps the diff minimal and focused):
- `filled` / `remaining` (decimal strings, separate concern; reporter didn't flag them).
- `perm_id`, `parent_id`, `client_id`, `order_id` (integer sentinels — IBKR rarely sends unset for these in practice).
- `status`, `why_held` (strings — `""` is harmless, no parse failure mode).
- v2-stable. The "patch" version (`next_optional_double()?.unwrap_or(f64::MAX)`) doesn't fix anything that's actually broken: for the literal UNSET_DOUBLE string `next_double()` already parses to `f64::MAX` (since `f64::MAX == 1.7976931348623157e308`) — same numeric value. The only behavioral delta is empty-field handling (`""` → `0.0` becomes `""` → `f64::MAX`), which IBKR doesn't send for these positions and which would diverge from C# `ReadDouble` (empty → 0). Not worth shipping. Ship the typed fix on v3 only.

## Files to change

### 1. `src/orders/mod.rs:1480, 1486, 1492`

Flip the three field types to `Option<f64>`. Update the doc comments to note that `None` means "IBKR did not send a value for this field" (UNSET_DOUBLE on text protocol; missing optional field on protobuf).

### 2. `src/orders/common/decoders/mod.rs:855-873` (`decode_order_status`)

Text decoder. Switch:
- `average_fill_price: message.next_double()?` → `next_optional_double()?`
- `last_fill_price: message.next_double()?` → `next_optional_double()?`
- `order_status.market_cap_price = message.next_double()?` → `next_optional_double()?`

`next_optional_double` already maps `UNSET_DOUBLE` → `None` and empty → `None` (`src/messages.rs:1119-1139`).

### 3. `src/orders/common/decoders/mod.rs:1149-1165` (`decode_order_status_proto`)

Protobuf decoder. Switch:
- `average_fill_price: p.avg_fill_price.unwrap_or_default()` → `p.avg_fill_price`
- `last_fill_price: p.last_fill_price.unwrap_or_default()` → `p.last_fill_price`
- `market_cap_price: p.mkt_cap_price.unwrap_or_default()` → `p.mkt_cap_price`

Proto fields are already `Option<f64>`, so this is a direct passthrough.

### 4. `src/orders/common/decoders/tests.rs:1197, 1200, 1203`

Wrap expected values: `Some(152.5)`, `Some(152.75)`, `Some(1.23)`.

**Add new tests** (CLAUDE.md rule: every new function needs a test, but here we're changing behavior — same standard applies):
- `decode_order_status_text_unset_double` — feed a wire string with `1.7976931348623157E308` for `last_fill_price` and `market_cap_price`, assert `None`.
- `decode_order_status_text_empty_double` — empty field for the same slots, assert `None`.
- `decode_order_status_proto_missing_doubles` — proto with `avg_fill_price`/`last_fill_price`/`mkt_cap_price` unset, assert `None`.

### 5. `src/orders/sync/tests.rs:198, 201, 204, 264, 265, 322, 325, 328`

Wire fixtures send literal `"0"` (not empty) for these slots, e.g. line 17: `"3|13|PreSubmitted|0|100|0|1376327563|0|0|100||0||"`. `next_optional_double()` only maps `""` and the UNSET_DOUBLE string to `None` — `"0"` parses to `Some(0.0)`. New assertions:

| line | wire field | new assertion |
|------|-----------|---------------|
| 198 | `"0"` | `Some(0.0)` |
| 201 | `"0"` | `Some(0.0)` |
| 204 | `"0"` | `Some(0.0)` |
| 264 | `"196.52"` | `Some(196.52)` |
| 265 | `"196.52"` | `Some(196.52)` |
| 322 | `"0"` | `Some(0.0)` |
| 325 | `"0"` | `Some(0.0)` |
| 328 | `"0"` | `Some(0.0)` |

This is a feature, not a quirk: `Some(0.0)` preserves the wire-observed value while `None` is reserved for "IBKR didn't send one". Callers can now distinguish the two — that's the whole point of the change.

### 6. `src/orders/builder/async_impl/tests.rs:333-365, 395`

Three handcrafted fixtures (`PendingSubmit`, `Submitted`, `Filled`) — values are arbitrary, not parsed from wire. Translate to model the typed contract honestly: `None` for unfilled stages, `Some` for the actual fill.

| line | status fixture | field | current | new |
|------|---------------|-------|---------|-----|
| 333 | PendingSubmit | `average_fill_price` | `0.0` | `None` |
| 336 | PendingSubmit | `last_fill_price` | `0.0` | `None` |
| 339 | PendingSubmit | `market_cap_price` | `0.0` | `None` |
| 346 | Submitted | `average_fill_price` | `0.0` | `None` |
| 349 | Submitted | `last_fill_price` | `0.0` | `None` |
| 352 | Submitted | `market_cap_price` | `0.0` | `None` |
| 359 | Filled | `average_fill_price` | `50.00` | `Some(50.00)` |
| 362 | Filled | `last_fill_price` | `50.00` | `Some(50.00)` |
| 365 | Filled | `market_cap_price` | `0.0` | `None` |
| 395 | (assert) | `average_fill_price` | `50.00` | `Some(50.00)` |

### 7. `examples/async/place_order.rs:55` and `examples/async/order_update_stream.rs:38`

`println!("  Avg Fill Price: {}", status.average_fill_price)` no longer compiles. Pick **one** display style and apply consistently:

```rust
println!("  Avg Fill Price: {}", status.average_fill_price.map_or("-".to_string(), |v| v.to_string()));
```

Or simpler — use `Debug`:

```rust
println!("  Avg Fill Price: {:?}", status.average_fill_price);
```

Prefer the `map_or("-")` form: examples are user-facing teaching material, and showing `Some(50.0)` to a new user is uglier than showing `-` for unset.

## Quality gates (per CLAUDE.md)

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings                 # default (async)
cargo clippy --all-targets --features sync -- -D warnings # sync-only
cargo clippy --all-features
just test
```

All three feature configurations must compile and pass tests — `OrderStatus` is shared across sync/async.

## Changelog / release notes

Add to `## What's New` (or a new `## Breaking Changes` section if one exists by then):

> ### `OrderStatus` price fields are now `Option<f64>` (#441)
>
> `average_fill_price`, `last_fill_price`, and `market_cap_price` change from `f64` to `Option<f64>`. IBKR sends an UNSET_DOUBLE sentinel (`1.7976931348623157E308`) when these fields have no value; the typed `Option` makes "unset" explicit instead of leaking `f64::MAX` into caller code.
>
> Migration: replace `status.last_fill_price` with `status.last_fill_price.unwrap_or(0.0)` (or pattern-match for the unset case).
>
> ```rust
> match status.last_fill_price {
>     Some(price) => println!("Last fill: {price}"),
>     None => println!("No fill yet"),
> }
> ```

Attribute the reporter (per memory: `feedback_release_notes.md`) — credit `@Assaf12345`.

## Self-review checklist (per `feedback_self_review.md`)

Before opening:
- [ ] Every new test function has coverage — text-unset, text-empty, proto-missing.
- [ ] No duplicated logic between text and proto decoders.
- [ ] All three feature configurations compile cleanly.
- [ ] Examples updated and visually verified (run them, even just to compile-check).
- [ ] Doc comments updated on the three struct fields.
- [ ] `serde` JSON behavior change (now serializes `null` instead of `0.0`) noted in PR description.
- [ ] PR title references `#441`.

## Issue close (on merge)

Use the PR description's `Closes #441` to auto-close. Add a brief comment on #441 at merge time:

> v3 fixes the underlying API mismatch by switching `average_fill_price`, `last_fill_price`, and `market_cap_price` to `Option<f64>` (#<PR>) — IBKR's UNSET_DOUBLE sentinel now decodes to `None` instead of leaking `f64::MAX` through.
>
> Note: the specific `"invalid digit found in string"` you reported is an integer parser error, not a double parser error (`1.7976931348623157E308` parses fine as `f64`). That points to a field-misalignment bug in a different slot (likely `perm_id`/`parent_id`). If you still see the parse failure on v3, please reopen with the raw message bytes (`IBAPI_RECORDING_DIR=/tmp/...`) and a backtrace and we'll dig in.
