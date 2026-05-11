# Typed-Status Sweep — PR 1 implementation plan

**Parent:** [typed-status-sweep.md](typed-status-sweep.md) §"PR 1 — Shared infrastructure + `ComboLeg.action: ComboAction`".

**Scope:** ship the shared `parse_required` / `parse_optional` helpers in `src/proto/decoders.rs`, refactor `parse_order_status` onto them, and type `ComboLeg.action: String` → `LegAction` with `SellShort` added.

**Plan correction:** the parent plan names the new enum `ComboAction { Buy, Sell, SellShort }`. The codebase already has `LegAction { Buy, Sell }` in `src/contracts/types.rs:526` used by `SpreadBuilder::add_leg(_, LegAction)`. Two enums for the same wire vocabulary is the worst outcome; PR 1 extends `LegAction` with `SellShort` and reuses it as `ComboLeg.action`'s field type. The parent plan's `ComboAction` text reads as "the typed combo-leg action enum" — fulfilled by an extended `LegAction`.

---

## Files touched

| File | Change |
|---|---|
| `src/proto/decoders.rs` | Add `parse_required<T>` / `parse_optional<T>` generic helpers; **delete `parse_order_status` and inline `parse_required(&proto.status, "OrderStatus")?` at its 2 callers**; make `decode_combo_leg` / `decode_contract` fallible (`Result<_, Error>`). |
| `src/proto/decoders_tests.rs` *(new sibling)* | Round-trip + boundary tests for the two helpers, using `OrderStatusKind` (already implements `FromStr`) as the test enum — no new test-only types. Also covers `decode_combo_leg` end-to-end fallibility (CLAUDE.md rule 10). |
| `src/contracts/types.rs` | Extend `LegAction`: add `SellShort`, `#[non_exhaustive]`, `Default = Buy`, `Copy`, `Hash`, `Serialize`, `Deserialize`; `impl FromStr<Err = Error>` (case-sensitive `"BUY"`/`"SELL"`/`"SSHORT"`, unknown → `Error::Parse`); `impl ToField` delegating to `Display`. Keep `as_str()`. |
| `src/contracts/types_tests.rs` | Add round-trip table over all variants; assert unknown wire → `Err(Error::Parse)`; assert `Display`/`FromStr` symmetric. Derive expected strings from `as_str()` per CLAUDE.md rule 21 — don't hardcode `"SSHORT"` again. |
| `src/contracts/mod.rs` | `pub action: String` → `pub action: LegAction`; `impl Default for ComboLeg` sets `action: LegAction::Buy`; update test fixtures at `:1156`/`:1163` (`action: "BUY".to_string()` → `action: LegAction::Buy`). |
| `src/proto/encoders.rs` | `encode_combo_leg`: `action: some_str(&leg.action)` → `action: some_str(&leg.action.to_string())` (or call `leg.action.to_field()` and wrap with `some_str` if `LegAction::Buy` should emit `"BUY"` rather than `None`). Confirm by reading the C# encoder — combo-leg action is **required** on the wire, so we want `Some("BUY")` not `None`. Likely: `action: Some(leg.action.to_string())` to bypass the empty-string drop. |
| `src/contracts/builders.rs` | `builders.rs:525` `action: leg.action.to_string()` → `action: leg.action` (passes the enum through; no more `.to_string()` round-trip). |
| `src/contracts/builders/tests.rs` | `assert_eq!(spread.combo_legs[0].action, "BUY")` → `assert_eq!(spread.combo_legs[0].action, LegAction::Buy)` (7 sites: `:204, :207, :220, :222, :245, :250, :254`). |
| `src/contracts/common/contract_builder/tests.rs` | Same conversion at `:288, :295, :310, :312, :421`. Struct-literal sites use `action: LegAction::Buy/Sell`. |
| `src/market_data/realtime/{sync,async}/tests.rs` | `tests.rs:619` (async) / `:516` (sync): `ComboLeg { ..., action: "BUY".to_string(), ... }` → `action: LegAction::Buy`. |
| `src/orders/sync/tests.rs` | `:383, :391` `ComboLeg { ... action: "BUY".to_string() ... }` and `"SELL"` → `LegAction::Buy/Sell`. |
| `src/testdata/builders/orders.rs` | `:474, :650` are `Order.action` (not combo legs) — verify, **leave unchanged**. |
| Callers of `decode_contract` (fallout from making it fallible) | 9 callsites; see "Decoder fallibility blast radius" below. |
| `docs/migration-3.0.md` | Add §"9. `ComboLeg.action` typed as `LegAction`" mirroring §5's shape (before/after, `Display` round-trip note, `Err(Error::Parse)` on unknown wire). |
| `README.md` | Grep for `combo_leg` / `ComboLeg` / `.action = "BUY"` — no current snippets touch this (verified), but re-grep before PR. |

