# v3.0 Public API Ergonomics — Tracking Doc

A living checklist of public-API rough edges to address before 3.0 ships. Goal:
the API should feel **simple, ergonomic, easy to use, and intuitive** — minimal
ceremony, no stringly-typed escape hatches, one obvious way to do each thing.

## How to use this doc

- One bullet per concrete change. Keep them small and independently shippable.
- Each item: **status · problem · proposal · breaking? · notes**.
- Status: `[ ]` open · `[~]` in progress · `[x]` shipped · `[-]` rejected (note why).
- When an item ships, leave it checked here with the PR link for ~one cycle, then prune.
- File a separate `todos/<topic>.md` for any item large enough to need its own plan
  and link it from here.

Related existing tracking docs in `todos/`:
- `algo-order-builders.md`, `generic-tick-types.md`, `legacy-text-protocol-cleanup.md`,
  `protobuf-migration.md`, `warning-message-routing.md`.

---

## 1. Construction & builders

- [ ] **Forbid bare `Contract { ... }` construction.** Today `Contract::stock(...).build()`
  is the blessed path, but examples (e.g. `examples/async/place_order.rs:22`) still
  build the struct field-by-field with `..Default::default()`. Fields are `pub`, so
  there's no compile-time push toward the builder.
  - Proposal: make required fields private (or wrap in newtypes that only the builder
    can construct), keep `pub` on getters; or `#[non_exhaustive]` + private constructor.
  - Breaking: yes (intentional for 3.0).

- [ ] **Newtype ergonomics: take `impl Into<Symbol>` / `&str` everywhere.**
  `Symbol::from("AAPL")`, `Exchange::from("SMART")`, `Currency::from("USD")` shows up
  in every example. Builder methods and constructors should accept `impl Into<_>` so
  callers can pass string literals directly.
  - Audit: `Symbol`, `Exchange`, `Currency`, `Cusip`, `Isin`, `BondIdentifier`,
    `ContractMonth`, `ExpirationDate`, `Strike`.

- [ ] **Converge order construction on one style.** Two coexisting paths today:
  - `order_builder::limit_order(Action::Buy, 100.0, 150.0)` (free fn, returns `Order`)
  - `client.order(&c).buy(100).limit(150.0).submit()` (fluent, owns submission)

  Pick the fluent one as canonical, keep the free fns as a thin convenience layer
  documented as "advanced — bring your own order id." Remove the per-method
  `Action::Buy` argument once the side is implied by `.buy()` / `.sell()`.

- [ ] **Drop `client.next_order_id()` from the canonical happy path.** `submit()` already
  allocates an id internally; the only caller that still needs `next_order_id()` is
  the low-level `place_order(order_id, contract, order)` form. Either:
  - keep `next_order_id()` for advanced callers but stop showing it in examples; or
  - hide it behind `client.advanced()` / a feature flag and have `place_order` accept
    `Option<i32>`.

## 2. Streaming surface

- [ ] **`SubscriptionItem<T>` consistency.** Per recent PR 3/PR 4 refactors, every
  per-T `Notice` variant (`PlaceOrder::Message`, `Orders::Notice`, …) is unreachable
  in production — notices route through `SubscriptionItem::Notice` and the dedicated
  `NoticeStream`. Tests still hit them via `MessageBusStub`.
  - Action: migrate the remaining tests off `MessageBusStub`-bypass, then delete the
    per-T `Notice` variants. Tracked separately in memory; surface here so it lands
    before 3.0.
  - Linked: `todos/warning-message-routing.md`.

- [ ] **Standardize the consumer interface on `Stream` (async) and `Iterator` (sync).**
  Today consumers call `subscription.next_data().await` (see
  `examples/async/place_order.rs:47`). For async this should be `StreamExt::next`,
  and for sync the `for item in subscription` form should be the default in examples.
  - Decision needed: keep `next_data()` as a thin alias, or remove and force `Stream`?
  - Breaking: yes if we remove.

- [ ] **Notice classification helpers.** Examples do `notice.code >= 200 && notice.code < 300`
  to decide "rejected/cancelled" (see `place_order.rs:92`). Provide:
  - `Notice::is_warning()`, `Notice::is_order_rejection()`, `Notice::category() -> NoticeCategory`.
  - A range-keyed enum or table-driven classifier so callers never reach for magic
    numbers.

- [ ] **Replace stringly-typed status fields with enums.**
  - `OrderStatus.status: String` compared against `"Filled"`, `"Cancelled"`, … in every
    order example. Introduce `OrderStatus::state: OrderState` enum, keep `status: String`
    only as a fallback for unknown values (or drop it entirely).
  - Same audit for `OrderState.status`, contract `secIdType`, exec `side`, etc.

