# v3.0 Public API Ergonomics — Tracking Doc

A living checklist of public-API rough edges to address before 3.0 ships. Goal:
the API should feel **simple, ergonomic, easy to use, and intuitive** — minimal
ceremony, no stringly-typed escape hatches, one obvious way to do each thing.

**Last audited:** 2026-05-10 (against `main`; floor at `PROTOBUF_SCAN_DATA` = 210).

## How to use this doc

- One bullet per concrete change. Keep them small and independently shippable.
- Each item: **status · problem · proposal · breaking? · notes**.
- Status: `[ ]` open · `[~]` in progress · `[x]` shipped · `[-]` rejected (note why).
- When an item ships, leave it checked here with the PR link for ~one cycle, then prune.
- File a separate `plans/<topic>.md` for any item large enough to need its own plan
  and link it from here.

Related existing tracking docs in `plans/`:
- `generic-tick-types.md`, `legacy-text-protocol-cleanup.md`,
  `protobuf-migration.md`.

---

## 1. Construction & builders

- [ ] **Forbid bare `Contract { ... }` construction.** Today `Contract::stock(...).build()`
  is the blessed path, but ~20+ example sites (e.g. `examples/async/place_order.rs:22`)
  still build the struct field-by-field with `..Default::default()`. Fields are `pub`,
  so there's no compile-time push toward the builder.
  - Proposal: make required fields private (or wrap in newtypes that only the builder
    can construct), keep `pub` on getters; or `#[non_exhaustive]` + private constructor.
  - Breaking: yes (intentional for 3.0).
  - **Escape-hatch invariant** (so locking down doesn't strand users on missing
    sec-type constructors): every `pub` field on `Contract` keeps a corresponding
    setter on `ContractBuilder`. `ContractBuilder::new()` is the named generic
    entry point — anything spellable as `Contract { ... }` stays spellable as
    `ContractBuilder::new().symbol(...).security_type(SecurityType::Warrant)…build()`.
    Today's builder already has setters for every field
    (`src/contracts/common/contract_builder/mod.rs:128-311`); the lockdown only
    works as long as that stays true.
  - Doc the escape hatch in `docs/migration-3.0.md` — "no convenience constructor
    for X? use `ContractBuilder::new()`" — otherwise readers parse "forbid bare
    construction" as "you're stuck if we missed your case."
  - Add a regression test that asserts setter-per-public-field parity (reflective
    enumeration via a macro, or a tiny `compile_fail`/doc-test). Without it the
    invariant drifts silently the next time someone adds a field.

- [x] **Newtype ergonomics: take `impl Into<Symbol>` / `&str` everywhere.** Shipped.
  `Symbol`, `Exchange`, `Currency` impl `From<&str>` + `From<String>` (`src/contracts/types.rs:24,85,141`)
  and the contract builder methods take `impl Into<String>` (`src/contracts/common/contract_builder/mod.rs:330`).
  Verified 2026-05-06.

- [ ] **Converge order construction on one style.** Two coexisting paths today:
  - `order_builder::limit_order(Action::Buy, 100.0, 150.0)` (free fn, returns `Order`)
  - `client.order(&c).buy(100).limit(150.0).submit()` (fluent, owns submission)

  Pick the fluent one as canonical, keep the free fns as a thin convenience layer
  documented as "advanced — bring your own order id." Remove the per-method
  `Action::Buy` argument once the side is implied by `.buy()` / `.sell()`.

- [ ] **Drop `client.next_order_id()` from the canonical happy path.** `submit()` already
  allocates an id internally; the only caller that still needs `next_order_id()` is
  the low-level `place_order(order_id, contract, order)` form. Examples still show it
  at `examples/async/place_order.rs:32, 100, 103`.
  - Decision: keep `next_order_id()` on `Client` for advanced callers (BYO-id flows
    + the low-level `place_order` form), but stop showing it in examples.
  - Sweep: rewrite `examples/async/place_order.rs` (and any sync sibling) to use the
    fluent `client.order(c).buy(n).limit(p).submit()` path; only retain a
    `next_order_id()` example in a dedicated "advanced / BYO order id" section.

## 2. Streaming surface

- [x] **`SubscriptionItem<T>` consistency.** Shipped in PR #517 — per-T `Notice`
  variants deleted; notices route through `SubscriptionItem::Notice` and the
  dedicated `NoticeStream`. Same PR removed the dead untyped `Err` arms.

