# Typed-Status Sweep — PR 4b implementation plan

**Parent:** [typed-status-sweep.md](typed-status-sweep.md) §"PR 4 — `ExecutionFilter.side: Option<ExecutionFilterSide>`" (split into 4a + 4b on the PR 3a/3b precedent).

**Prereq:** [PR 4a](typed-status-sweep-pr4a.md) — promotes `impl_wire_enum!`, the `check_wire_enum_*` test helpers, and the `some_display` encoder helper to crate-wide reach. PR 4b consumes all three.

**Scope:** type `ExecutionFilter.side: String` → `Option<ExecutionFilterSide>`. New 2-variant `#[non_exhaustive] enum ExecutionFilterSide { Buy, Sell }` lives in `src/orders/mod.rs` next to `ExecutionFilter`. Uses the crate-wide `impl_wire_enum!` macro (from 4a) — no new hand-rolled trait impls.

---

## Why a new enum, not `Action`

`ExecutionFilter.side` accepts only `BUY` or `SELL` on the wire — confirmed against:
- C# reference `ExecutionFilter.cs:47-49`: `@brief The Contract's side (BUY or SELL)`.
- Our doc comment at `src/orders/mod.rs:1614`: identical.
- Encoder test fixture at `src/proto/encoders.rs:612`: `"BUY".to_string()`.

`Action` (the outbound order-side enum) includes `SellShort` and `SellLong` — neither valid on the filter. Reusing `Action` would let `filter.side = Some(Action::SellShort)` compile and silently mismatch at the server. A 2-variant `ExecutionFilterSide` makes the invalid filter states unrepresentable (CLAUDE.md rule 16).

Symmetric reasoning to PR 1 (`LegAction` vs `Action`): subset wire vocabulary ⇒ new enum, even when the variant names overlap.

---

## Asymmetry vs PR 2/PR 3b

`ExecutionFilter` is **outgoing-only** — TWS never sends back an `ExecutionFilter`. So:

- **No decoder call site to update.** `parse_required` / `parse_optional` (PR 1 helpers) are not exercised by this PR.
- **Encoder uses `some_display`** (from PR 4a). Mirrors `right` / `sec_id_type` pattern at `proto/encoders.rs:75`, `:82`.
- **`FromStr` is exercised only by tests** (round-trip + reject-unknown). Production code never parses a filter side from wire.

The macro from 4a still earns its place — `Display` runs in the encoder, `ToField` is a free additive impl, and `FromStr` powers the round-trip test.

---

## Files touched

