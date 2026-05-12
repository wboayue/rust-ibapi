# Typed-Status Sweep — PR 2 implementation plan

**Parent:** [typed-status-sweep.md](typed-status-sweep.md) §"PR 2 — `Contract.right: Option<OptionRight>`".

**Scope:** type `Contract.right: String` → `Option<OptionRight>`. `OptionRight` already exists in `src/contracts/types.rs:208` — PR 2 extends it (`FromStr`, `Hash`, `Serialize`, `Deserialize`, `#[non_exhaustive]`, `ToField`), promotes the field to typed, and migrates every call site.

**Plan corrections vs. parent plan:**

1. **`Contract::option` takes `OptionRight` directly, not `&str`.** The parent plan reads "keeps `&str` parameter (parsed via `FromStr`) for ergonomics" — that would force an `.expect()` inside a non-fallible constructor, exactly the rule-21 silent-default trap. Matching PR 1's `LegAction::Buy` precedent (`Contract::option("AAPL", "20240119", 150.0, OptionRight::Call)`) keeps the typed surface uniform. 4 in-repo call sites to migrate; trivial breakage.

2. **`ContractBuilder::right(impl Into<String>)` becomes `right(OptionRight)`.** Same rationale — no `From<&str>`, no silent defaults. The builder's `to_uppercase()` + `"P"/"C"` runtime validation (`contract_builder/mod.rs:472-475`) becomes structurally unreachable and gets deleted.

3. **Modernize touched module (rule 9): extract the inline `#[cfg(test)] mod tests` block at `src/contracts/mod.rs:957-1177` to a sibling `mod_tests.rs`.** ~220 lines of inline test code in a file we already need to edit; the tests reference `.right` and need updates anyway. Wire via `#[cfg(test)] #[path = "mod_tests.rs"] mod tests;` from `mod.rs` per CLAUDE.md rule 8.

---

## Files touched

