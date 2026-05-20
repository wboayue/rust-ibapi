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

- [x] **Forbid bare `Contract { ... }` construction.** Shipped 2026-05-10 in
  PR #547 (example modernization) + PR #548 (lockdown). `Contract` is now
  `#[non_exhaustive]`; bare struct literals fail to compile in external crates.
  The escape-hatch invariant — every `pub` field on `Contract` is settable on
  `ContractBuilder` — is enforced by a compile-time regression test
  (`setter_parity_with_contract_fields`) plus a `compile_fail,E0639` doc-test
  on `Contract` that pins to the rustc error code so it can't silently pass
  for the wrong reason. Migration guide §8 documents the escape hatch.
  Bonus: PR #548 also revived `ContractBuilder` (it had been deprecated when
  typed builders shipped) and added `PartialEq<str>`/`PartialEq<&str>` (both
  directions, via macro) for `Symbol`/`Exchange`/`Currency`.

- [x] **Newtype ergonomics: take `impl Into<Symbol>` / `&str` everywhere.** Shipped.
  `Symbol`, `Exchange`, `Currency` impl `From<&str>` + `From<String>` (`src/contracts/types.rs:24,85,141`)
  and the contract builder methods take `impl Into<String>` (`src/contracts/common/contract_builder/mod.rs:330`).
  Verified 2026-05-06.

- [x] **Converge order construction on one style.** Shipped 2026-05-10. Fluent
  `client.order(&c).buy(n).<type>().submit()` is the canonical path; the
  `order_builder::*` free functions are reframed as the advanced / client-less
  layer (BYO order id, offline construction, hand-composed multi-leg orders).
  Module docs at `src/orders/common/order_builder/mod.rs` lead with the fluent
  example and demote the free fns. The fluent path's `.buy()/.sell()` already
  implies side (no `Action` arg on per-type methods). The 50 free fns retain
  their `Action` parameter for backward compat — no deprecation, since the
  plan keeps them as the convenience layer.

- [x] **Drop `client.next_order_id()` from the canonical happy path.** Shipped
  2026-05-10. `client.next_order_id()` stays public for BYO-id callers, but
  examples and docs now use `submit()` (which allocates the id internally) on
  the canonical happy path. `examples/sync/next_order_id.rs` is reframed as
  the dedicated BYO-id advanced example. Diagnostic `next_order_id` prints
  removed from `examples/{sync,async}/connect.rs`,
  `examples/sync/{stream_bars,contract_details}.rs`. Doc-tree
  (`docs/{quick-start,api-patterns,order-types,extending-api,code-style,troubleshooting}.md`)
  swept to fluent canonical; the BYO-id sections in `order-types.md` and
  `troubleshooting.md` retain the manual id + `place_order`/`submit_order`
  pattern as the labelled advanced flow.

## 2. Streaming surface

- [x] **`SubscriptionItem<T>` consistency.** Shipped in PR #517 — per-T `Notice`
  variants deleted; notices route through `SubscriptionItem::Notice` and the
  dedicated `NoticeStream`. Same PR removed the dead untyped `Err` arms.

- [x] **Standardize the consumer interface on `Stream` (async) and `Iterator` (sync).**
  Shipped 2026-05-10 in PR #550. Async `Subscription<T>` now impls
  `Stream<Item = Result<SubscriptionItem<T>, Error>>` directly; inherent
  `next_data` / `stream` / `data_stream` deleted. Data-only flow via
  `SubscriptionItemStreamExt::filter_data()` extension trait. Sync surface kept
  asymmetric on purpose (inherent `next()` / `iter_data()` + `IntoIterator`).
  Full sweep of async examples / docs / tests landed in the same PR. See
  CLAUDE.md rule 24 for the consumer idiom.

- [x] **Notice classification helpers.** Shipped 2026-05-10 in PR #551. Adds
  `Notice::is_order_rejection()` (range 200–399), `Notice::category() ->
  NoticeCategory`, and a `#[non_exhaustive]` `NoticeCategory` enum
  (`Cancellation, Warning, SystemMessage, OrderRejection, Error`) with
  documented precedence chain. Disjoint partition resolves the 202 overlap
  (cancellation > rejection range). Mirrors the `OrderStatusKind::is_terminal()`
  pattern from PR #518.