- [ ] **Standardize the consumer interface on `Stream` (async) and `Iterator` (sync).**
  Today consumers call `subscription.next_data().await` (see
  `examples/async/place_order.rs:47`). For async this should be `StreamExt::next`,
  and for sync the `for item in subscription` form should be the default in examples.
  - Current state: sync `Subscription` impls `IntoIterator`
    (`src/subscriptions/sync.rs:341,349,373`); async exposes `.stream()` returning
    `impl Stream + Unpin` (`src/subscriptions/async.rs:301`) but does NOT impl
    `Stream` directly on `Subscription` itself.
  - Decision needed: (a) impl `Stream` directly on async `Subscription` and drop
    `.stream()`; (b) keep `next_data()` as a thin alias and just sweep examples to
    show `.stream().next().await`; (c) remove `next_data()` entirely.
  - Breaking: yes if we remove.

- [ ] **Notice classification helpers.** Today callers reach for `notice.code` ranges
  to decide "warning" vs "rejection" vs "system message" (some predicates already
  exist on `Notice` — `is_warning`, `is_system_message` — but no public taxonomy
  on the wire-error ranges). Provide:
  - `Notice::is_order_rejection()` (codes 200–399), `Notice::category() -> NoticeCategory`.
  - Precedent: `OrderStatusKind::is_terminal()` (PR #518) — table-driven typed
    classifier, no magic numbers at the call site.

- [x] **`OrderStatus.status: String` → `OrderStatusKind` enum.** Shipped in PR #518
  (commit `b9ed884`). `src/orders/mod.rs:1557` is `pub status: OrderStatusKind` with
  `is_terminal()` etc. Examples now use `.is_terminal()` (lines 61, 143).

- [ ] **Continue the typed-status sweep.** `OrderState.status` already typed as
  `OrderStatusKind` (`src/orders/mod.rs:1280`); remaining `String` fields whose
  wire vocabulary is enumerated:
  - `Execution.side` (`src/orders/mod.rs:1473`) — Buy / Sell / SShort / SLng
  - `Contract.security_id_type` (`src/contracts/mod.rs`) — CUSIP / ISIN / SEDOL / RIC
  - any others surfaced by audit
  Follow the PR #518 pattern (per `CLAUDE.md` rule 21): strict enum, `Display`
  round-trips, decoder rejects empty/missing as `Error::Parse`. **First**: grep
  captured-wire fixtures + the C# reference to confirm the field is actually
  enumerated (rule 21 caveat — `OrderState.completed_status` looked enumerated
  but is free-form text).

- [ ] **One canonical `Subscription` import path.** `Subscription` is reachable from
  `crate::subscriptions::Subscription` (canonical at `src/subscriptions/mod.rs:32,35`),
  `crate::client::Subscription` (feature-gated at `src/client/mod.rs:41,44`), and
  `crate::prelude::Subscription` (feature-gated at `src/prelude.rs:51,53`). Pick
  `crate::subscriptions::Subscription` as canonical and keep the others as `pub use`
  aliases (or remove the client-level paths).

- [x] **`NoticeStream` should not mirror `Subscription`'s sync/async toggle in the
  prelude.** Shipped — `src/prelude.rs:54` re-exports a single `NoticeStream`
  (per-feature impls share the public name; only `await` differs).

- [x] **Unify the two notice APIs.** Shipped 2026-05-08 as part of the
  `Client::builder()` work (option 3 — folded with §4.1). Hard-removed
  `ConnectionOptions::startup_notice_callback`; new path:
  `Client::builder()...connect_with_notice_stream()`. Race fix is automatic —
  the broadcaster lives on `Connection` and is reused across the handshake
  loop AND the post-connect bus. PR #526.

## 3. Naming, layout, prelude

- [ ] **Eliminate prelude collisions.** `BarSize` and `WhatToShow` exist for both
  historical and realtime market data and are re-exported as `HistoricalBarSize` /
  `RealtimeBarSize` in the prelude (`src/prelude.rs:31-34`). Options:
  - Rename one (e.g. `RealtimeBarSize` already differs in variants — rename in source
    too, drop the alias).
  - Or keep the aliases but document them as the canonical names.

- [ ] **Async-vs-blocking naming asymmetry.** `ibapi::Client` is the async client when
  `async` is on; the sync client lives at `ibapi::client::blocking::Client`
  (`src/client/mod.rs:15`). `async` is a reserved keyword so a literal
  `client::async::Client` path needs `r#async` (already used internally). Decision:
  either (a) keep the asymmetry and document `Client` (root) + `client::blocking::Client`
  as the two canonical paths, or (b) expose `client::r#async::Client` as a sibling
  for symmetry in docs/examples.

- [ ] **Reorganize re-exports out of `orders` for non-order types.** `TagValue` is
  re-exported from `orders` (`src/orders/mod.rs:67`) for historical reasons. Move to
  `contracts` (or wherever it logically belongs) and drop the alias.

- [ ] **Hide internal types from the public surface.** Audit `pub` items that look
  like plumbing:
  - `Client::message_bus()` (`src/client/async.rs:359`) and `Client::stubbed()`
    (`src/client/async.rs:342`) — both `pub` on the async side; sync has neither
    in its public signature, so the async exposure looks accidental.
  - `subscriptions::common::SubscriptionItem` (re-exported at module root — fine, but
    confirm `DecoderContext`, `StreamDecoder` stay `pub(crate)`)
  - `pub mod messages` and `pub mod proto` (`src/lib.rs:107, 127`) — confirm what
    consumers actually need versus what's just exposed for tests/examples; consider
    `#[doc(hidden)]` for the advanced bits.

## 4. Connection API

- [x] **Fold connect variants into a builder.** Shipped 2026-05-08 alongside the
  notice-API unification. `Client::builder()` is the canonical entry point
  (`address`, `client_id`, `tcp_no_delay`, `startup_callback` configurators;
  `connect()` and `connect_with_notice_stream()` terminals). Hard-removed
  `connect_with_callback` and `connect_with_options`. `Client::connect(addr, id)`
  stays as the one-liner.

- [x] **`StartupMessageCallback` builder ergonomics.** Shipped — the
  `ConnectionOptions::startup_callback` builder method accepts
  `impl Fn(StartupMessage) + Send + Sync + 'static` and boxes internally
  (`src/connection/common.rs:122`). The type alias `StartupMessageCallback`
  (`src/connection/common.rs:67`) is still `Box<dyn Fn(...)>`, but it's no longer
  on the call-site path.

## 5. Errors

- [ ] **Audit remaining `Error::Simple` / `Error::Message` callers.** Enum is
  `#[non_exhaustive]` (`src/errors.rs:18`) and the typed variants predominate
  (`Io`, `Parse`, `ServerVersion`, `ConnectionFailed`, `ConnectionReset`,
  `Cancelled`, `Shutdown`, `EndOfStream`, `UnexpectedResponse`,
  `UnsupportedTimeZone`, `InvalidArgument`, etc.). ~42 callsites remain
  (concentrated in `src/messages.rs` ~24, `src/errors.rs` ~7, scattered elsewhere).
  Continue splitting `Simple(format!(...))` / `Message` sites into typed variants
  where downstream code would plausibly match (auth failure, request timeout, …).

- [ ] **Distinguish "request rejected by server" from "transport error" in return
  types.** Today both come through `Result<_, Error>`. Consider promoting server-side
  rejections (TWS error codes 200-299, 300-399, 10000+) into a typed sub-enum so
  callers can pattern-match without string parsing.

## 6. Examples & docs

- [ ] **Modernize every example to use the canonical happy path.** After the
  decisions above land, sweep `examples/sync/*.rs` and `examples/async/*.rs`:
  - `Contract::stock("AAPL").build()` not bare struct literals.
  - `client.order(c).buy(n).limit(p).submit()` for orders (drop `next_order_id`
    boilerplate where possible).
  - `while let Some(item) = stream.next().await` for streams.
  - No magic-number notice code comparisons.

- [x] **Migration guide created.** `docs/migration-3.0.md` exists. Keep updating it
  in lockstep with breaking changes (see `CLAUDE.md` § "Keep `README.md` and
  `docs/migration-3.0.md` in sync with v3.0 work" — this is enforced as a PR-time
  check, not a one-shot deliverable).

- [ ] **Consolidate the docs index.** `docs/api-patterns.md` and `docs/contract-builder.md`
  overlap. Merge or cross-link cleanly so the prelude + builders are documented in one
  obvious place.

## 7. Cross-cutting

- [ ] **One way to spell each thing.** After the moves above, audit for duplicate
  re-exports (`pub use foo::Bar;` in two modules). The crate root, `prelude`, and the
  domain module should each pick one home for each type.

- [ ] **`#[non_exhaustive]` on every public enum and struct that may grow.** Avoids
  future breaking releases for additive variants/fields. Coverage today is **very
  low**: only `Error` carries the attribute. Public enums like `Action`,
  `SecurityType`, `OrderStatusKind`, `Liquidity` are unannotated. Sweep before
  3.0 cuts.

- [ ] **`#[must_use]` on every builder and `Subscription`.** Forgetting `.subscribe()`
  / `.submit()` / `.build()` should produce a lint, not silent no-ops. Today: zero
  `#[must_use]` annotations on `ContractBuilder`, `OrderBuilder`, or `Subscription`.

- [x] **No `block_on` in async paths.** Verified clean across `src/` on 2026-05-06
  (no `futures::executor::block_on` usages). Project rule (`CLAUDE.md` §10) keeps
  it that way.