---

## Decoder fallibility blast radius

`decode_combo_leg` and `decode_contract` become `Result<_, Error>`. Today there are 9 callsites of `decode_contract` (5 in `accounts/common/decoders/mod.rs`, 2 in `orders/common/decoders/mod.rs`, 1 in `scanner/common/decoders.rs`, 1 in `contracts/common/decoders/mod.rs`) plus 1 of `decode_contract_details` which now fails through. All currently follow the same shape:

```rust
let contract = p.contract.as_ref().map(decode_contract).unwrap_or_default();
```

New shape:

```rust
let contract = p.contract.as_ref().map(decode_contract).transpose()?.unwrap_or_default();
```

The `?` propagates `Error::Parse` from a malformed combo-leg action up to the caller; the surrounding functions all already return `Result<_, Error>`, so this stays in-domain.

`decode_contract_details` at `proto/decoders.rs:433` becomes fallible too; its callers in `contracts/common/decoders/mod.rs:89` are already in `Result` context.

**Per CLAUDE.md rule 16**, silently defaulting `LegAction` on a malformed wire would mask incomplete TWS responses (the OrderStatus bug class). Going strict is the contract; the blast radius is mechanical.

### Considered and rejected: `decode_contract_or_default` helper

A helper to abbreviate the 9 callsites:

```rust
pub(crate) fn decode_contract_or_default(opt: Option<&proto::Contract>) -> Result<Contract, Error> {
    Ok(opt.map(decode_contract).transpose()?.unwrap_or_default())
}
```

**Rejected because:**
- `.map(F).unwrap_or_default()` is a stable Rust idiom; the `.transpose()?` insertion is mechanical at every site
- 9 callsites isn't enough churn to justify a domain-specific shim that adds an indirection layer
- A named helper hides which fallback is used (`Contract::default()`) — worse for `git grep` audits of silent fallbacks

The "Macro out repeated trait impls" pattern (`feedback_macro_repeated_trait_impls.md`) applies to *shape-identical impls*; here we have shape-identical function calls, which the stdlib chain already abbreviates. Leave the 9 callsites as direct `.map(decode_contract).transpose()?.unwrap_or_default()`.

---

## Concrete change sketches

### `parse_required` / `parse_optional` in `proto/decoders.rs`

Place above the existing `parse_order_status` (around line 39).

```rust
/// Shape for required wire enums (the OrderStatusKind / LegAction pattern).
pub(crate) fn parse_required<T>(opt: &Option<String>, label: &str) -> Result<T, Error>
where
    T: std::str::FromStr<Err = Error>,
{
    match opt.as_deref() {
        Some(s) if !s.is_empty() => s.parse(),
        _ => Err(Error::Parse(0, String::new(), format!("missing {label}"))),
    }
}

/// Shape for optional / filter wire enums (PR 2/3/4 will consume this).
pub(crate) fn parse_optional<T>(opt: &Option<String>) -> Result<Option<T>, Error>
where
    T: std::str::FromStr<Err = Error>,
{
    match opt.as_deref() {
        Some(s) if !s.is_empty() => s.parse().map(Some),
        _ => Ok(None),
    }
}
```

