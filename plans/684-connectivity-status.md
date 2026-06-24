# Issue #684 — Expose farm-state granularity within the 2100–2169 warning range

## Goal

Let downstream reconnect/health logic distinguish data-farm connectivity sub-states
without re-parsing codes. Implement **Option 1** from the issue: an additive
`Notice::connectivity_status(&self) -> Option<ConnectivityStatus>` accessor.
`NoticeCategory` stays untouched (the 2100..=2169 partition is unchanged).

Additive-only → no breaking change. CHANGELOG `Added`; no `migration-3.0.md` entry.

## Decisions (confirmed)

- **Vocabulary: wire-accurate 4-state**, correcting the issue's table (which contradicts
  the wire text + this repo's own test comments in `transport/common_tests.rs` /
  `messages/tests.rs`):

  | State | Codes | Wire meaning |
  |---|---|---|
  | `Ok` | 2104, 2106, 2158 | …data farm connection is OK |
  | `Broken` | 2103, 2105, 2157 | …data farm connection is broken |
  | `Inactive` | 2107, 2108 | …connection is inactive but should be available upon demand |
  | `Connecting` | 2119 | …data farm is connecting |

  All other codes → `None`. The issue grouped 2105 (broken) with 2108 (inactive-but-fine)
  as "degraded" and called 2107 "disconnected"; that conflates a broken link with two
  dormant-idle states. We expose the wire-accurate split instead.

- **Consolidation**: hold farm code sets in shared `pub(crate) const` arrays in `messages.rs`
  (single source of truth), and rewrite transport's `is_benign_connectivity_notice` to
  delegate to the new classifier. Removes the duplicated
  `BENIGN_CONNECTIVITY_CODES = [2104,2106,2158]` array.

- **`#[non_exhaustive]` on the enum**: no. The variant set is closed from the consumer's
  perspective — an unrecognized code classifies as `None` via `from_code`, not a future
  variant, so unknowns are handled through the `Option`, not the enum. Exhaustive matching
  is a feature for callers (per the "non_exhaustive is deliberate, not default" rule); adding
  the attribute would force every consumer into a dead `_` arm for no benefit.

- **Pure primitive + accessor split** (composability/SRP): `ConnectivityStatus::from_code(i32)`
  is the public classifier; `Notice::connectivity_status(&self)` is a one-line delegate. The
  issue's own evidence — `ibcore::classify_farm(code: i32)` — works over a raw `i32`, not a
  `Notice`, so the primitive is motivated, not speculative. Diverges from the `NoticeCategory`
  method-only precedent **deliberately**, because farm classification has a documented external
  raw-`i32` consumer that `NoticeCategory` never had.

