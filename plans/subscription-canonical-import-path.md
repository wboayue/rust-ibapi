# One canonical `Subscription` import path

**Parent:** [v3-api-ergonomics.md §2 "One canonical `Subscription` import path"](v3-api-ergonomics.md).

**Scope:** consolidate the three+ ways to import `Subscription` so there is one canonical public path (`crate::subscriptions::Subscription`) plus one labelled sync-explicit escape hatch (`crate::client::blocking::Subscription`). Drop the duplicate `crate::client::Subscription` re-exports and rewire `prelude` symmetrically.

**Why now:** v3.0 ergonomics work — "one obvious way to do each thing" per the parent doc's preamble. Zero new functionality; pure path consolidation.

---

## Current state (audited 2026-05-12)

Four exposure points exist today:

| Path | Source | Feature gate | Consumers |
|---|---|---|---|
| `crate::subscriptions::Subscription` | `subscriptions/mod.rs:32,35` | `sync` (when `not async`) → `sync::Subscription`; `async` → `r#async::Subscription` | **The canonical home.** Most domain modules import here (`orders/async/mod.rs`, `accounts/async/mod.rs`, `scanner/async.rs`, `news/async.rs`, `display_groups/async.rs`, `market_data/realtime/async/mod.rs`). |
| `crate::client::Subscription` | `client/mod.rs:41,44` | `sync` (when `not async`) → `subscriptions::sync::Subscription`; `async` → `subscriptions::r#async::Subscription` | **Duplicate.** Zero direct consumers in production code; only used as the sync source for `prelude::Subscription`. |
| `crate::client::blocking::Subscription` | `client/mod.rs:20-22` inside `pub mod blocking { ... }` | `#[cfg(feature = "sync")]` (always sync) | The labelled sync-explicit path. 2 internal callers: `src/orders/sync/mod.rs:5`, `src/scanner/sync.rs:7`. Needed when both features are enabled — top-level alias prefers async, so sync explicit needs its own path. |
| `crate::prelude::Subscription` | `prelude.rs:51,54` | sync path sources from `client::Subscription`; async path sources directly from `subscriptions::{Subscription, SubscriptionItemStreamExt}` | Public via `use ibapi::prelude::*;`. The asymmetric sourcing is the bug the parent plan calls out. |

External docs (`docs/*.md`) reference `Subscription<T>` as a type but **do not** spell any import path (verified by `grep`). No README references. No breakage from path reshuffling at the doc layer.

---

## Recommendation

1. **Remove `crate::client::Subscription` re-exports** (`client/mod.rs:40-44`). Zero production consumers; only `prelude.rs:51` reaches in via this path, and that gets rewired.

2. **Update `prelude::Subscription` to source from `crate::subscriptions::Subscription` on both feature paths.** The async path already does. The sync path (`prelude.rs:51`) currently goes through `client::Subscription` — make it symmetric.

3. **Keep `crate::client::blocking::Subscription`** untouched. It is the labelled sync-explicit path and has real callers. Distinct concern from "the top-level alias."

The end state:

```
Canonical type alias:    crate::subscriptions::Subscription    (sync OR async, feature-aware)
Sync-explicit path:      crate::client::blocking::Subscription (always sync, needed under both features)
Prelude convenience:     crate::prelude::Subscription          (re-exports the canonical)
```

The `client::Subscription` non-blocking path goes away.

---

## Files touched

| File | Change |
|---|---|
| `src/client/mod.rs` | Delete lines 40-44 (the two `#[cfg]`-gated `pub use crate::subscriptions::{sync,r#async}::Subscription;` blocks). The adjacent `#[cfg(feature = "sync")] pub use crate::subscriptions::sync::SharesChannel;` at line 47 stays — `SharesChannel` is sync-specific plumbing, separate concern. The `pub mod blocking { ... pub use crate::subscriptions::sync::Subscription ... }` block at lines 14-23 stays — that is the sync-explicit path. |
| `src/prelude.rs` | Line 51: `#[cfg(all(feature = "sync", not(feature = "async")))] pub use crate::client::Subscription;` → `pub use crate::subscriptions::Subscription;`. Move the `cfg` attribute consistently — the async path at line 54 has no `cfg` (the `subscriptions::Subscription` re-export is feature-gated at its source). Consider collapsing the two `Subscription` re-exports into a single unconditional `pub use crate::subscriptions::Subscription;` since `subscriptions::mod.rs` already does the feature-gated picking. |
| `src/orders/sync/mod.rs` | No change required. Keeps `use crate::client::blocking::Subscription;` — that path is preserved. |
| `src/scanner/sync.rs` | No change required (same reason). |
| `src/client/sync.rs` | No change required. Internal rustdoc link `[Subscription::next](crate::client::blocking::Subscription::next)` at line 259 stays valid. |