| File | Change |
|---|---|
| `src/contracts/types.rs` | Extend `OptionRight`: add `#[non_exhaustive]`, `Hash`, `Serialize`, `Deserialize` to the derives (keep `Debug, Clone, Copy, PartialEq, Eq`); **do not** add `Default` — `Option<OptionRight>::None` is the no-right state. `impl FromStr<Err = Error>` accepting `"C"`, `"CALL"`, `"P"`, `"PUT"` (case-sensitive — match `LegAction`'s precedent; the legacy "c/p" tolerance was a stringly-typed band-aid). Add `impl ToField` delegating to `Display`. Keep existing `as_str()` / `Display`. |
| `src/contracts/types_tests.rs` | Replace the existing 4-line `option_right_str_and_display` (lines 109-114) with a table-driven round-trip per CLAUDE.md rule 21 (derive expected strings from `as_str()`, don't hardcode `"C"`/`"P"` a second time). Add: unknown-wire → `Err(Error::Parse)`; `FromStr` accepts both `"C"`/`"CALL"` and `"P"`/`"PUT"`; `FromStr` rejects `""`/lowercase. |
| `src/contracts/mod.rs` | `pub right: String` → `pub right: Option<OptionRight>` (struct field, line 184); `Contract::default()` sets `right: None` (line 229); `Contract::option(symbol, exp, strike, right: OptionRight)` — break the 4th param (line 544); update the doc-example at lines 530, 537, 540 to use `OptionRight::Call`. |
| `src/contracts/mod_tests.rs` *(new sibling, lifted from inline `mod tests`)* | Per CLAUDE.md rule 8/9: extract the entire `#[cfg(test)] mod tests { ... }` block at `mod.rs:957-1177` into a sibling file with `use super::*;`. Update right-related assertions: `assert_eq!(call.right, "C")` → `assert_eq!(call.right, Some(OptionRight::Call))` (lines 981, 987, 1015); `Contract::option(..., "C")` → `Contract::option(..., OptionRight::Call)` (lines 1011, 1128). The implementation file becomes `#[cfg(test)] #[path = "mod_tests.rs"] mod tests;`. |
| `src/proto/decoders.rs` | `decode_contract` line 89: `right: s(&proto.right)` → `right: parse_optional(&proto.right)?` (using the helper landed in PR 1, drop its `#[allow(dead_code)]`). |
| `src/proto/encoders.rs` | `encode_contract_with_order` line 75: `right: some_str(&contract.right)` → `right: contract.right.as_ref().map(|r| r.to_string())`. Empty wire (`None`) emits `None` on the proto side — matches the existing `some_str` drop behavior. |
| `src/contracts/builders.rs` | Line 207 `OptionBuilder::build`: `right: self.right.to_string()` → `right: Some(self.right)` (the `OptionBuilder` field is already typed `OptionRight` at line 73 — just lift it through). |
| `src/contracts/builders/tests.rs` | Lines 36, 56: `assert_eq!(call.right, "C")` → `assert_eq!(call.right, Some(OptionRight::Call))`; same for Put. Lines 283, 284 (existing `OptionRight::Call.to_string()` asserts) are now redundant with `types_tests.rs` table — delete. |
| `src/contracts/common/contract_builder/mod.rs` | Line 86: `right: Option<String>` → `right: Option<OptionRight>`. Line 193: `pub fn right<S: Into<String>>(mut self, right: S)` → `pub fn right(mut self, right: OptionRight)` (drop the generic). Lines 471-476: delete the `to_uppercase()` / `"P"/"C"` validation block — structurally unrepresentable now (`right.is_none()` check at line 467 stays). Line 495: `right: self.right.unwrap_or_default()` → `right: self.right` (already `Option<OptionRight>`). Doc comments at lines 40, 75, 78, 177, 190-192, 358, 446 update from `"C"/"P"` strings to `OptionRight::Call/Put`. |
| `src/contracts/common/contract_builder/tests.rs` | Mechanical replace of `.right("C")` → `.right(OptionRight::Call)` (lines 24, 116, 189, 213, 275, 408); `assert_eq!(builder.right, Some("C".to_string()))` → `assert_eq!(builder.right, Some(OptionRight::Call))` (line 43); `assert_eq!(contract.right, "")` → `assert_eq!(contract.right, None)` (lines 107, 373); `assert_eq!(contract.right, "C")` → `assert_eq!(contract.right, Some(OptionRight::Call))` (line 126); destructure `right` in `setter_parity_with_contract_fields` at line 446 — assertion updates from `assert_eq!(right, "C")` to `assert_eq!(right, Some(OptionRight::Call))` at line 469. **Delete** `test_contract_builder_build_invalid_option_right` (lines 242-254) — runtime "INVALID" string is now structurally impossible; the test guards a removed code path. **Delete** `test_contract_builder_build_valid_option_rights` (lines 257-269) — the `["P", "C", "p", "c"]` matrix tested case-insensitive `&str` acceptance, which no longer exists. A rewritten `[OptionRight::Call, OptionRight::Put]` loop would add nothing — `types_tests.rs::option_right_display_round_trip` covers the variants and `test_contract_builder_build_option_success` covers builder pass-through. |
| `src/contracts/common/test_tables.rs` | Lines 506, 527: `right: "C".to_string()` → `right: Some(OptionRight::Call)` (struct-literal fixture for `ContractDetails`). |
| `src/accounts/common/decoders/mod.rs` | Lines 32, 68, 206 (text-protocol decoder path): `position.contract.right = msg.next_string()?;` → `let right_str = msg.next_string()?; position.contract.right = parse_optional(&Some(right_str))?;` (or extract a one-line `parse_optional_str(&right_str)?` if a 4th call site shows up in PR 3; for now inline is fine — 3 sites, mechanical). Add `use crate::proto::decoders::parse_optional;` to imports. |
| `src/accounts/common/decoders/tests.rs` | Lines 27, 61, 96, 125, 252, 278, 373, 418, 463, 508, 553, 598, 643, 688, 733, 762 — fixtures and assertions over `expected_right: &'static str` shape. Convert table column to `expected_right: Option<OptionRight>`, assert with `result.contract.right == Some(OptionRight::Put)`. Text-fixture rows still emit `"P"` over the wire (string field in the response stream); only the typed expectation changes. |
| `src/testdata/builders/{orders,positions}.rs` | **Leave unchanged.** These builders write `proto::Contract.right` as `Option<String>` to the wire (proto field is `Option<String>`); the decoder side is what changes. Verified: each `right` reference in these files is `proto::Contract.right: Some(self.right.clone())` — wire-level, not Rust-level Contract field. |
| `examples/sync/calculate_implied_volatility.rs` | Line 17: `Contract::option("AAPL", "20250620", 240.0, "C")` → `Contract::option("AAPL", "20250620", 240.0, OptionRight::Call)`. Add `use ibapi::contracts::OptionRight;`. |
| `examples/async/last_trade_date_check.rs` | Line 21: `println!("  right: {}", d.contract.right)` → `println!("  right: {}", d.contract.right.map_or(String::new(), |r| r.to_string()))` or `{:?}` (the printed string format is informational — pick the form that least disrupts existing log shape). |
| `docs/examples.md` | Line 156: `Contract::option("AAPL", "20240119", 150.0, "C")` → `Contract::option("AAPL", "20240119", 150.0, OptionRight::Call)`. Add the `OptionRight` import to the snippet's `use` line (if shown). |
| `docs/contract-builder.md` | Lines 364-365 already show `OptionRight::Call` / `OptionRight::Put` — verify the surrounding builder snippet uses `.right(OptionRight::Call)` (not `.right("C")`). |
| `docs/migration-3.0.md` | Add §"10. `Contract.right` typed as `Option<OptionRight>`" — see "Migration note" section below. |
| `README.md` | Re-grep for `right`, `Contract::option`, `.right(` — no current snippets touch this (verified `grep -n right README.md` empty), but re-verify before opening the PR per CLAUDE.md rule on `.md` snippet rot. |

---

## Concrete change sketches

### `OptionRight` updates in `contracts/types.rs`

```rust
/// Option right (Call or Put). Matches IBKR's wire vocabulary `"C"` / `"P"`
/// (canonical); `"CALL"` / `"PUT"` historical variants are accepted on parse.
///
/// Unlike [`LegAction`], `OptionRight` does **not** derive `Default`:
/// `Contract.right` is meaningful only when `security_type == Option`, and
/// the field is typed `Option<OptionRight>` — `None` is the no-right state.
/// A `Default` impl would invite `OptionRight::default()` and silently pick
/// `Call`.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum OptionRight {
    /// Call option right.
    Call,
    /// Put option right.
    Put,
}

impl OptionRight {
    /// Return the canonical single-character wire string (`"C"` or `"P"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            OptionRight::Call => "C",
            OptionRight::Put => "P",
        }
    }
}