**Delete `parse_order_status`** (a 1-line wrapper would just hide `parse_required<OrderStatusKind>` behind a non-uniform name) and inline at its 2 callers:

```rust
// proto/decoders.rs:363  (decode_order_state)
status: parse_required(&proto.status, "OrderStatus")?,

// orders/common/decoders/mod.rs:57
status: crate::proto::decoders::parse_required(&p.status, "OrderStatus")?,
```

`OrderStatusKind` is inferred from the assignment target, so no turbofish is needed. This matches the shape PR 2/3/4/5 will use (`parse_optional(&proto.right)?`, etc.) — uniformity across the sweep > one extra named helper.

### `LegAction` updates in `contracts/types.rs`

```rust
/// Trading action for spread/combo legs. Mirrors the IBKR wire vocabulary
/// `BUY` / `SELL` / `SSHORT`. `SLONG` is not accepted on combo legs (see
/// `EClient.cs:1289` — only the SSHORT gate exists).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LegAction {
    /// Buy the leg.
    #[default]
    Buy,
    /// Sell the leg.
    Sell,
    /// Short-sell the leg. Gated by `MIN_SERVER_VER_SSHORT_COMBO_LEGS` (147),
    /// well below our floor of 210.
    SellShort,
}

impl LegAction {
    /// Return the canonical IB wire string (`"BUY"` / `"SELL"` / `"SSHORT"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            LegAction::Buy => "BUY",
            LegAction::Sell => "SELL",
            LegAction::SellShort => "SSHORT",
        }
    }
}

impl fmt::Display for LegAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for LegAction {
    type Err = crate::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "BUY" => Self::Buy,
            "SELL" => Self::Sell,
            "SSHORT" => Self::SellShort,
            other => return Err(crate::Error::Parse(0, other.to_string(), "unknown LegAction".into())),
        })
    }
}

impl crate::ToField for LegAction {
    fn to_field(&self) -> String {
        self.to_string()  // delegate to Display, matching Action / TimeInForce
    }
}
```

### `ComboLeg` in `contracts/mod.rs`

```rust
pub struct ComboLeg {
    pub contract_id: i32,
    pub ratio: i32,
    /// The side (buy / sell / sshort) of the leg.
    pub action: LegAction,  // was String
    pub exchange: String,
    pub open_close: ComboLegOpenClose,
    pub short_sale_slot: i32,
    pub designated_location: String,
    pub exempt_code: i32,
}
```

`#[derive(Default)]` will pick `LegAction::Buy` automatically via `#[default]`. No manual `Default` impl needed.

Add `use crate::contracts::types::LegAction;` to the imports at the top of `contracts/mod.rs` (already exists, verify).

### `encode_combo_leg` in `proto/encoders.rs:117`

```rust
fn encode_combo_leg(leg: &contracts::ComboLeg, per_leg_price: Option<f64>) -> proto::ComboLeg {
    proto::ComboLeg {
        // ...
        action: Some(leg.action.to_string()),  // was: some_str(&leg.action)
        // ...
    }
}
```

The wire field is required; we always emit `Some(...)`. The previous code happened to do the same since `LegAction`'s `Display` is always non-empty, but `some_str` would have dropped `""` if a future caller stored an empty string — irrelevant once the field is typed.

### `decode_combo_leg` in `proto/decoders.rs:91`

```rust
pub fn decode_combo_leg(proto: &proto::ComboLeg) -> Result<ComboLeg, Error> {
    Ok(ComboLeg {
        contract_id: proto.con_id.unwrap_or_default(),
        ratio: proto.ratio.unwrap_or_default(),
        action: parse_required::<LegAction>(&proto.action, "LegAction")?,
        exchange: s(&proto.exchange),
        open_close: ComboLegOpenClose::from(proto.open_close.unwrap_or_default()),
        short_sale_slot: proto.short_sales_slot.unwrap_or_default(),
        designated_location: s(&proto.designated_location),
        exempt_code: proto.exempt_code.unwrap_or_default(),
    })
}
```