- **Const visibility: `pub(crate)`, not `pub`.** With `from_code` public, raw-code matching
  no longer needs the arrays exposed. Unlike `DATA_ADVISORY_CODES` (#680), which earned `pub`
  via an in-crate cross-module reader (`transport/routing.rs:145`), the farm consts have no
  such reader once transport delegates to the method — so no `lib.rs` / `prelude.rs` const
  re-export.

## Out of scope

- Farm *kind* (market-data vs HMDS vs sec-def). The codes carry it (2105 HMDS vs 2103
  market-data), but the issue asks only for connectivity status. Note as a possible future
  `Notice::data_farm()` extension; do not build now.

## Source-of-truth verification

C# `EClientErrors.cs` does **not** carry these — they are TWS server-emitted notices, not
client error-table entries (grep of `/Users/wboayue/projects/tws-api/source/csharpclient`
returns nothing for 2103–2158). Authority is IB's published Message Codes table, already
encoded in this repo's test comments:
- `src/transport/common_tests.rs:13-15` — 2103/2105/2157 "broken"
- `src/transport/common.rs:14-18` — 2104/2106/2158 "OK"
- `src/messages/tests.rs:229` — 2107 "HMDS data farm connection is inactive"

## Implementation

### 1. `src/messages.rs` — constants

Add near the other notice-code constants (after `DATA_ADVISORY_CODES`, ~line 1120).
`pub(crate)` (see Decisions). Each array doc-comments the per-code wire string:

```rust
/// Data-farm codes reporting a healthy connection ("…connection is OK").
/// Subset of `WARNING_CODE_RANGE`; classified `ConnectivityStatus::Ok`.
pub(crate) const FARM_OK_CODES: [i32; 3] = [2104, 2106, 2158];

/// Data-farm codes reporting a broken connection ("…connection is broken").
pub(crate) const FARM_BROKEN_CODES: [i32; 3] = [2103, 2105, 2157];

/// Data-farm codes reporting a dormant-but-available connection
/// ("…inactive but should be available upon demand").
pub(crate) const FARM_INACTIVE_CODES: [i32; 2] = [2107, 2108];

/// Data-farm codes reporting a connection in progress ("…farm is connecting").
pub(crate) const FARM_CONNECTING_CODES: [i32; 1] = [2119];
```

### 2. `src/messages.rs` — `ConnectivityStatus` enum

Place adjacent to `NoticeCategory` (~line 1190). Mirror its derives/doc style:

```rust
/// Connectivity sub-state of a data-farm notice within `WARNING_CODE_RANGE`.
///
/// Returned by [`Notice::connectivity_status`] for the data-farm status codes;
/// `None` for every other notice. Lets reconnect/health logic tell "farm came
/// back online" from "farm went inactive" without re-parsing codes.
///
/// `#[non_exhaustive]` — IBKR controls this vocabulary and may add farm states.
///
/// # Examples
/// ```no_run
/// use ibapi::{Notice, ConnectivityStatus};
/// # let notice: Notice = unimplemented!();
/// match notice.connectivity_status() {
///     Some(ConnectivityStatus::Ok)       => {/* farm healthy */}
///     Some(ConnectivityStatus::Broken)   => {/* link down */}
///     Some(ConnectivityStatus::Inactive) => {/* dormant, available on demand */}
///     Some(ConnectivityStatus::Connecting) => {/* reconnecting */}
///     _ => {}
/// }
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityStatus {
    /// Data-farm connection is OK ([`FARM_OK_CODES`]).
    Ok,
    /// Data-farm connection is broken ([`FARM_BROKEN_CODES`]).
    Broken,
    /// Connection inactive but available upon demand ([`FARM_INACTIVE_CODES`]).
    Inactive,
    /// Connection is in the process of connecting ([`FARM_CONNECTING_CODES`]).
    Connecting,
}
```

### 3. `src/messages.rs` — `ConnectivityStatus::from_code` (pure primitive)

Add to `impl ConnectivityStatus`. This is the single classifier; the if/else chain mirrors
`Notice::category()`'s idiom (a table-driven `[(status, &codes[..])].find(..)` was considered
and rejected — less readable, no real DRY gain, against rule 25).

```rust
impl ConnectivityStatus {
    /// Classify a raw TWS error code into a data-farm connectivity sub-state.
    ///
    /// Returns `Some(..)` for the data-farm status codes inside
    /// `WARNING_CODE_RANGE`; `None` for every other code. Use this when you
    /// hold a raw code; use [`Notice::connectivity_status`] when you hold a
    /// [`Notice`].
    ///
    /// # Examples
    /// ```
    /// use ibapi::ConnectivityStatus;
    /// assert_eq!(ConnectivityStatus::from_code(2104), Some(ConnectivityStatus::Ok));
    /// assert_eq!(ConnectivityStatus::from_code(2105), Some(ConnectivityStatus::Broken));
    /// assert_eq!(ConnectivityStatus::from_code(500), None);
    /// ```
    pub fn from_code(code: i32) -> Option<ConnectivityStatus> {
        if FARM_OK_CODES.contains(&code) {
            Some(ConnectivityStatus::Ok)
        } else if FARM_BROKEN_CODES.contains(&code) {
            Some(ConnectivityStatus::Broken)
        } else if FARM_INACTIVE_CODES.contains(&code) {
            Some(ConnectivityStatus::Inactive)
        } else if FARM_CONNECTING_CODES.contains(&code) {
            Some(ConnectivityStatus::Connecting)
        } else {
            None
        }
    }
}
```

### 3b. `src/messages.rs` — `Notice::connectivity_status` (one-line delegate)

Add to `impl Notice` (near `is_warning` / `category`):

```rust
/// Classify the data-farm connectivity sub-state of this notice.
///
/// Returns `Some(..)` for the data-farm status codes inside
/// `WARNING_CODE_RANGE`; `None` for every other notice. Additive to
/// [`Notice::category`], which still classifies all of 2100..=2169 as
/// [`NoticeCategory::Warning`]. Thin wrapper over
/// [`ConnectivityStatus::from_code`].
///
/// # Examples
/// ```no_run
/// use ibapi::{Notice, ConnectivityStatus};
/// # let notice: Notice = unimplemented!();
/// if notice.connectivity_status() == Some(ConnectivityStatus::Broken) {
///     eprintln!("data farm down: {notice}");
/// }
/// ```
pub fn connectivity_status(&self) -> Option<ConnectivityStatus> {
    ConnectivityStatus::from_code(self.code)
}
```

### 4. `src/lib.rs` — re-exports

- Add `ConnectivityStatus` to the `pub use messages::{... NoticeCategory ...}` line (221).
- **No const re-export** — `FARM_*_CODES` are `pub(crate)`.

### 5. `src/prelude.rs` — re-export

Add `ConnectivityStatus` alongside `NoticeCategory` (line 44).

### 6. `src/transport/common.rs` — DRY consolidation

Replace the local `BENIGN_CONNECTIVITY_CODES` array + `is_benign_connectivity_notice`
body with a delegation to the shared classifier:

```rust
use crate::messages::ConnectivityStatus;

fn is_benign_connectivity_notice(notice: &Notice) -> bool {
    notice.connectivity_status() == Some(ConnectivityStatus::Ok)
}
```

Update the `log_unrouted_notice` call site (it passes `notice.code` today → pass `notice`).
Confirm `FARM_OK_CODES == [2104,2106,2158]` so the info-vs-warn behavior is byte-identical.

**Logging-policy invariant (do not widen):** "benign → `info`" must stay **`Ok`-only**.
`Broken` (2103/2105/2157), `Inactive` (2107/2108), and `Connecting` (2119) keep falling
through to `warn!` via `is_warning()`, exactly as today. The refactor swaps the *vocabulary
source*, not the *policy* — the policy (which states are quiet) stays in transport. The
migrated common test pins this.

## Tests

### `src/messages/tests.rs`

- **`test_connectivity_status_from_code_table`** (tests the *primitive*): drive every code in
  each `FARM_*_CODES` const through `ConnectivityStatus::from_code(code)`, assert the matching
  variant. Derive expectations from the consts (rule 21) — loop over `FARM_OK_CODES` asserting
  `Some(Ok)`, etc. Assert non-farm neighbors (e.g. `2100`, `2120`, `2169`, `500`) → `None`.
- **`test_connectivity_status_delegates`** (thin): one `notice_with_code(2105).connectivity_status()
  == ConnectivityStatus::from_code(2105)` check — confirms the accessor delegates without
  re-asserting the whole table (avoids test duplication; finding C3).
- **`test_connectivity_status_subset_of_warning`**: every farm code is also `is_warning()`
  and `category() == NoticeCategory::Warning` (proves the additive contract — the partition
  is unchanged). 2119/2158 etc. are all inside 2100..=2169 ✓.
- Reuse the existing `notice_with_code` helper.

### `src/transport/common_tests.rs`

- Update `test_is_benign_connectivity_notice` for the new `&Notice` signature; keep the
  assertion set (2104/2106/2158 benign; 2103/2105/2157/etc. **not** benign). Name/comment it
  to pin the logging-policy invariant: only `ConnectivityStatus::Ok` codes are benign;
  `Broken`/`Inactive`/`Connecting` are not. Reference `FARM_OK_CODES` rather than the deleted
  local array.

## Docs / changelog

- **CHANGELOG.md** `## [Unreleased]` → `### Added`:
  `- `ConnectivityStatus` enum with `ConnectivityStatus::from_code()` and
  `Notice::connectivity_status()` to expose data-farm connectivity sub-states (Ok / Broken /
  Inactive / Connecting) within the 2100–2169 warning band (#684).`
  (The `FARM_*_CODES` constants are `pub(crate)` — not part of the public API.)
- **README.md**: only if a notice/category example is shown — grep for `NoticeCategory`;
  add a one-liner pointing at `connectivity_status()` if there's a natural home, else skip.
- **migration-3.0.md**: no entry (additive, non-breaking).
- No rustdoc intra-doc link risk beyond the `[`FARM_*`]` / `[`Notice::...`]` links added.

## Quality gate (before PR)

Run the full matrix from CLAUDE.md "Quick Commands":
- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings` / `--features sync` / `--all-features`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` ×3 configs (default / sync / all-features)
- `just test`
- `just cover` — verify `src/messages.rs` stays ≥90% (the new method is fully table-tested)
- Integration crates: `messages.rs` is not wire-format-adjacent (no encoder/decoder/Subscription
  change), so the `ibapi-integration-*` build is not strictly required — but cheap to run.

## Risk / notes

- Pure addition over an existing classifier; no decoder, encoder, or routing change.
- The only behavioral touch is transport severity-logging, kept byte-identical by the
  `FARM_OK_CODES == old BENIGN array` invariant — covered by the existing common test.
- `connectivity_status()` is `&self` + pure (no clock/IO) → trivially testable, no seam needed.
```