impl fmt::Display for OptionRight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for OptionRight {
    type Err = crate::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "C" | "CALL" => Self::Call,
            "P" | "PUT" => Self::Put,
            other => return Err(crate::Error::Parse(0, other.to_string(), "unknown OptionRight".into())),
        })
    }
}

impl crate::ToField for OptionRight {
    fn to_field(&self) -> String {
        self.to_string()
    }
}
```

**Why no `Default`:** unlike `LegAction` (every combo leg has an action), `Contract.right` is meaningful only for options. The `Option<OptionRight>` wrapper carries the "no right" state via `None`; deriving `Default` on the enum would invite callers to construct `OptionRight::default()` and silently pick `Call`.

**Why `"CALL"`/`"PUT"` accepted:** the IBKR docs and historical wire format used both forms. Accepting both on parse without `to_uppercase()` keeps the surface predictable (rejects `"call"` / `"Call"`) while preserving the historical compatibility note in `Contract.right`'s rustdoc at `mod.rs:183`.

### `Contract.right` field + `Contract::option` constructor in `contracts/mod.rs`

```rust
pub struct Contract {
    // ...
    /// Option type (only meaningful when `security_type == SecurityType::Option`).
    /// `None` on non-option contracts. Wire values are `"C"` (Call) and `"P"` (Put);
    /// historical `"CALL"` / `"PUT"` forms are accepted on parse.
    pub right: Option<OptionRight>,  // was: pub right: String
    // ...
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            // ...
            right: None,  // was: right: String::new()
            // ...
        }
    }
}

