# Typed-Status Sweep — PR 4a implementation plan

**Parent:** [typed-status-sweep.md](typed-status-sweep.md) §"PR 4 — `ExecutionFilter.side: Option<ExecutionFilterSide>`" (split into 4a + 4b on the PR 3a/3b precedent).

**Scope:** zero behavior change. Promote three pieces of dedupe infrastructure introduced for `contracts/`-side typed-status work to crate-wide reach so PR 4b (`ExecutionFilterSide`) and PR 5b (`ExecutionSide`) inherit the shape:

1. `impl_wire_enum!` macro — currently module-local in `src/contracts/types.rs:45`.
2. `check_wire_enum_round_trip<T>` / `check_wire_enum_rejects_unknown<T>` test helpers — currently private in `src/contracts/types_tests.rs:112-134`.
3. `some_display(Option<&impl Display>) -> Option<String>` — new sibling of `some_str` in `src/proto/encoders.rs`. Three current call sites (`right`, `sec_id_type`, plus PR 4b's `side`) meet the rule-of-three tripwire; the PR 3b /simplify pass explicitly tracked this as a deferred follow-up.

Then retrofit the in-`orders/` hand-roll of `OrderStatusKind` (the PR 3a follow-up note's "adopt in a follow-up" item) and convert the two existing PR 2/PR 3b encoder sites to `some_display`.

**Why split off from 4b:** PR 3a/3b precedent in the same plan — infrastructure ships first, typing migration consumes it. Each PR stays focused and small. PR 5b inherits the entire shape with zero further infra work.

---

## Decisions

### Macro reach: `#[macro_use] mod macros;` at crate root

Project-convention break: there are currently **zero** `#[macro_export]` or `#[macro_use] mod` declarations in `src/` (audited: only module-local `macro_rules!` in `testdata/builders/mod.rs`, `proto/encoders.rs`, `contracts/types.rs`, `contracts/types_tests.rs`). Justify the break with the consumer count: 6 typed-wire-enum consumers when PR 4b + PR 5b land (3 in `contracts/types.rs` — `OptionRight`, `SecurityIdType`, `LegAction`; 3 in `orders/mod.rs` — `OrderStatusKind` retrofit, `ExecutionFilterSide`, `ExecutionSide`). Plus the orphan rule forecloses any blanket-impl alternative (rule 25's earned-cost case `(a)`).

`#[macro_export]` rejected: leaks `impl_wire_enum` / `impl_str_partial_eq` to the public crate API on docs.rs. The crate-internal `#[macro_use] mod macros;` shape keeps them `pub(crate)`-equivalent.

`impl_str_partial_eq!` moves alongside `impl_wire_enum!` — both currently colocated in `contracts/types.rs`, both crate-internal, same justification.

### Test helper home: `src/common/test_utils.rs::wire_enum`

`src/common/test_utils.rs` already hosts `#[cfg(test)] pub mod helpers { ... }` (`create_test_client` and friends). Add a sibling `pub mod wire_enum { ... }` inside the same `#[cfg(test)]` block. Both `contracts/types_tests.rs` and `orders/tests.rs` import via `crate::common::test_utils::wire_enum::check_wire_enum_round_trip`.

The helpers are already generic functions (rule 25's preferred shape over macros) — no changes to their bodies, just relocation + visibility bump.

### `some_display` signature

```rust
pub(crate) fn some_display<T: std::fmt::Display>(opt: Option<&T>) -> Option<String> {
    opt.map(|v| v.to_string())
}
```

`Option<&T>` (not `Option<T>`) so callers don't have to clone `Copy` enums into the helper. Existing `some_str(s: &str) -> Option<String>` drops empty strings; `some_display` does not — `Option::None` already carries the "no value" state at the type level (per PR 2/PR 3b shape), so empty-string filtering would be wrong here. Sibling, not replacement.

### `OrderStatusKind` retrofit

Drop the hand-rolled `Display` (`orders/mod.rs:747-761`), `FromStr` (`:763-779`). Add `as_str(&self) -> &'static str` + `fn from_wire(s: &str) -> Option<Self>`. Call `impl_wire_enum!(OrderStatusKind);`.

**Behavior delta:** the macro adds `impl ToField for OrderStatusKind` (currently absent — verified `grep "ToField for OrderStatusKind"` empty). New impl, no existing callers, no removal of public API. Zero behavior change for current callers.

Retrofit `orders/tests.rs:5-35`:

- Delete the `ALL_KINDS: &[(OrderStatusKind, &str)]` table at lines 5-15.
- Replace `order_status_kind_from_str_round_trips_for_all_variants` (lines 17-26) with a one-liner calling `check_wire_enum_round_trip(&[(OrderStatusKind::ApiPending, "ApiPending"), ...])`. The literal table stays — it's the **data** under test, not the loop boilerplate.
- Replace `order_status_kind_from_str_rejects_unknown_status` (lines 28-35) with `check_wire_enum_rejects_unknown::<OrderStatusKind>(&["NotARealStatus", ""])`.
- `is_active_and_is_terminal_partition_eight_of_nine_variants` (lines 37-59) is unrelated to the wire-enum shape — keep verbatim.

---

## Files touched

| File | Change |
|---|---|
| `src/macros.rs` *(new)* | Move `impl_str_partial_eq!` (from `contracts/types.rs:13-36`) and `impl_wire_enum!` (from `contracts/types.rs:45-64`) verbatim into a new top-level module file. Preserve the existing doc comments. No body changes. |
| `src/lib.rs` | Add `#[macro_use] mod macros;` near the top of the module declarations, **before** `pub mod contracts;` (currently line 102) and `pub mod orders;` (line 111). Place it after the `pub mod` family for stylistic consistency — Rust scope rule is "before consuming modules in the same parent", so any line < 102 works. Recommended slot: just before `pub mod accounts;` at line 42. |
| `src/contracts/types.rs` | Delete `impl_str_partial_eq!` (lines 11-36) and `impl_wire_enum!` (lines 38-64). The three `impl_wire_enum!(X)` invocations and the four `impl_str_partial_eq!(Y)` invocations downstream in the same file resolve via the now-crate-wide macros. No call-site changes. |
| `src/common/test_utils.rs` | Add a new `pub mod wire_enum { ... }` block inside the existing `#[cfg(test)] #[allow(dead_code)] pub mod helpers { ... }` parent — or as a sibling `#[cfg(test)] pub mod wire_enum { ... }` (cleaner; helpers stay isolated). Define `pub fn check_wire_enum_round_trip<T>(table: &[(T, &'static str)])` and `pub fn check_wire_enum_rejects_unknown<T>(unknowns: &[&str])` lifted verbatim from `contracts/types_tests.rs:112-134`. |
| `src/contracts/types_tests.rs` | Delete the two helper definitions at lines 112-134. Replace local references at lines 138, 145, 150, 157, 167, 179 with `crate::common::test_utils::wire_enum::check_wire_enum_round_trip` / `check_wire_enum_rejects_unknown` (or `use crate::common::test_utils::wire_enum::*;` at the top of the test mod). No assertion changes. |
| `src/proto/encoders.rs` | Add `pub(crate) fn some_display<T: std::fmt::Display>(opt: Option<&T>) -> Option<String> { opt.map(\|v\| v.to_string()) }` next to `some_str` (currently line 30). Convert the two existing call sites: line 75 `right: contract.right.as_ref().map(\|r\| r.to_string())` → `right: some_display(contract.right.as_ref())`; line 82 `sec_id_type: contract.security_id_type.as_ref().map(\|s\| s.to_string())` → `sec_id_type: some_display(contract.security_id_type.as_ref())`. Add a `test_some_display` unit test in the existing `#[cfg(test)] mod tests` block matching `test_some_str_empty`'s shape. |
| `src/orders/mod.rs` | Retrofit `OrderStatusKind` (lines 705-779): drop hand-rolled `Display` (`:747-761`) and `FromStr` (`:763-779`). In their place, add an `impl OrderStatusKind { fn as_str(&self) -> &'static str { ... } fn from_wire(s: &str) -> Option<Self> { ... } }` block (private helpers — `as_str` can stay `pub` if any current caller depends on it, audit first; `from_wire` is `fn`). Then `impl_wire_enum!(OrderStatusKind);` below. Keep the existing `is_active` / `is_terminal` impl block at lines 781-797 verbatim. |
| `src/orders/tests.rs` | Rewrite `order_status_kind_from_str_round_trips_for_all_variants` (lines 17-26) as a one-line call to `check_wire_enum_round_trip(&[(OrderStatusKind::ApiPending, "ApiPending"), ... ])`. Rewrite `order_status_kind_from_str_rejects_unknown_status` (lines 28-35) as a one-line call to `check_wire_enum_rejects_unknown::<OrderStatusKind>(&["NotARealStatus", ""])`. Delete the `ALL_KINDS` constant (lines 5-15) since the new table is local to the round-trip test. Keep `is_active_and_is_terminal_partition_eight_of_nine_variants` (lines 37-59) but rebuild its inner data from a freshly-declared inline `let all_kinds = [...]` slice to avoid the `ALL_KINDS` dependency. Add `use crate::common::test_utils::wire_enum::*;` to the imports. |

---

## Concrete change sketches

### `src/macros.rs` (new, verbatim move from `contracts/types.rs`)

```rust
//! Crate-internal macros for shape-identical trait impls.
//!
//! Reachable crate-wide via `#[macro_use] mod macros;` in `lib.rs`. Not exported
//! to the public API (no `#[macro_export]`).

/// Mirrors std `String`'s `PartialEq` ergonomics on a string-newtype:
/// `wrapper == "literal"` and `"literal" == wrapper` both work.
macro_rules! impl_str_partial_eq {
    ($t:ty) => {
        impl PartialEq<str> for $t {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }
        impl PartialEq<&str> for $t {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }
        impl PartialEq<$t> for str {
            fn eq(&self, other: &$t) -> bool {
                self == other.0
            }
        }
        impl PartialEq<$t> for &str {
            fn eq(&self, other: &$t) -> bool {
                *self == other.0
            }
        }
    };
}

/// Generate `Display` / `FromStr<Err = Error>` / `ToField` impls from
/// hand-written `as_str(&self) -> &'static str` + `from_wire(&str) -> Option<Self>`
/// methods. The data tables stay in normal Rust (visible to goto-def); only
/// the boilerplate plumbing — `Display` via `as_str`, `FromStr` via `from_wire`
/// with canonical `Error::Parse`, `ToField` via `Display` — runs through the
/// macro. Orphan rule blocks a blanket `impl<T: WireEnum> Display`, so a
/// macro is the only viable shape.
macro_rules! impl_wire_enum {
    ($name:ident) => {
        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
        impl ::std::str::FromStr for $name {
            type Err = $crate::Error;
            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                Self::from_wire(s).ok_or_else(|| $crate::Error::Parse(0, s.to_string(), concat!("unknown ", stringify!($name)).into()))
            }
        }
        impl $crate::ToField for $name {
            fn to_field(&self) -> String {
                self.to_string()
            }
        }
    };
}
```

### `src/lib.rs` insertion

```rust
// ... existing top of lib.rs ...

#[macro_use]
mod macros;

pub mod accounts;
// ... rest unchanged ...
```

### `src/common/test_utils.rs` addition

```rust
// ... existing helpers module ...

#[cfg(test)]
#[allow(dead_code)] // Used by per-module wire-enum tests
pub mod wire_enum {
    /// Assert `Display`, `FromStr`, and `ToField` agree on a hand-written
    /// `(variant, wire)` table. One helper covers every trait impl generated
    /// by `impl_wire_enum!` — independent verification (the table is not
    /// derived from `as_str()`, so a typo in either direction surfaces).
    pub fn check_wire_enum_round_trip<T>(table: &[(T, &'static str)])
    where
        T: Copy + std::fmt::Display + std::fmt::Debug + PartialEq + std::str::FromStr<Err = crate::Error> + crate::ToField,
    {
        for &(variant, wire) in table {
            assert_eq!(variant.to_string(), wire, "Display for {variant:?}");
            assert_eq!(T::from_str(wire).unwrap(), variant, "FromStr({wire})");
            assert_eq!(variant.to_field(), wire, "ToField for {variant:?}");
        }
    }

    pub fn check_wire_enum_rejects_unknown<T>(unknowns: &[&str])
    where
        T: std::str::FromStr<Err = crate::Error> + std::fmt::Debug,
    {
        for &s in unknowns {
            let err = T::from_str(s);
            assert!(
                matches!(err, Err(crate::Error::Parse(_, _, _))),
                "expected Parse error for {s:?}, got {err:?}",
            );
        }
    }
}
```

### `src/proto/encoders.rs` `some_display` + retrofits

```rust
// next to `some_str` at line ~30
pub(crate) fn some_display<T: std::fmt::Display>(opt: Option<&T>) -> Option<String> {
    opt.map(|v| v.to_string())
}
```

```rust
// line 75 — encode_contract_with_order
right: some_display(contract.right.as_ref()),
// line 82
sec_id_type: some_display(contract.security_id_type.as_ref()),
```

Unit test sibling to `test_some_str_empty`:

```rust
#[test]
fn test_some_display() {
    assert!(some_display::<u32>(None).is_none());
    assert_eq!(some_display(Some(&42_i32)), Some("42".to_string()));
}
```

### `OrderStatusKind` retrofit

```rust
// orders/mod.rs around line 705-779
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatusKind {
    // ... variants unchanged ...
}

impl OrderStatusKind {
    /// Return the canonical wire string.
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatusKind::ApiPending => "ApiPending",
            OrderStatusKind::PendingSubmit => "PendingSubmit",
            OrderStatusKind::PendingCancel => "PendingCancel",
            OrderStatusKind::PreSubmitted => "PreSubmitted",
            OrderStatusKind::Submitted => "Submitted",
            OrderStatusKind::ApiCancelled => "ApiCancelled",
            OrderStatusKind::Cancelled => "Cancelled",
            OrderStatusKind::Filled => "Filled",
            OrderStatusKind::Inactive => "Inactive",
        }
    }

    fn from_wire(s: &str) -> Option<Self> {
        match s {
            "ApiPending" => Some(Self::ApiPending),
            "PendingSubmit" => Some(Self::PendingSubmit),
            "PendingCancel" => Some(Self::PendingCancel),
            "PreSubmitted" => Some(Self::PreSubmitted),
            "Submitted" => Some(Self::Submitted),
            "ApiCancelled" => Some(Self::ApiCancelled),
            "Cancelled" => Some(Self::Cancelled),
            "Filled" => Some(Self::Filled),
            "Inactive" => Some(Self::Inactive),
            _ => None,
        }
    }
}

impl_wire_enum!(OrderStatusKind);

impl OrderStatusKind {
    pub fn is_active(self) -> bool { /* unchanged */ }
    pub fn is_terminal(self) -> bool { /* unchanged */ }
}
```

Net delta: ~5 lines removed (the hand-rolled Display + FromStr are ~30 lines; the new `as_str` + `from_wire` are ~25). Adds `ToField` (previously absent).

### `orders/tests.rs` rewrite

```rust
use super::*;
use crate::common::test_utils::wire_enum::{check_wire_enum_rejects_unknown, check_wire_enum_round_trip};

#[test]
fn order_status_kind_round_trip() {
    check_wire_enum_round_trip(&[
        (OrderStatusKind::ApiPending, "ApiPending"),
        (OrderStatusKind::PendingSubmit, "PendingSubmit"),
        (OrderStatusKind::PendingCancel, "PendingCancel"),
        (OrderStatusKind::PreSubmitted, "PreSubmitted"),
        (OrderStatusKind::Submitted, "Submitted"),
        (OrderStatusKind::ApiCancelled, "ApiCancelled"),
        (OrderStatusKind::Cancelled, "Cancelled"),
        (OrderStatusKind::Filled, "Filled"),
        (OrderStatusKind::Inactive, "Inactive"),
    ]);
}

#[test]
fn order_status_kind_from_str_rejects_unknown() {
    check_wire_enum_rejects_unknown::<OrderStatusKind>(&["NotARealStatus", "", "submitted", "FILLED"]);
}

#[test]
fn is_active_and_is_terminal_partition_eight_of_nine_variants() {
    let all_kinds = [
        (OrderStatusKind::ApiPending, "ApiPending"),
        (OrderStatusKind::PendingSubmit, "PendingSubmit"),
        // ... 9 entries inline ...
    ];
    for (kind, text) in all_kinds {
        // existing assertions unchanged
    }
}
```

---

## Cross-cutting checks

Per CLAUDE.md "Quick Commands":

- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo clippy --all-targets --features sync -- -D warnings`
- [ ] `cargo clippy --all-features`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` × 3 feature configs
- [ ] `just test` (× 3 configs implicit)
- [ ] `cargo build -p ibapi-integration-sync --tests` + async equivalent
- [ ] `just cover` — touched modules (`contracts/types.rs`, `orders/mod.rs`, `proto/encoders.rs`, `common/test_utils.rs`) should not regress
- [ ] Grep for `impl_wire_enum\!\|impl_str_partial_eq\!\|check_wire_enum_round_trip\|check_wire_enum_rejects_unknown` across the crate — verify every callsite resolves under the new locations

---

## Verification

Zero behavior change. The PR ships if:

1. All clippy + tests + rustdoc gates pass across all three feature configs.
2. `OrderStatusKind`'s public API (`Display`, `FromStr`, `is_active`, `is_terminal`) is unchanged. The new `as_str()` + `ToField` impl are additive.
3. The two converted encoder sites (`right`, `sec_id_type`) produce identical bytes — covered by existing `test_encode_*` round-trip tests; no new ones needed.

---

## Out of scope (defer to follow-up)

- Promoting `string_newtype_surface!` (`contracts/types_tests.rs:40`) to crate-wide reach. Currently single-consumer; doesn't meet the rule-25 threshold.
- Adding `pub(crate) fn some_display_into<T: Into<String>>` or similar variants. YAGNI until a callsite needs it.
- Renaming `as_str` to `as_wire` across all typed-status enums for naming uniformity. Pure rename; separate cosmetic PR if desired.
- `OrderStatusKind::as_str` visibility audit (currently the hand-rolled impl exposes nothing; the new `as_str` will be `pub` to match `OptionRight`/`SecurityIdType`/`LegAction`). Audit before opening the PR — if no external callers exist, `pub(crate)` is fine; if anyone does `.as_str()`, keep `pub`.

---

## Rule references

- CLAUDE.md rule 4 — composition over repetition (3 encoder sites → `some_display`).
- CLAUDE.md rule 9 — modernize touched modules (OrderStatusKind retrofit in scope).
- CLAUDE.md rule 23 — restrictive API additions split (precedent for PR 4a/4b separation).
- CLAUDE.md rule 25 — macros only when ordinary Rust can't express the pattern (justified here: orphan rule + 6 consumers).
- `feedback_macro_repeated_trait_impls` — N identical-shape impls → `macro_rules!`.
- `feedback_distillation_cadence` — apply duplication/SRP/composability at plan-time (PR 4a is the result of this pass on the original single-PR plan).