| File | Change |
|---|---|
| `src/orders/mod.rs` | Add `#[non_exhaustive] pub enum ExecutionFilterSide { Buy, Sell }` next to `ExecutionFilter` (currently line 1597-1620). Derives `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize` (+ `utoipa::ToSchema` cfg-gated); **no `Default`** — `Option<ExecutionFilterSide>::None` carries no-filter state. `impl ExecutionFilterSide { pub fn as_str(&self) -> &'static str { ... } fn from_wire(s: &str) -> Option<Self> { ... } }`. Then `impl_wire_enum!(ExecutionFilterSide);`. Change `pub side: String` (line 1615) → `pub side: Option<ExecutionFilterSide>`. Update field doc to call out `None` = no filter and the `Action` distinction. `#[derive(Default)]` on `ExecutionFilter` still works (`Option<T>` defaults to `None`). |
| `src/proto/encoders.rs` | Line ~372 in `encode_execution_filter`: `side: some_str(&filter.side)` → `side: some_display(filter.side.as_ref())`. Update test `test_encode_execution_filter` at line 604: `side: "BUY".to_string()` → `side: Some(ExecutionFilterSide::Buy)`; **add** the missing `assert_eq!(proto.side.as_deref(), Some("BUY"))` assertion (currently absent — tighten while touched per rule 9). |
| `src/orders/sync/mod.rs` | Doc-example at lines 128-143: `side: "BUY".to_owned()` → `side: Some(ExecutionFilterSide::Buy)`. Add `use ibapi::orders::ExecutionFilterSide;` to the example's `use` lines. |
| `src/orders/async/mod.rs` | Line 145-146 `pub async fn executions(...)` has only a one-line `///` doc; no `# Examples` block. Add a mirror of the sync doc-example using the typed form (rule 18 — public API needs a doc-example; rule 9 — modernize touched modules). Example: ```ignore``` block constructing `ExecutionFilter { side: Some(ExecutionFilterSide::Buy), ..Default::default() }` and `for await on subscription.next() { ... }`. |
| `src/orders/sync/tests.rs` | Lines 316-335: both `filter` and `expected_filter` literals: `side: "BUY".to_owned()` → `side: Some(ExecutionFilterSide::Buy)`. |
| `src/orders/tests.rs` | Append `ExecutionFilterSide` tests using the 4a-promoted helpers: `check_wire_enum_round_trip(&[(ExecutionFilterSide::Buy, "BUY"), (ExecutionFilterSide::Sell, "SELL")])`; `check_wire_enum_rejects_unknown::<ExecutionFilterSide>(&["", "INVALID", "buy", "sell", "SSHORT", "SLONG", "BOT", "SLD"])`. The `SSHORT`/`SLONG` rejects confirm the `Action` distinction; `BOT`/`SLD` (Execution.side wire — PR 5b) rejects confirm field-scoped vocabulary. |
| `src/prelude.rs` | Lines 39 and 42: add `ExecutionFilterSide` next to `ExecutionFilter` in both the `#[cfg(feature = "async")]` and `#[cfg(all(feature = "sync", not(feature = "async")))]` re-exports. |
| `examples/sync/executions.rs` | Line 10: add `ExecutionFilterSide` to `use ibapi::orders::{ExecutionFilter, ExecutionFilterSide};`. Line 24: commented placeholder `// filter.side = side.to_owned();` → `// filter.side = Some(ExecutionFilterSide::Buy);`. |
| `docs/migration-3.0.md` | Add §"12. `ExecutionFilter.side` typed as `Option<ExecutionFilterSide>`" after the existing §11 `SecurityIdType` section. See draft below. |
| `README.md` | Re-grep for `ExecutionFilter\|filter.side` — no current snippets touch this (verified empty), but re-verify before opening the PR per `feedback_md_doc_snippets_rot_silently`. |

---

## Concrete change sketches

### `ExecutionFilterSide` definition in `orders/mod.rs`

```rust
/// Side filter on outbound execution requests. IBKR's wire vocabulary for
/// `ExecutionFilter.side` is `"BUY"` / `"SELL"` only.
///
/// Distinct from [`Action`] — the outbound order-side vocabulary includes
/// `SSHORT` / `SLONG`, neither of which is accepted on the filter. A subset
/// enum here makes invalid filter values unrepresentable.
///
/// No `Default` — [`ExecutionFilter::side: Option<ExecutionFilterSide>`]
/// carries the no-filter state via `None`. `#[non_exhaustive]` leaves
/// headroom for IBKR vocabulary additions.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExecutionFilterSide {
    /// Filter to buy fills only.
    Buy,
    /// Filter to sell fills only.
    Sell,
}

impl ExecutionFilterSide {
    /// Return the canonical IBKR wire string (`"BUY"` / `"SELL"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionFilterSide::Buy => "BUY",
            ExecutionFilterSide::Sell => "SELL",
        }
    }

    fn from_wire(s: &str) -> Option<Self> {
        match s {
            "BUY" => Some(Self::Buy),
            "SELL" => Some(Self::Sell),
            _ => None,
        }
    }
}

impl_wire_enum!(ExecutionFilterSide);
```

### `ExecutionFilter` field update in `orders/mod.rs`

```rust
pub struct ExecutionFilter {
    // ... unchanged fields ...
    /// Side filter — `None` for no filter, `Some(ExecutionFilterSide::Buy)`
    /// or `Some(ExecutionFilterSide::Sell)` to restrict the response.
    pub side: Option<ExecutionFilterSide>,
    // ... unchanged fields ...
}
```

`#[derive(Debug, Default)]` already on the struct continues to work — `Option<T>::default() == None`.

### Encoder update in `proto/encoders.rs`

```rust
// line ~372
side: some_display(filter.side.as_ref()),
```

`some_display` (from PR 4a) handles `None` → `None` and `Some(variant)` → `Some(variant.to_string())`. Display round-trips to the wire string via `impl_wire_enum!`.

### Sync doc-example update in `orders/sync/mod.rs`

