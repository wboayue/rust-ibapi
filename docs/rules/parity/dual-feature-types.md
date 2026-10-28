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
precedents: ["#512", "#526", "#823", "#916", "#957", "#958"]
memory: [feedback_raw_identifier_async_path, feedback_plan_cfg_receiver_types]
---

A public type with distinct sync/async impls (different receivers, different async-ness) follows
the `NoticeStream` pattern:

1. Two submodules in one file, `pub mod sync_impl` and `pub mod async_impl`
   (`src/subscriptions/notice_stream.rs`).
2. A top-level alias that **prefers async** when both features are on (`subscriptions/mod.rs`):
   ```rust
   #[cfg(feature = "async")]
   pub use notice_stream::async_impl::NoticeStream;
   #[cfg(all(feature = "sync", not(feature = "async")))]
   pub use notice_stream::sync_impl::NoticeStream;
   ```
3. The sync version re-exported at `client::blocking::*`, so a both-features build can still name
   it.

Trait method return types and `Client::*` signatures spell out the full per-feature path
(`crate::subscriptions::notice_stream::async_impl::NoticeStream`), not the alias.

## Why

Same-name sibling structs gated `#[cfg(feature = "sync")]` / `#[cfg(feature = "async")]` compile
in each single-feature build and collide under `--all-features`. The alias lets both exist while
one name stays canonical, so `cargo check --all-features` is the check that matters (the
`all-features` leg of the [feature matrix](feature-matrix.md)).

`async` is a keyword, so paths through that module need the raw identifier:
`crate::subscriptions::r#async::Subscription`. Plain `async::` is a parse error whose message
reads as unrelated.

Two `#[cfg]`-gated impl blocks on **different receiver types** coexist legally; they are not
mutually exclusive. Plans that assume otherwise call for needless feature-splitting, so state the
receiver types explicitly when scoping.

`Subscription` predates the convention: flat `subscriptions/sync.rs` + `subscriptions/async.rs`
instead of submodules. Don't copy that layout. Both halves share one shape,
`Subscription<T: StreamDecoder<T>>` dispatching statically on `T` with `#[allow(private_bounds)]`
(`StreamDecoder` is `pub(crate)`). A divergence in a dual-feature type's impl reads as deliberate
long after it stopped being so: bring the halves together rather than documenting the gap.

One divergence is deliberate: after `cancel()` both halves yield `Err(Error::Cancelled)` and end,
but the blocking side first yields what was already queued, while every async clone stops at its
next poll through a shared flag (#894, #916). Don't "fix" either side to match.

## Precedents

- #512 — global `NoticeStream` for unrouted notices; established the per-feature submodule split.
- #526 — unified the notice API on `Client::builder()`; the async-preferring alias and
  `client::blocking::*` mirror settled into their current shape.
- #823 — closed the last structural divergence between the `Subscription` halves: the async one's
  type-erased decoder and `PreDecoded` arm (for an uncalled `pub fn new(receiver)`) forced
  `message_bus: Option<_>` and a `panic!` in `Clone`. Binding on `T: StreamDecoder<T>` deleted
  both (+422 −689). The dead constructor was public, so removing it still cost a migration-guide
  section.
- #916 — cancel always ends the subscription; the queued-vs-immediate split above.
- #957 — `AccountSummarySnapshots` in both halves: blocking named via `client::blocking`,
  top-level name async.
- #958 — extended the `client::blocking` mirror to `TickSubscription` (+ its four iterators) and
  `DisplayGroupSubscription`, which had no blocking path in a both-features build.