The diff is small — roughly 7 lines deleted, 1 line changed.

---

## Concrete change sketch

### `src/client/mod.rs`

```rust
// DELETE these blocks (lines 40-44):
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use crate::subscriptions::sync::Subscription;

#[cfg(feature = "async")]
pub use crate::subscriptions::r#async::Subscription;
```

Keep `pub mod blocking { ... pub use crate::subscriptions::sync::Subscription ... }` intact — that's the labelled sync escape hatch.

### `src/prelude.rs`

```rust
// BEFORE (lines 49-54):
// Client subscription type
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use crate::client::Subscription;
pub use crate::subscriptions::{NoticeStream, SubscriptionItem};
#[cfg(feature = "async")]
pub use crate::subscriptions::{Subscription, SubscriptionItemStreamExt};

// AFTER:
// Subscription types (canonical home: crate::subscriptions)
pub use crate::subscriptions::{NoticeStream, Subscription, SubscriptionItem};
#[cfg(feature = "async")]
pub use crate::subscriptions::SubscriptionItemStreamExt;
```

`subscriptions::Subscription` is itself feature-gated at the source (`subscriptions/mod.rs:32,35`) — picking async when `async` is on, sync when `sync` is on without async. So the prelude re-export can be unconditional; the gating happens upstream. Symmetric and shorter.

`SubscriptionItemStreamExt` stays under the async `cfg` — it's the async-stream extension trait, no sync analog.

---

## Verification

1. **Build all three feature configs**:
   ```bash
   cargo check                                            # default-async
   cargo check --no-default-features --features sync
   cargo check --all-features
   ```
2. **Run full quality gates** per CLAUDE.md "Quick Commands" (fmt + clippy × 3 + rustdoc × 3 + just test + integration crates).
3. **Grep for any straggling `crate::client::Subscription` / `ibapi::client::Subscription`** to confirm nothing else reaches in via the removed path:
   ```bash
   grep -rn 'client::Subscription\b' src examples integration docs
   # Expected hits AFTER the change: zero. The only old hit was prelude.rs:51, now rewired.
   # Hits matching 'client::blocking::Subscription' stay — that's the explicit sync path.
   ```

---

## Public API delta

- **Removed**: `ibapi::client::Subscription` (the duplicate alias). Zero in-crate consumers; was an unintentional convenience path that duplicated `ibapi::subscriptions::Subscription`.
- **Unchanged**: `ibapi::Subscription` (via prelude), `ibapi::subscriptions::Subscription`, `ibapi::client::blocking::Subscription`.

External callers using `use ibapi::client::Subscription;` will break. Migration: change to `use ibapi::subscriptions::Subscription;` or `use ibapi::prelude::*;`. Document in `docs/migration-3.0.md` (new §13).

This is a v3.0 breaking change but a small one — the path is functionally identical and obviously redundant. The migration note is one line.

---

## Migration note

Shipped in [`docs/migration-3.0.md` §13](../docs/migration-3.0.md). One-line redirect; one before/after `use` snippet.

---

## Risk assessment

- **In-crate**: trivially low. Two-line touch on `client/mod.rs`, one-line touch on `prelude.rs`. The compiler verifies the full graph; CI catches anything stale.
- **External crates depending on `ibapi`**: anyone who wrote `use ibapi::client::Subscription;` will get a "not found" compile error. The fix is mechanical and the migration note above is one line. Pre-3.0 release window, so the breakage cost is low.
- **Documentation**: no `.md` snippets reference the removed path (verified by grep).

---

## Out of scope

- **The `NoticeStream` / `SubscriptionItem` / `SubscriptionItemStreamExt` re-export tidiness.** They follow the same pattern (canonical home in `subscriptions`, convenience in `prelude`). Already symmetric — no change needed in this PR.
- **The `SharesChannel` re-export at `client/mod.rs:47`.** Sync-only plumbing trait; separate concern from the `Subscription` consolidation. The parent §3 "Hide internal types from the public surface" audit will revisit.
- **Renaming the `subscriptions` module.** Out of scope; the canonical name is the existing one.
- **Adding `#[must_use]` to `Subscription`.** Tracked separately in parent §7.

---

## Rule references

- CLAUDE.md rule 14 — narrow re-exports over widened module visibility.
- v3-api-ergonomics.md §7 "One way to spell each thing."