```rust
/// # Examples
///
/// ```no_run
/// use ibapi::client::blocking::Client;
/// use ibapi::orders::{ExecutionFilter, ExecutionFilterSide};
///
/// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
///
/// let filter = ExecutionFilter {
///    side: Some(ExecutionFilterSide::Buy),
///    ..ExecutionFilter::default()
/// };
///
/// let subscription = client.executions(filter).expect("request failed");
/// for execution_data in &subscription {
///    println!("{execution_data:?}")
/// }
/// ```
```

### Async doc-example addition in `orders/async/mod.rs`

```rust
/// Requests current day's executions matching the filter.
///
/// # Examples
///
/// ```no_run
/// # async fn run() -> Result<(), ibapi::Error> {
/// use ibapi::Client;
/// use ibapi::orders::{ExecutionFilter, ExecutionFilterSide};
///
/// let client = Client::connect("127.0.0.1:4002", 100).await?;
/// let filter = ExecutionFilter {
///     side: Some(ExecutionFilterSide::Buy),
///     ..ExecutionFilter::default()
/// };
/// let mut subscription = client.executions(filter).await?;
/// while let Some(item) = subscription.next().await {
///     println!("{item:?}");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn executions(&self, filter: ExecutionFilter) -> Result<Subscription<Executions>, Error> {
```

(Verify the exact async `Subscription` consumer idiom against `feedback_stream_adapter_consume_form` and existing async examples before submitting — the example must use the consume form / pattern-match form, not the `(&mut sub).filter_data()` cast.)

### `orders/tests.rs` additions

```rust
// at the bottom of the file, after the OrderStatusKind tests
use super::ExecutionFilterSide;

#[test]
fn execution_filter_side_round_trip() {
    check_wire_enum_round_trip(&[
        (ExecutionFilterSide::Buy, "BUY"),
        (ExecutionFilterSide::Sell, "SELL"),
    ]);
}

#[test]
fn execution_filter_side_from_str_rejects_unknown() {
    // Empty + arbitrary; case-sensitive (lowercase rejected); Action variants
    // not accepted on the filter; Execution.side variants (BOT/SLD) also rejected.
    check_wire_enum_rejects_unknown::<ExecutionFilterSide>(&[
        "", "INVALID", "buy", "sell", "SSHORT", "SLONG", "BOT", "SLD",
    ]);
}
```

### `prelude.rs` update

```rust
// line 39 (default-async)
pub use crate::orders::{order_builder, Action, ExecutionFilter, ExecutionFilterSide, OrderUpdate, Orders, PlaceOrder};

// line 42 (sync-only)
pub use crate::orders::{order_builder, Action, ExecutionFilter, ExecutionFilterSide, OrderUpdate, Orders, PlaceOrder};
```

### `examples/sync/executions.rs` update

```rust
use ibapi::client::blocking::Client;
use ibapi::orders::{ExecutionFilter, ExecutionFilterSide};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let filter = ExecutionFilter {
        client_id: Some(32),
        ..Default::default()
    };
    // filter.account_code = account_code.to_owned();
    // filter.time = time.to_owned();
    // filter.symbol = symbol.to_owned();
    // filter.security_type = security_type.to_owned();
    // filter.exchange = exchange.to_owned();
    // filter.side = Some(ExecutionFilterSide::Buy);

    let client = Client::connect("127.0.0.1:4002", 100)?;

    let subscription = client.executions(filter)?;
    for execution in &subscription {
        println!("{execution:?}")
    }

    Ok(())
}
```

---

## Migration note for `docs/migration-3.0.md` §12 (draft)

```markdown
### 12. `ExecutionFilter.side` typed as `Option<ExecutionFilterSide>`

`ExecutionFilter.side` was `String` in 2.x (empty string meant "no filter"). In 3.0 it is typed as `Option<ExecutionFilterSide>` — `None` for no filter, `Some(ExecutionFilterSide::Buy)` or `Some(ExecutionFilterSide::Sell)` to restrict the response. The encoder rejects invalid filter values at compile time rather than letting them reach the server.

`ExecutionFilterSide` is `#[non_exhaustive]` and implements `Display` returning the canonical wire string (`"BUY"` / `"SELL"`) and `FromStr<Err = Error>`. `FromStr` is case-sensitive.