- [x] **`OrderStatus.status: String` → `OrderStatusKind` enum.** Shipped in PR #518
  (commit `b9ed884`). `src/orders/mod.rs:1557` is `pub status: OrderStatusKind` with
  `is_terminal()` etc. Examples now use `.is_terminal()` (lines 61, 143).

- [~] **Continue the typed-status sweep.** Tracked in
  [`plans/typed-status-sweep.md`](typed-status-sweep.md). Audit complete; 5 PRs
  staged (ComboLeg.action → Contract.right → Contract.security_id_type →
  ExecutionFilter.side → Execution.side via live-diagnostic split). Note:
  the parent's original vocab claim for `Execution.side` was wrong —
  `BOT`/`SLD` is the wire (`Buy/Sell/SShort/SLng` belongs to `Action`); the
  tracker has the corrected analysis.
  - PR 1 shipped (PR #556) — `ComboLeg.action: LegAction` + shared
    `parse_required` / `parse_optional` helpers in `proto/decoders.rs`.
  - **PR 2 scope add: modernize `OptionRight` tests.** The existing
    `OptionRight` per-variant asserts at `src/contracts/types_tests.rs:107-114`
    are hand-rolled; PR 1's new `LegAction` tests at `:117-149` use a
    table-driven loop (CLAUDE.md rule 21). PR 2 types `Contract.right` as
    `Option<OptionRight>` — fold the test-shape rewrite into that PR
    (rule 9, "modernize touched modules"). Drop the hand-rolled asserts,
    use the same loop-over-variants shape.

- [x] **One canonical `Subscription` import path.** Shipped in PR #571
  (`Subscription`): dropped `ibapi::client::Subscription`, sourced the prelude
  re-export directly from `crate::subscriptions::Subscription`, preserved
  `ibapi::client::blocking::Subscription` as the labelled sync-explicit path.
  Same shape applied to `SharesChannel` in the follow-up PR: dropped
  `ibapi::client::SharesChannel` and `ibapi::client::sync::SharesChannel`;
  canonical at `ibapi::subscriptions::SharesChannel`; `client::blocking::SharesChannel`
  preserved for the labelled sync-explicit path. (PRs #571, #572)

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

- [x] **Reorganize re-exports out of `orders` for non-order types.** Shipped
  2026-05-20. `TagValue` lives only at `ibapi::contracts::TagValue` (its
  definition site); the historical `pub use crate::contracts::TagValue;` in
  `src/orders/mod.rs` removed along with all internal/external callsites
  (scanner, market-data realtime, proto encoders, order-builder, examples,
  `docs/order-types.md`). Migration guide §19 documents the path move.

- [x] **Hide internal types from the public surface.** Shipped — PRs #574
  (`Client::stubbed` / `message_bus` async-side narrowed), #575
  (`pub mod proto` → `pub(crate)`), #577 (`pub mod messages` → `pub(crate)`;
  user-facing types lifted to crate root + prelude; `parser_registry`
  reachable via `#[doc(hidden)]` re-export for the recording example), and
  #581 (`StartupMessage::Other` removed, `ResponseMessage` narrowed to
  `pub(crate)`). Verification pass on 2026-05-17 confirmed no external
  reach via `ibapi::messages::*` or `ibapi::proto::*` (both fail with
  E0603 from a downstream test crate), and that `DecoderContext` /
  `StreamDecoder` are unreachable.

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

