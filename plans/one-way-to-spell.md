# One Way To Spell Each Thing — Public Surface Consolidation

Parent: `plans/v3-api-ergonomics.md` §7 first bullet.

**Goal.** A user reading `docs.rs` should see one obvious place for each public
type. Today the same type is often reachable at 2–3 paths because internal
grouping modules (`contracts::builders`, `contracts::types`, `orders::builder`,
domain `sync` / `r#async` impl modules) are `pub mod` even though their
contents are already re-exported up to the canonical parent.

The canonical pattern (already enforced for `Client`, `Subscription`,
`NoticeStream`, `SharesChannel` per §3 and §2) is:

- **Definition site** — wherever the type is `pub struct`/`pub enum`.
- **Canonical public path** — the domain module (`ibapi::orders::Foo`,
  `ibapi::contracts::Bar`).
- **Sync-explicit alternate** (only for dual-feature `Client`/`Subscription`-
  shaped types) — `ibapi::client::blocking::*`.
- **Convenience re-exports** — `ibapi::*` (crate root) and
  `ibapi::prelude::*` (high-frequency types only).

Internal grouping submodules that exist only to organize the source tree
should be `pub(crate)` or `#[doc(hidden)]`. Users reaching past the
canonical paths is a smell.

---

## Audit findings (2026-05-19)

### A. Externally reachable duplicates

| Type / family | Canonical | Duplicate path | External callers using the dupe? |
|---|---|---|---|
| `TickType` | `contracts::tick_types::TickType` | `market_data::realtime::TickType` (via `pub use` at `src/market_data/realtime/mod.rs:25`) | None (tests use canonical) |
| Typed contract builders (`StockBuilder`, `OptionBuilder`, `FuturesBuilder`, `ContinuousFuturesBuilder`, `ForexBuilder`, `CryptoBuilder`, `SpreadBuilder`, `LegBuilder`, `Leg`) | `contracts::*` (via `pub use builders::*` at `src/contracts/mod.rs:20`) | `contracts::builders::*` (because `pub mod builders` at `src/contracts/mod.rs:28`) | None |
| `contracts::types::*` (`Symbol`, `Exchange`, `Currency`, `OptionRight`, `LegAction`, `Cusip`, `Isin`, `Strike`, `BondIdentifier`, `ContractMonth`, `ExpirationDate`, `SecurityIdType`, `Missing`, …) | `contracts::*` (via `pub use types::*` at `src/contracts/mod.rs:22`) | `contracts::types::*` (because `pub mod types` at `src/contracts/mod.rs:29`) | None |
| `OrderBuilder`, `BracketOrderBuilder`, `BracketOrderIds`, `OrderId` | `orders::*` (via `pub use builder::{...}` at `src/orders/mod.rs:46`) | `orders::builder::*` (because `pub mod builder` at `src/orders/mod.rs:37`) | Only test code (`src/orders/builder/tests.rs`); zero in `examples/`, `docs/`, `README.md` |

### B. Internal-impl modules that show up in docs.rs nav

The `client::sync` / `client::r#async` modules were marked `#[doc(hidden)]` in
§3's resolution. The same shape is open for:

| Module | Why it's `pub mod` today | What's accessible |
|---|---|---|
| `market_data::historical::sync` / `historical::r#async` | impls live here | Items re-exported up via `pub use sync::*` and `pub use r#async::TickSubscription`; the modules themselves don't add anything users should reach directly. |
| `market_data::realtime::sync` / `realtime::r#async` | impls live here | Same — content is hoisted via `pub use sync::*`. |
| `subscriptions::sync` / `subscriptions::r#async` | per-feature `Subscription` / `SharesChannel` impls | Already mirrored at `subscriptions::*` and `client::blocking::*` per §2's PR #571 + #572. |

External grep confirms no `examples/`, `docs/`, or `README.md` callers reach
these paths.

### C. Already canonical (no work needed)

These look like duplicates at first glance but are intentional per the rules
quoted above:

- `ibapi::Client` + `ibapi::client::Client` + `ibapi::client::blocking::Client` —
  the labelled sync-explicit path is the documented v3 convention (§3).
- `ibapi::Error` + `ibapi::errors::Error` + `ibapi::prelude::Error` —
  domain + root + prelude trio.
- `ibapi::Subscription` (prelude) + `ibapi::subscriptions::Subscription` +
  `ibapi::client::blocking::Subscription` — same trio, post PR #571.
- `ibapi::contracts::TickType` is *not* reachable today (it's at
  `contracts::tick_types::TickType` only); the duplicate is in `market_data`,
  not in `contracts`.

### D. Already `pub(crate)`, no concern

- `ibapi::connection` / `ibapi::transport` / `ibapi::messages` / `ibapi::proto`
  — all `pub(crate)` at the lib.rs level. Their internal `pub` declarations
  are crate-internal. (`ibapi::StartupMessage` is reachable only via the
  explicit re-export at `src/lib.rs:89`.)

---

## End-state target

After this plan ships, every user-facing type has **one** public spelling
under its domain module plus the optional root/prelude/blocking trio:

```
ibapi::contracts::{Contract, ContractBuilder, StockBuilder, OptionBuilder,
                   Symbol, Exchange, Currency, OptionRight, ...,
                   tick_types::TickType}
ibapi::orders::{Order, OrderBuilder, BracketOrderBuilder, BracketOrderIds,
                OrderId, OrderStatusKind, Action, ExecutionFilter, ...,
                builder::{algo_builders::*, algo_helpers::*, condition_helpers::*}}
ibapi::market_data::{historical::*, realtime::*, MarketDataType, TradingHours}
ibapi::accounts::*
ibapi::news::*  ibapi::scanner::*  ibapi::wsh::*
ibapi::subscriptions::{Subscription, NoticeStream, SubscriptionItem, ...}
ibapi::{Client, ClientBuilder, Error, Notice, NoticeCategory, ...}  // root
ibapi::prelude::*
ibapi::client::blocking::{Client, ClientBuilder, Subscription, ...}  // sync-explicit
```

Internal impl modules (`historical::sync`, `historical::r#async`,
`realtime::sync`, `realtime::r#async`, `subscriptions::sync`,
`subscriptions::r#async`) remain `pub` (we can't narrow them without breaking
the existing top-level `pub use` rewrites) but are `#[doc(hidden)]` so
docs.rs only shows the canonical parent paths.

Internal grouping modules (`contracts::builders`, `contracts::types`,
optionally `orders::builder` itself for the top-level overlap only) become
`pub(crate)` — what's hoisted via `pub use *::*` remains the only public
spelling.

---

## Per-PR breakdown

Each PR independently shippable; CLAUDE.md rule 23 (modernize callers,
*then* restrict) is irrelevant here because the duplicate paths have no
external callers. CI is the contract — green on default, `--features sync`,
and `--all-features` is the gate.

### PR 1 — Drop the `TickType` cross-domain re-export

- Delete `src/market_data/realtime/mod.rs:25` (`pub use crate::contracts::tick_types::TickType;`).
- Verify: no `market_data::realtime::TickType` references in `src/`,
  `examples/`, `docs/`, `README.md`. Tests already use the canonical
  `crate::contracts::tick_types::TickType` path.
- Migration guide §N: "`TickType` moved" — but only if anyone was using the
  duplicate; audit shows none, so just add a one-line note.
- ~5 LoC change.

### PR 2 — Narrow `contracts::builders` and `contracts::types` to `pub(crate)`

- `src/contracts/mod.rs:28` — `pub mod builders;` → `pub(crate) mod builders;`.
- `src/contracts/mod.rs:29` — `pub mod types;` → `pub(crate) mod types;`.
- The `pub use builders::*;` + `pub use types::*;` hoists at lines 20, 22
  remain — they are the canonical public spelling.
- Verify: zero external uses of `contracts::builders::` or `contracts::types::`
  in `examples/`, `docs/`, `README.md`. (Confirmed by grep.)