**Note: distinct from [`Action`].** `Action` covers the outbound order-side vocabulary (including `SellShort` / `SellLong`), neither of which is accepted on the filter. A subset enum here prevents accidentally constructing filter values the server rejects.

```rust,ignore
// v2.x
let filter = ExecutionFilter {
    side: "BUY".to_owned(),
    ..ExecutionFilter::default()
};

// v3.0
use ibapi::orders::ExecutionFilterSide;

let filter = ExecutionFilter {
    side: Some(ExecutionFilterSide::Buy),
    ..ExecutionFilter::default()
};
```

If you're matching on the field, swap `if filter.side == "BUY"` for `if filter.side == Some(ExecutionFilterSide::Buy)`.
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
- [ ] `just cover` — touched modules (`orders/mod.rs`, `proto/encoders.rs`, `orders/sync/mod.rs`, `orders/async/mod.rs`) should not regress
- [ ] Grep `README.md` / `docs/*.md` / module rustdoc for `ExecutionFilter`, `filter.side`, the literal string `"BUY".to_owned()` adjacent to `side:` — manual mental compile per `feedback_md_doc_snippets_rot_silently`
- [ ] Confirm no `examples/async/` executions example needs the same update (currently none — verified)
- [ ] `docs/migration-3.0.md` §12 added in the same PR

---

## Open verification before opening the PR

1. **Integration crates**: `integration/sync/tests/orders.rs:250` and `integration/async/tests/orders.rs:256` both call `client.executions(ExecutionFilter::default())` — default produces `side: None`, unchanged behavior. Verify no other integration test constructs a non-default filter.
2. **`testdata/builders/orders.rs:1013-1037`**: `ExecutionsRequestBuilder` holds an `ExecutionFilter` directly and forwards to `encode_execution_filter`. No type-erased side path — the wire is generated from the typed field. Verify after migration.
3. **`utoipa::ToSchema` derive**: the `#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]` annotation must work on `#[non_exhaustive]` enums. Confirmed in PR 3b for `SecurityIdType`; same shape here.

---

## Cross-cutting follow-ups

### `Error::Parse(usize, String, String)` index slot

Running count (parent plan §"Cross-cutting follow-ups"): PR 1 added 2 fake-`0` sites, PR 2 added 1, PR 3b added 1, PR 4a adds 1 (via the `OrderStatusKind` retrofit — `impl_wire_enum!` generates the `Err` arm), PR 4b adds 1 (`ExecutionFilterSide`). Cumulative: **6 fakes**. Past the 5-fake inflection point — decide on the index-slot shape before PR 5b lands. Track in [`v3-api-ergonomics.md` §5](v3-api-ergonomics.md).

### Three-way `ExecutionFilter` construction overlap

Like PR 2's `Contract::option` observation, this PR highlights that `ExecutionFilter` has no builder — callers use struct-literal `..Default::default()` patterns exclusively. Once `side` becomes typed, the struct-literal shape stays readable; no builder needed. Not a follow-up — just noting the path.

---

## Out of scope

- `Execution.side` (the inbound fill-side field) — that's PR 5a (live diagnostic) → PR 5b (typed). Distinct field, distinct wire vocab (`BOT`/`SLD` confirmed; short-sale forms unverified). Tracked in parent plan.
- Promoting `parse_optional` / `parse_required` to take `&str` instead of `Option<&str>` — already accepted shape from PR 1 + PR 2/3a.
- `ExecutionFilter` builder pattern — not requested; struct-literal `..Default::default()` shape is idiomatic for filter types.

---

## Rule references

- CLAUDE.md rule 9 — modernize touched modules (async `# Examples` block added).
- CLAUDE.md rule 16 — typed-status migration pattern; verify wire before typing (done: C# `ExecutionFilter.cs:47-49` confirms `BUY`/`SELL` only).
- CLAUDE.md rule 18 — public API needs a doc-example (async `executions` gap closed).
- CLAUDE.md rule 23 — restrictive API additions split (PR 4a/4b separation).
- `feedback_verify_wire_before_typing` — C# reference verified.
- `feedback_md_doc_snippets_rot_silently` — `docs/*.md` and `README.md` greps before merge.
- `feedback_stream_adapter_consume_form` — async doc-example uses consume form, not the `(&mut sub).filter_data()` cast.