- [x] **Audit remaining `Error::Simple` / `Error::Message` callers.** Shipped
  across 6 PRs: PR-1 #584 (validation → `InvalidArgument`), PR-2 #585
  (EOF/no-response → `UnexpectedEndOfStream`), PR-3 #586 (server-version +
  protobuf-decode → typed variants), PR-4 #587 (datetime + message-type
  parse → `Parse` via new `parse_field`/`parse_proto` constructors),
  PR-5 #589 (cursor EOF + unexpected-response + decoder-mismatch;
  introduced `Error::eof_at`), PR-6 #590 (new
  `Error::ConnectionRejected(String)` variant for handshake refusal).
  §5.3 Parse-shape resolved Option 4 (no-index constructors) rather than
  changing the variant tuple. Net: ~80 sites typed, four factory helpers
  (`unexpected_response`, `parse_field`, `parse_proto`, `eof_at`) added
  to `errors.rs`.

- [x] **Distinguish "request rejected by server" from "transport error" in return
  types.** Shipped 2026-05-19 in PR #591. `Error::Message(i32, String)` replaced
  by `Error::Notice(Notice)`; the new variant carries the full typed `Notice`
  (code, message, `error_time`, `advanced_order_reject_json`) and exposes the
  same classification API as `SubscriptionItem::Notice` — `Notice::category()`,
  `is_order_rejection`, `is_warning`. `From<ResponseMessage>` and
  `From<DecodedError>` absorbed the variant change so the ~25
  dispatcher/decoder sites that emit `Err(Error::from(message))` needed no
  edits. Bonus: the projection now preserves `error_time` and
  `advanced_order_reject_json` that the old tuple dropped. Distinct from
  `Error::ConnectionRejected` (handshake-time refusal) and the transport
  variants.

- [x] **Revisit `Error::Parse(usize, String, String)` shape.** Resolved
  2026-05-17 with Option 4: keep the variant tuple, add no-index
  constructors `Error::parse_field(value, reason)` / `Error::parse_proto(field,
  reason)` / `Error::eof_at(i, label)` that absorb the `0` placeholder.
  Non-breaking; future promotion to `Option<usize>` or a struct variant
  remains possible behind the constructors. Shipped via PR-4 #587 +
  PR-5 #589 (see §5.1). Typed-status sweep
  ([`plans/typed-status-sweep.md`](typed-status-sweep.md)) PRs 2–5 should
  use the new constructors when they land.

## 6. Examples & docs

- [~] **Modernize every example to use the canonical happy path.** After the
  decisions above land, sweep `examples/sync/*.rs` and `examples/async/*.rs`:
  - `Contract::stock("AAPL").build()` not bare struct literals.
  - `client.order(c).buy(n).limit(p).submit()` for orders (drop `next_order_id`
    boilerplate where possible).
  - `while let Some(item) = stream.next().await` for streams.
  - No magic-number notice code comparisons.

  Status (2026-05-10): order construction sweep done — all `examples/{sync,async}/*.rs`
  + `examples/conditional_orders.rs` use the fluent path or are reframed as the
  advanced/offline layer. Stream-shape sweep done in PR #550 (async examples now
  use `subscription.next().await` / `filter_data()`). Market-data sweep still
  pending.

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

- [x] **`#[must_use]` on every builder and `Subscription`.** Shipped 2026-05-19.
  31 builders annotated (`ContractBuilder` + 8 typed contract builders
  Stock/Option/Futures/ContinuousFutures/Forex/Crypto/Spread/Leg, `OrderBuilder` +
  `BracketOrderBuilder`, `MarketDataBuilder`, `RealtimeBarsBuilder`, 14 algo
  builders, 6 condition builders; `ClientBuilder` already had it). Subscription
  surface: `Subscription` (sync/async), `NoticeStream` (sync/async),
  `DisplayGroupSubscription` (sync/async), `TickSubscription` (sync/async).
  Forgetting the terminator (`.build()` / `.submit()` / `.subscribe()` /
  `.next().await`) is now a compile-time warning. The new lint surfaced 9 real
  callsites (1 unit test + 8 integration cleanup-cancel paths) that intentionally
  drop the result; each got an explicit `let _ = ...` bind.

- [x] **No `block_on` in async paths.** Verified clean across `src/` on 2026-05-06
  (no `futures::executor::block_on` usages). Project rule (`CLAUDE.md` §10) keeps
  it that way.