- [ ] **One canonical `Subscription` import path.** `Subscription` is reachable from
  `crate::client::Subscription`, `crate::subscriptions::Subscription`, and
  `crate::prelude::Subscription`, with feature-gated divergence between `client::sync`
  and `client::r#async`. Pick `crate::subscriptions::Subscription` as canonical and
  keep the others as `pub use` aliases (or remove the client-level paths).

- [ ] **`NoticeStream` should not mirror `Subscription`'s sync/async toggle in the
  prelude.** Today the prelude conditionally re-exports a sync vs async `NoticeStream`.
  Either expose distinct `NoticeStream` / `BlockingNoticeStream` types, or keep a single
  type whose API is the same shape and only differs in `await`.

## 3. Naming, layout, prelude

- [ ] **Eliminate prelude collisions.** `BarSize` and `WhatToShow` exist for both
  historical and realtime market data and are re-exported as `HistoricalBarSize` /
  `RealtimeBarSize` in the prelude (`src/prelude.rs:31-34`). Options:
  - Rename one (e.g. `RealtimeBarSize` already differs in variants — rename in source
    too, drop the alias).
  - Or keep the aliases but document them as the canonical names.

- [ ] **Async-vs-blocking naming asymmetry.** `ibapi::Client` is the async client when
  `async` is on; the sync client lives at `ibapi::client::blocking::Client`. Consider
  symmetric paths (`client::async::Client` + `client::blocking::Client`) and a
  feature-driven re-export at the crate root, so docs and examples can refer to
  either by an obvious path.

- [ ] **Reorganize re-exports out of `orders` for non-order types.** `TagValue` is
  re-exported from `orders` (`src/orders/mod.rs:67`) for historical reasons. Move to
  `contracts` (or wherever it logically belongs) and drop the alias.

- [ ] **Hide internal types from the public surface.** Audit `pub` items that look
  like plumbing:
  - `Client::message_bus()`, `Client::stubbed()` (currently `pub`)
  - `subscriptions::common::SubscriptionItem` (re-exported at module root — fine, but
    confirm `DecoderContext`, `StreamDecoder` stay `pub(crate)`)
  - `pub mod messages` and `pub mod proto` — confirm what consumers actually need
    versus what's just exposed for tests/examples; consider `#[doc(hidden)]` for the
    advanced bits.

## 4. Connection API

- [ ] **Fold connect variants into a builder.** Today there are three:
  `connect`, `connect_with_callback`, `connect_with_options`. Replace with:
  ```rust
  Client::builder("127.0.0.1:4002", 100)
      .startup_callback(cb)
      .options(opts)
      .connect()
      .await?;
  ```
  Keep `Client::connect(addr, id)` as the one-liner; deprecate the rest.

- [ ] **`StartupMessageCallback` ergonomics.** Currently `Box<dyn Fn(...)>`. Accept
  `impl Fn(...) + Send + 'static` and box internally so the call site doesn't need
  the `Box::new(...)` ceremony shown in `lib.rs:73`.

## 5. Errors

- [ ] **Audit `Error` variants for actionable matching.** Per memory, `main` already
  has typed variants vs. `v2-stable`'s `Error::Simple`. Double-check every place that
  still funnels into `Error::Simple` / `Error::Message` and split into typed variants
  where downstream code would plausibly match (auth failure, version mismatch,
  connection lost, request timeout, request rejected by TWS, …).

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

- [ ] **`MIGRATION.md` for 2.x → 3.0.** Start drafting alongside changes — every item
  here that's marked breaking needs a one-paragraph entry.

- [ ] **Consolidate the docs index.** `docs/api-patterns.md` and `docs/contract-builder.md`
  overlap. Merge or cross-link cleanly so the prelude + builders are documented in one
  obvious place.

## 7. Cross-cutting

- [ ] **One way to spell each thing.** After the moves above, audit for duplicate
  re-exports (`pub use foo::Bar;` in two modules). The crate root, `prelude`, and the
  domain module should each pick one home for each type.

- [ ] **`#[non_exhaustive]` on every public enum and struct that may grow.** Avoids
  future breaking releases for additive variants/fields. Sweep before 3.0 cuts.

- [ ] **`#[must_use]` on every builder and `Subscription`.** Forgetting `.subscribe()`
  / `.submit()` / `.build()` should produce a lint, not silent no-ops.

- [ ] **Remove `block_on` audit (already a project rule).** Confirm no remaining
  `futures::executor::block_on` in async paths before 3.0.
