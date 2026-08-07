---
id: dual-feature-types
title: Dual-feature public types use per-feature submodules and an async-preferring alias
cluster: parity
status: active
triggers:
  - adding a public type with distinct sync and async impls
  - two same-name structs collide under --all-features
  - writing a trait method or Client method that returns a per-feature type
  - a path containing the async keyword fails to parse
symbols: [NoticeStream, sync_impl, async_impl, Subscription, r#async]
related: [feature-matrix, no-parity-wrappers, subscription-consumer-idiom]
precedents: ["#512", "#526"]
memory: [feedback_raw_identifier_async_path, feedback_plan_cfg_receiver_types]
---

A public type with distinct sync/async impls — different receivers, different async-ness —
follows the `NoticeStream` pattern in `src/subscriptions/notice_stream.rs`:

1. Two submodules in one file: `pub mod sync_impl` and `pub mod async_impl`.
2. A top-level alias that **prefers async** when both features are on:
   ```rust
   #[cfg(feature = "async")]
   pub use notice_stream::async_impl::NoticeStream;
   #[cfg(all(feature = "sync", not(feature = "async")))]
   pub use notice_stream::sync_impl::NoticeStream;
   ```
3. The sync version also re-exported at `client::blocking::*`, so a both-features build can
   still name it.

Trait method return types and `Client::*` signatures must spell out the full per-feature path
(`crate::subscriptions::notice_stream::async_impl::NoticeStream`), not the alias.

## Why

Naive same-name sibling structs gated `#[cfg(feature = "sync")]` / `#[cfg(feature = "async")]`
compile fine in each single-feature build and then collide under `--all-features`. The alias
indirection is what lets both types exist at once while one name stays canonical, so
`cargo check --all-features` is the check that matters here — see
[feature matrix](feature-matrix.md), whose CI legs do not cover it.

`async` is a Rust keyword, so any path crossing that module needs the raw identifier:
`crate::subscriptions::r#async::Subscription`. Plain `async::` is a parse error, not a
resolution error — the compiler message points at syntax and reads as unrelated.

Two `#[cfg]`-gated impl blocks on **different receiver types** coexist legally; they are not
mutually exclusive alternatives. Plans that assume otherwise call for needless
feature-splitting, so state the receiver types explicitly when scoping this work.

`Subscription` predates the convention: same dual-export idea, but flat
`subscriptions/sync.rs` + `subscriptions/async.rs` files instead of `sync_impl` / `async_impl`
submodules. Don't copy that file layout for new types.

## Precedents

- #512 — introduced the global `NoticeStream` for unrouted IB notices, establishing the
  per-feature submodule split.
- #526 — unified the notice API on `Client::builder()`, which is where the async-preferring
  alias and the `client::blocking::*` mirror settled into their current shape.