`decode_contract` line 85: `combo_legs: proto.combo_legs.iter().map(decode_combo_leg).collect::<Result<Vec<_>, _>>()?,` and the function signature becomes `Result<Contract, Error>`.

---

## Sibling test files

### `src/proto/decoders_tests.rs` (new)

Wire via `#[cfg(test)] #[path = "decoders_tests.rs"] mod tests;` at the bottom of `src/proto/decoders.rs` (per CLAUDE.md rule 8).

**Helper-level** (uses `OrderStatusKind` as the bound-satisfying test enum — already implements `FromStr<Err = Error>`; no new test-only types):

- `parse_required(&None, "X")` → `Err(Error::Parse)`
- `parse_required(&Some(String::new()), "X")` → `Err(Error::Parse)` and label appears in the message
- `parse_required(&Some("Submitted".into()), "OrderStatus")` → `Ok(OrderStatusKind::Submitted)`
- `parse_required(&Some("Garbage".into()), "OrderStatus")` → `Err(Error::Parse)` (propagated from `FromStr`)
- `parse_optional::<OrderStatusKind>(&None)` → `Ok(None)`
- `parse_optional::<OrderStatusKind>(&Some(String::new()))` → `Ok(None)`
- `parse_optional(&Some("Filled".into()))` → `Ok(Some(OrderStatusKind::Filled))`
- `parse_optional::<OrderStatusKind>(&Some("Garbage".into()))` → `Err(Error::Parse)`

**Production-decoder level** (CLAUDE.md rule 10 — exercise `decode_combo_leg`, not just the helper it calls):

```rust
fn proto_leg(action: Option<&str>) -> proto::ComboLeg {
    proto::ComboLeg {
        con_id: Some(1),
        ratio: Some(1),
        action: action.map(str::to_string),
        ..Default::default()
    }
}

#[test]
fn decode_combo_leg_rejects_missing_action() {
    assert!(matches!(decode_combo_leg(&proto_leg(None)), Err(Error::Parse(_, _, _))));
}

#[test]
fn decode_combo_leg_rejects_empty_action() {
    assert!(matches!(decode_combo_leg(&proto_leg(Some(""))), Err(Error::Parse(_, _, _))));
}

#[test]
fn decode_combo_leg_rejects_unknown_action() {
    assert!(matches!(decode_combo_leg(&proto_leg(Some("SLONG"))), Err(Error::Parse(_, _, _))));
}

#[test]
fn decode_combo_leg_accepts_sshort() {
    let leg = decode_combo_leg(&proto_leg(Some("SSHORT"))).unwrap();
    assert_eq!(leg.action, LegAction::SellShort);
}
```

The unknown-action test specifically uses `"SLONG"` to guard against a future "let's just reuse `Action` after all" regression — `SLONG` is the variant `LegAction` deliberately excludes.

### `src/contracts/types_tests.rs` additions

Update the existing `LegAction` block (currently 4 asserts at lines 118-121). Replace with a table-driven test per CLAUDE.md rule 21 (derive expectations from the constant, not hardcoded strings):

```rust
#[test]
fn lega_ction_display_round_trip() {
    use std::str::FromStr;
    for variant in [LegAction::Buy, LegAction::Sell, LegAction::SellShort] {
        let wire = variant.as_str();
        assert_eq!(variant.to_string(), wire);
        assert_eq!(LegAction::from_str(wire).unwrap(), variant);
    }
}

#[test]
fn leg_action_from_str_rejects_unknown() {
    use std::str::FromStr;
    assert!(matches!(LegAction::from_str("INVALID"), Err(crate::Error::Parse(_, _, _))));
    assert!(matches!(LegAction::from_str(""), Err(crate::Error::Parse(_, _, _))));
    assert!(matches!(LegAction::from_str("buy"), Err(_)));  // case-sensitive
}

#[test]
fn leg_action_default_is_buy() {
    assert_eq!(LegAction::default(), LegAction::Buy);
}
```