- Migration guide §N: "`contracts::builders::*` and `contracts::types::*`
  paths removed; types remain at `contracts::*`." Cheap, no caller breakage.
- ~2-line change + migration note.

### PR 3 — Drop `OrderBuilder` / `BracketOrderBuilder` / `BracketOrderIds` / `OrderId` from `orders::builder::*`

- `src/orders/builder/mod.rs:28-29` — remove the `pub use order_builder::{...}`
  and the `OrderId, BracketOrderIds` entries from the `types::{...}` list.
  Keep everything else (algo builders, algo helpers, condition helpers,
  `OrderType`, `Price`, `Quantity`, `TimeInForce`, `ValidationError`,
  `AuctionType`, `OrderAnalysis`) — those have no `orders::*` duplicate.
- Update test imports in `src/orders/builder/tests.rs` to use
  `crate::orders::{OrderBuilder, OrderId}`.
- Fix internal rustdoc link `crate::orders::builder::OrderBuilder::build_order`
  at `src/orders/common/order_builder/mod.rs:31` to point to
  `crate::orders::OrderBuilder::build_order`.
- Verify: zero external uses of `orders::builder::OrderBuilder` etc. in
  `examples/`, `docs/`, `README.md`. (Confirmed.)
- Migration guide §N: "`orders::builder::OrderBuilder` removed; use
  `orders::OrderBuilder`." Same for the other three.
- ~15-line change + test/doc edits.

### PR 4 — `#[doc(hidden)]` on `historical::sync`, `historical::r#async`, `realtime::sync`, `realtime::r#async`, `subscriptions::sync`, `subscriptions::r#async`

- Mirror the `client::sync` / `client::r#async` treatment from §3:

  ```rust
  #[doc(hidden)]
  #[cfg(feature = "sync")]
  pub mod sync;

  #[doc(hidden)]
  #[cfg(feature = "async")]
  pub mod r#async;
  ```

- Sites:
  - `src/market_data/historical/mod.rs:17, 20-21`
  - `src/market_data/realtime/mod.rs:16-18, 21-22`
  - `src/subscriptions/mod.rs:7-8, 10-11`
- Add a `## Canonical paths` rustdoc section to each parent module that
  explains the convention (mirrors `src/client/mod.rs:3-22`). For
  `historical` / `realtime`: the canonical is the domain module; the impl
  submodules are hidden but reachable as paths for crate-internal use. For
  `subscriptions`: canonical is `subscriptions::*` (which prefers async when
  both features are on) and `client::blocking::*` for sync-explicit.
- No source-breaking change — modules remain `pub`, just dropped from the
  nav.
- Verify: docs.rs sidebar no longer lists the impl modules under each
  domain. `cargo doc --no-deps --all-features` clean.
- ~6 cfg-attribute insertions + ~3 rustdoc sections.

### PR 5 — Migration guide consolidation + audit close-out

- Single migration guide section "Public path consolidation" listing the
  retired duplicate paths (PR 1, 2, 3) and the doc-hidden modules (PR 4).
- Update `plans/v3-api-ergonomics.md` §7 first bullet to `[x]` with PR
  numbers.
- Delete this plan once §7 is checked in v3-api-ergonomics.md (per
  v3-api-ergonomics.md "## How to use this doc" — prune after ~one cycle).

---

## What this plan is **not**

- **Not** a `pub(crate)` sweep on `orders::builder`. The algo/condition
  helper modules under it have heavy external usage (15+ doc callsites
  for `orders::builder::price`, `orders::builder::time`, `pct_vol`, `twap`,
  etc.). Keeping `orders::builder` as the documented home for the
  low-level fluent layer is intentional, mirroring the
  `order_builder::*` (legacy free fns) / `OrderBuilder` (fluent) split
  documented in §1 PR #549.
- **Not** a `#[non_exhaustive]` sweep. Rejected in §7 second bullet — see
  the rationale there and the `feedback_non_exhaustive_caseby_case`
  memory.
- **Not** a rename or restructure of any public path. Every type stays
  where users reach it today; this plan only removes the *second*,
  third, or fourth path to the same destination.