impl Contract {
    /// Creates a simple option contract.
    ///
    /// # Arguments
    /// * `symbol` - Symbol of the underlying asset
    /// * `expiration_date` - Expiration date of option contract (YYYYMMDD)
    /// * `strike` - Strike price of the option contract
    /// * `right` - `OptionRight::Call` or `OptionRight::Put`
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::contracts::{Contract, OptionRight, Symbol};
    ///
    /// let call = Contract::option("AAPL", "20240119", 150.0, OptionRight::Call);
    /// assert_eq!(call.symbol, Symbol::from("AAPL"));
    /// assert_eq!(call.strike, 150.0);
    /// assert_eq!(call.right, Some(OptionRight::Call));
    /// ```
    pub fn option(symbol: &str, expiration_date: &str, strike: f64, right: OptionRight) -> Contract {
        Contract {
            symbol: symbol.into(),
            security_type: SecurityType::Option,
            exchange: "SMART".into(),
            currency: "USD".into(),
            last_trade_date_or_contract_month: expiration_date.into(),
            strike,
            right: Some(right),
            ..Default::default()
        }
    }
}
```

### Decoder/encoder updates in `proto/decoders.rs` / `proto/encoders.rs`

```rust
// proto/decoders.rs:89  (decode_contract)
right: parse_optional(&proto.right)?,  // was: right: s(&proto.right)
```

```rust
// proto/encoders.rs:75  (encode_contract_with_order)
right: contract.right.as_ref().map(|r| r.to_string()),  // was: some_str(&contract.right)
```

Symmetric to `some_str(&str)` — `None` and empty wire are equivalent on the IB protocol, so `Option<OptionRight>::None` maps to `proto.right: None`. Drop `#[allow(dead_code)]` on `parse_optional` (`proto/decoders.rs:56`) — PR 2 is its first consumer.

### `ContractBuilder` updates in `contracts/common/contract_builder/mod.rs`

```rust
pub struct ContractBuilder {
    // ...
    pub(crate) right: Option<OptionRight>,  // was: Option<String>
    // ...
}

impl ContractBuilder {
    /// Sets the option right (Call or Put).
    ///
    /// Required for option contracts.
    pub fn right(mut self, right: OptionRight) -> Self {
        self.right = Some(right);
        self
    }

    pub fn build(self) -> Result<Contract, Error> {
        // ...
        if security_type == SecurityType::Option || security_type == SecurityType::FuturesOption {
            // ... strike check ...

            if self.right.is_none() {
                return Err(Error::Simple("Right (OptionRight::Call or OptionRight::Put) is required for options".into()));
            }
            // Old to_uppercase() / "P"/"C" validation block deleted —
            // structurally unrepresentable once the field is typed.

            // ... expiration check ...
        }

        Ok(Contract {
            // ...
            right: self.right,  // was: self.right.unwrap_or_default()
            // ...
        })
    }
}
```

Update the `Right (P for PUT or C for CALL) is required for options` test assertion message at `contract_builder/tests.rs:207` to match the new error string.

### Text-decoder updates in `accounts/common/decoders/mod.rs`

```rust
// line 32 (decode_position)
let right_str = msg.next_string()?;
position.contract.right = parse_optional(&Some(right_str))?;
```

Three sites total (lines 32, 68, 206). Add to imports: `use crate::proto::decoders::parse_optional;`. **Note**: `parse_optional`'s API takes `&Option<String>` — wrapping in `Some(...)` here costs one allocation per decode, but the text path is legacy and being phased out per PR #527/#529/#531 ratchet. Not worth introducing a `parse_optional_str(&str)` sibling for 3 fading call sites.

### `accounts/common/decoders/tests.rs` table conversion

The big test table at lines 252-762 has an `expected_right: &'static str` column. Change to `expected_right: Option<OptionRight>` and update each row:

```rust
// before
expected_right: "P",
// after
expected_right: Some(OptionRight::Put),
```

Wire-side fixtures (the `right: &str` parameter in the row constructor at line 278) **stay** `&str` — they feed the text-protocol decoder's `msg.next_string()`. Only the **expected** column type changes.

---

## Sibling test files

### `src/contracts/types_tests.rs` (modify existing)

Replace the existing `option_right_str_and_display` (lines 109-114) with:

```rust
#[test]
fn option_right_display_round_trip() {
    use std::str::FromStr;
    for variant in [OptionRight::Call, OptionRight::Put] {
        let wire = variant.as_str();
        assert_eq!(variant.to_string(), wire);
        assert_eq!(format!("{variant}"), wire);
        assert_eq!(OptionRight::from_str(wire).unwrap(), variant);
    }
}

#[test]
fn option_right_from_str_accepts_historical_long_form() {
    use std::str::FromStr;
    assert_eq!(OptionRight::from_str("CALL").unwrap(), OptionRight::Call);
    assert_eq!(OptionRight::from_str("PUT").unwrap(), OptionRight::Put);
}

#[test]
fn option_right_from_str_rejects_unknown() {
    use std::str::FromStr;
    assert!(matches!(OptionRight::from_str("INVALID"), Err(crate::Error::Parse(_, _, _))));
    assert!(matches!(OptionRight::from_str(""), Err(crate::Error::Parse(_, _, _))));
    // Case-sensitive: lowercase must not match.
    assert!(matches!(OptionRight::from_str("c"), Err(crate::Error::Parse(_, _, _))));
    assert!(matches!(OptionRight::from_str("call"), Err(crate::Error::Parse(_, _, _))));
}

#[test]
fn option_right_to_field_matches_display() {
    use crate::ToField;
    for variant in [OptionRight::Call, OptionRight::Put] {
        assert_eq!(variant.to_field(), variant.to_string());
    }
}
```

The 4-line `option_right_str_and_display` test that hardcoded `"C"`/`"P"` strings four times disappears — round-trip + rule-21 derivation covers it.

### `src/contracts/mod_tests.rs` (new, lifted)

Move the entire `#[cfg(test)] mod tests { ... use super::*; ... }` block at `mod.rs:957-1177` into this file with `use super::*;` at the top. Replace the inline `#[cfg(test)] mod tests` declaration in `mod.rs` with:

```rust
#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
```

Update right-related assertions inside the extracted file:

- `mod_tests.rs:981`: `assert_eq!(call.right, "C")` → `assert_eq!(call.right, Some(OptionRight::Call))`
- `mod_tests.rs:987`: `assert_eq!(put.right, "P")` → `assert_eq!(put.right, Some(OptionRight::Put))`
- `mod_tests.rs:1011`: `Contract::option("AAPL", "20231215", 150.0, "C")` → `Contract::option("AAPL", "20231215", 150.0, OptionRight::Call)`
- `mod_tests.rs:1015`: same `right == Some(OptionRight::Call)` shape
- `mod_tests.rs:1128`: another `Contract::option(..., "C")` constructor call

(Line numbers post-extract may shift; verify after the mechanical move.)

---

## Decoder fallibility blast radius

`decode_contract` is already `Result<Contract, Error>` after PR 1 (PR 1 made it fallible to surface combo-leg `LegAction` parse errors). Adding `parse_optional` for `right` propagates through the same `?` — no new callsites need a fallibility upgrade.

If `parse_optional` returns `Err(Error::Parse(_, _, _))` for an unknown right (e.g. TWS sends `"X"`), the error flows up through every existing `decode_contract` caller — which all already handle `Result`. Verified during PR 1's 9-callsite blast-radius audit.

---

## Cross-cutting checks

Per CLAUDE.md "Quick Commands" + parent plan §"Cross-cutting checks":

- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings` (default-async)
- [ ] `cargo clippy --all-targets --features sync -- -D warnings`
- [ ] `cargo clippy --all-features`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` × 3 feature configs
- [ ] `just test` (× 3 configs implicit)
- [ ] `cargo build -p ibapi-integration-sync --tests` + async equivalent
- [ ] `just cover` — confirm touched modules (`contracts/types.rs`, `contracts/mod.rs`, `proto/decoders.rs`, `contracts/common/contract_builder/mod.rs`, `accounts/common/decoders/mod.rs`) stay ≥ 90% line coverage
- [ ] Grep `README.md` / `docs/*.md` / module rustdoc for `Contract::option`, `.right(`, `right:` — verify each remaining match still compiles (manual mental compile per `feedback_md_doc_snippets_rot_silently.md`)
- [ ] `docs/migration-3.0.md` §10 added in the same PR