Existing `as_str()` test (`types_tests.rs:118-121`) is now redundant with the round-trip — delete it.

---

## Cross-cutting checks

Per CLAUDE.md "Quick Commands" + parent plan §"Cross-cutting checks":

- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings` (default-async)
- [ ] `cargo clippy --all-targets --features sync -- -D warnings`
- [ ] `cargo clippy --all-features`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` × 3 feature configs
- [ ] `just test`
- [ ] `cargo build -p ibapi-integration-sync --tests` + async
- [ ] `just cover` — confirm touched modules (`contracts/types.rs`, `proto/decoders.rs`, `contracts/mod.rs`) stay ≥90% line coverage
- [ ] Grep `README.md` / `docs/*.md` / module rustdoc for `combo_leg`, `ComboLeg`, `.action = "`; verify each remaining match still compiles (manual mental compile per rule on `.md` rot)
- [ ] `docs/migration-3.0.md` §9 added in the same PR

---

## Open verification before opening the PR

1. **`MIN_SERVER_VER_SSHORT_COMBO_LEGS`**: parent plan cites this is below our floor of 210. Confirm against `/Users/wboayue/projects/tws-api/source/csharpclient/client/MinServerVer.cs` so the `SSHORT` variant is unconditionally accepted (no version gate needed in the decoder).
2. **`SLONG` exclusion**: confirm there's no `SLONG`-gated check on combo legs in `EClient.cs` around line 1289 (parent plan's claim).
3. **Wire fixtures**: search for any captured-wire fixture that emits a combo-leg `action` value not in `{BUY, SELL, SSHORT}` (e.g. `SLONG`, lowercase variants). If found, that fixture either needs deletion or proves the vocabulary is wider than the plan says — re-scope before merging.
4. **`testdata/builders/orders.rs:474, :650`**: confirm these are `Order.action` (the field typed as `Action`, not `LegAction`) and not affected by this PR. They look like outer order action based on the surrounding fixture shape, but verify by reading.

---

## Migration note for `docs/migration-3.0.md` §9 (draft)

```markdown
### 9. `ComboLeg.action` typed as `LegAction`

`ComboLeg.action` was `String` in 2.x. In 3.0 it is typed as `LegAction`, a strict 3-variant enum (`Buy`, `Sell`, `SellShort`) matching IBKR's combo-leg wire vocabulary. `LegAction` already existed as the `SpreadBuilder::add_leg(_, LegAction)` parameter type; 3.0 reuses it as the struct field.

`SLONG` is intentionally excluded — combo legs do not accept it (only `MIN_SERVER_VER_SSHORT_COMBO_LEGS` is gated in the C# reference; no `SLONG` gate exists for combo legs). If you need long-undelivered semantics, that's outer `Order.action: Action::SellLong`, not a combo leg.

```rust,ignore
// v2.x
let leg = ComboLeg {
    contract_id: 12345,
    action: "BUY".to_string(),
    ..Default::default()
};

// v3.0
let leg = ComboLeg {
    contract_id: 12345,
    action: LegAction::Buy,
    ..Default::default()
};
```

`LegAction` implements `Display` (`"BUY"` / `"SELL"` / `"SSHORT"`) and `FromStr<Err = Error>`. The decoder propagates `Error::Parse` if TWS sends an empty or unknown action — silent defaults are off the table.
```

---

## Out of scope

- `OrderState.completed_status` — verified free-form by audit (parent plan), stays `String`.
- `Execution.side`, `Contract.right`, `Contract.security_id_type`, `ExecutionFilter.side` — separate PRs (2, 3, 4, 5).
- `Action.from(...)` → `FromStr` migration — `Action` already has `Display`; its `from(&str)` panics with `todo!()` on unknown input (`orders/mod.rs:700`). That's a separate cleanup; not blocking PR 1.