---

## Open verification before opening the PR

1. **`"CALL"` / `"PUT"` historical wire**: confirm against `EDecoder.cs` and IBKR docs that the wire actually emits the long form anywhere — or whether `"C"`/`"P"` is the only form TWS uses. If only short form is observed in fixtures, consider dropping the long-form acceptance to match `LegAction`'s strict single-form pattern. (Inverse of the PR 1 pattern: `LegAction` rejected lowercase + `SLNG`; here we want to know whether `"CALL"` is real or just doc cruft.) Per CLAUDE.md rule 16 / `feedback_verify_wire_before_typing.md`: **grep `src/testdata/builders/` and `accounts/common/decoders/tests.rs` fixtures for any `right: "CALL"` or `"PUT"` wire string** — if none exist, drop the historical-form acceptance from `FromStr` and the matching test.
2. **`accounts/async/decoders.rs` (if exists)**: parallel async-path text decoders may have a sibling shape to `accounts/common/decoders/mod.rs:32, 68, 206`. Grep before opening the PR — symmetry matters per rule 4.
3. **`testdata/builders/{orders, positions}.rs:right`**: re-read each `right: some_str(&self.right)` line to confirm these write **wire** strings to `proto::Contract.right`, not the Rust `Contract.right` field. Plan claims they're unchanged; verify line by line.

---

## Migration note for `docs/migration-3.0.md` §10 (draft)

```markdown
### 10. `Contract.right` typed as `Option<OptionRight>`

`Contract.right` was `String` in 2.x (empty string meant "no right"). In 3.0 it is typed as `Option<OptionRight>` — `None` on non-option contracts, `Some(OptionRight::Call)` or `Some(OptionRight::Put)` on options. The decoder rejects unknown wire values as `Error::Parse` rather than silently storing them as raw strings.

`OptionRight` is `#[non_exhaustive]` and implements `Display` (`"C"` / `"P"`) and `FromStr<Err = Error>`. `FromStr` accepts both the canonical short form (`"C"` / `"P"`) and the historical long form (`"CALL"` / `"PUT"`); it is case-sensitive — lowercase forms now produce `Err`.

`Contract::option`'s 4th parameter changes from `&str` to `OptionRight`. `ContractBuilder::right()` changes from `impl Into<String>` to `OptionRight`. The builder's runtime "right must be P or C" validation has been removed — invalid rights are now structurally unrepresentable.

```rust,ignore
// v2.x
let call = Contract::option("AAPL", "20240119", 150.0, "C");
assert_eq!(call.right, "C");

let builder_call = ContractBuilder::option("AAPL", "SMART", "USD")
    .strike(150.0)
    .right("C")
    .build()?;

// v3.0
use ibapi::contracts::OptionRight;

let call = Contract::option("AAPL", "20240119", 150.0, OptionRight::Call);
assert_eq!(call.right, Some(OptionRight::Call));

let builder_call = ContractBuilder::option("AAPL", "SMART", "USD")
    .strike(150.0)
    .right(OptionRight::Call)
    .build()?;
```

If you're matching on the field, swap `if contract.right == "C"` for `if contract.right == Some(OptionRight::Call)`. The `as_str()` round-trip (`OptionRight::Call.as_str() == "C"`) is unchanged for code that needs to emit the wire string.
```

---

## Out of scope

- `Contract.security_id_type: String` → `Option<SecurityIdType>` — PR 3 (separate sweep entry).
- Renaming `OptionRight::as_str` to match other enums' patterns — already `as_str() -> &'static str`; consistent. No change.
- Deleting `Contract::option` in favor of `Contract::call(...).strike(...).expires(...).build()` — separate ergonomics decision, parent plan keeps `Contract::option` as the simple-case escape hatch.
- Text-protocol position decoder cleanup (`accounts/common/decoders/mod.rs`) — out of scope for this typing change; will be handled by the proto-cleanup ratchet (rule 19 / 20) once `position` goes proto-only.

### Design observation: three-way option constructor overlap

The typed-status sweep highlights an unresolved API decision: there are three ways to construct an option `Contract`:

1. `Contract::option(symbol, exp, strike, OptionRight::Call)` — legacy positional constructor.
2. `Contract::call(symbol).strike(...).expires_on(...).build()` — type-state `OptionBuilder` (the canonical fluent path).
3. `ContractBuilder::option(symbol, exch, ccy).strike(...).right(OptionRight::Call).last_trade_date_or_contract_month(...).build()?` — generic `ContractBuilder` for unusual cases.

Once `right` becomes typed, (1) loses its "easy `&str` shortcut" ergonomic argument over (2). The remaining differentiator is "positional 4-arg constructor vs. fluent chain" — a thin justification. **Flag for a future ergonomics PR**: does `Contract::option` still earn its place once strikes/expiries/rights all go typed, or should it be deprecated in favor of `Contract::call`/`put` + a generic catch-all? Not a blocker for PR 2; calling it out so the question doesn't get lost.

---

## Cross-cutting follow-ups

### Refactor `parse_optional` signature: `&Option<String>` → `Option<&str>`

`parse_optional` (landed in PR 1 at `src/proto/decoders.rs:57`) is the central composability hinge for the typed-status sweep. Its current signature takes `&Option<String>` — the shape the proto-derived `proto.right: Option<String>` field hands over directly. But text-protocol decoders work with `String` (from `msg.next_string()?`), so every text-side caller has to wrap: `parse_optional(&Some(right_str))?`. Three sites in this PR; PR 3/4/5 may add more.

Refactor to `Option<&str>`:

```rust
pub(crate) fn parse_optional<T>(opt: Option<&str>) -> Result<Option<T>, Error>
where T: std::str::FromStr<Err = Error>,
{
    match opt {
        Some(s) if !s.is_empty() => s.parse().map(Some),
        _ => Ok(None),
    }
}
```

Call sites:

- Proto path (`decoders.rs:89`): `parse_optional(proto.right.as_deref())?` — zero allocation.
- Text path (`accounts/common/decoders/mod.rs:32`): `parse_optional(Some(right_str.as_str()))?` — zero allocation, no `Some` wrapper allocation.
- Test path: `parse_optional(Some("CALL"))?` — natural.

**Recommendation: bundle into PR 2** since this is the first PR exercising both proto + text sides of the helper; the refactor pays for itself immediately. Apply the same change to `parse_required` for symmetry (its current 2 callers all sit on the proto side, so the change there is mechanical: `parse_required(proto.status.as_deref(), "OrderStatus")?`).

If the refactor feels too entangled with PR 2's typing work, split into a tiny precursor **PR 2a** that flips both helpers' signatures and migrates the existing PR 1 callers — then PR 2 lands the new `Contract.right` consumer on the cleaner shape.

### `Error::Parse(usize, String, String)` index slot

Tracked by the parent plan, restated here to keep the running count visible. After PR 2:

- PR 1 introduced 2 fake-`0` sites (`LegAction::from_str` + `parse_required`'s `Err` arm).
- PR 2 adds 1-2 more (`OptionRight::from_str`; helper changes if `parse_optional`'s refactor lands).
- PR 3/4/5 each add ~1.

Five accumulated fakes is the inflection point — decide shape before PR 5 (parent plan §"Cross-cutting follow-ups" and [`v3-api-ergonomics.md` §5](v3-api-ergonomics.md)).

---

## Rule references

- CLAUDE.md rule 8 — separate `_tests.rs` files (forces the `mod.rs` inline test extract).
- CLAUDE.md rule 9 — modernize touched modules (inline test extract in scope).
- CLAUDE.md rule 16 — typed-status migration pattern; reject empty/unknown as `Error::Parse`; verify wire before typing.
- CLAUDE.md rule 21 — derive test expectations from the constant.
- CLAUDE.md rule 23 — restrictive API additions split into modernize-callers + restrict; one field per PR.
- `feedback_md_doc_snippets_rot_silently.md` — verify `docs/*.md` snippets after rename.
- `feedback_unreachable_regression_guards.md` — delete `test_contract_builder_build_invalid_option_right` and `test_contract_builder_build_valid_option_rights` (case-insensitive) — both guard removed code paths.
