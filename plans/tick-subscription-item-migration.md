# Plan: migrate `TickSubscription<T>` to the `SubscriptionItem` shape (issue #675)

## Problem

`Subscription<T>` (PR #504) returns `Option<Result<SubscriptionItem<T>, Error>>`, so
errors surface through the `Err` arm. The historical-tick `TickSubscription<T>`
(sync + async) was missed in that pass and keeps the old error-storing pattern:

- **sync** `src/market_data/historical/sync.rs:399` — private `error: Mutex<Option<Error>>`
  (no public accessor), `next() -> Option<T>`. On transport error `fill_buffer`
  calls `set_error(e)` then returns `Err(())`, which `next_helper` maps to `None`
  (`:488-491`, `:512`). Stored-but-unobservable.
- **async** `src/market_data/historical/async.rs:418` — `error: Option<Error>`,
  `next() -> Option<T>`. Worse: `set_error` is `#[allow(dead_code)]` and
  `fill_buffer` does `Some(Err(_)) => Err(())` (`:490`) — the error is **dropped
  on the floor**, never stored.

Result: a short tick batch from an error is indistinguishable from end-of-data.

## Gap audit (the "other subscription gaps" check)

Swept every `*Subscription` struct and every `next()/iter` that yields bare data:

| type | status |
|------|--------|
| `Subscription<T>` (sync/async) | ✅ migrated in #504 |
| `TickSubscription<T>` (sync/async) | ❌ **the only holdout — this plan** |
| `DisplayGroupSubscription` (sync/async) | ✅ `Deref`s to `Subscription<DisplayGroupUpdate>`, inherits migrated API |
| `ScannerSubscription` (`scanner/mod.rs`, `proto/protobuf.rs`) | n/a — request-param struct, not a stream handle |
| `NoticeStream` (sync/async) | n/a — already a notice-only stream |

`grep` confirms `error: Mutex<Option<Error>>` / `error: Option<Error>` and
`set_error`/`clear_error` exist **only** in the two `TickSubscription` files.
So #675's scope is complete: sync + async `TickSubscription`, nothing else.

## Transport feasibility (Notice arm)

`TickSubscription` currently pulls from `InternalSubscription::next()` /
`AsyncInternalSubscription::next()`, which call `RoutedItem::into_legacy()`
(`subscriptions/common.rs:75`): `Notice → None` (dropped), `Error → Some(Err)`.
That is exactly why notices vanish and errors get swallowed.

Both transports already expose the routed API the regular `Subscription` uses:
- sync: `next_routed()` / `try_next_routed()` / `next_timeout_routed()` (`transport/mod.rs:99-111`)
- async: `next_routed()` / `try_next_routed()` (`transport/async.rs:157,170`)

So the full `SubscriptionItem` shape (Data + Notice + terminal Error) is
reachable — switch the tick fill loops from `next()` to `next_routed()` and
handle `RoutedItem` directly, mirroring `Subscription::handle_response`
(`subscriptions/sync.rs:158-195`).

## Design

Reuse the existing shared machinery — **do not** invent a parallel item type:
- `SubscriptionItem<T>` and `filter_notice` from `subscriptions::common`
- `SubscriptionItemIterExt::filter_data` (sync) / `SubscriptionItemStreamExt::filter_data` (async)

A wrinkle: `TickDecoder::decode` returns `(Vec<T>, done)` — a *batch* per wire
message, not one item. Keep the existing `VecDeque<T>` buffer. The shape becomes
"drain buffered `Data` items, then on the next routed pull emit `Notice`/`Err`
or refill". Concretely:

- Buffered tick → `Some(Ok(SubscriptionItem::Data(t)))`.
- `RoutedItem::Response` that is `T::MESSAGE_TYPE` → decode batch, push to buffer, loop.
- `RoutedItem::Response` other type → skip (log `debug!`), loop.
- `RoutedItem::Notice(n)` → `Some(Ok(SubscriptionItem::Notice(n)))` (stream stays open).
- `RoutedItem::Error(EndOfStream)` / `None` / `done` flag → `None`.
- `RoutedItem::Error(e)` → set `done`, `Some(Err(e))`.

Also fix the latent `T::decode(&message).unwrap()` panic (`sync.rs:500`,
`async.rs:481`) — route a decode error to `Some(Err(_))` instead of panicking,
now that the `Err` arm exists.

### SRP refinements (from distillation review)

- **Split classification from mutation.** Factor a pure
  `classify(RoutedItem) -> TickAction` (Data-batch(Vec<T>,done) / Skip / Notice /
  EndOfStream / Error) out of `fill_buffer`. `fill_buffer` (and the async loop)
  then only apply the action: extend the buffer, set flags, return the envelope.
  The classification is the part with the bug history (swallowed Err, dropped
  Notice), so isolating it makes it unit-testable without a live buffer/transport.
- **Don't overload `done`.** Keep `done` = decoder-signaled completion; add a
  *separate* terminal flag (`stream_ended`) set on error/EndOfStream, mirroring
  `Subscription`'s split of `stream_ended` vs `snapshot_ended`. Overloading one
  bool to mean both "decoder finished" and "errored, stop polling" gives it a
  misleading name.

## Changes

### 1. sync — `src/market_data/historical/sync.rs`
- Drop `error: Mutex<Option<Error>>` field, `set_error`, `clear_error`.
- `next` / `try_next` / `next_timeout` → `Option<Result<SubscriptionItem<T>, Error>>`,
  driven by `next_routed` / `try_next_routed` / `next_timeout_routed`.
- Add `next_data` (filters notices → `Option<Result<T, Error>>`).
- `next_helper`/`fill_buffer` reworked to consume `RoutedItem` and return the
  envelope; decode errors → `Err` instead of `unwrap()`.
- Iterators `TickSubscriptionIter` / `OwnedIter` / `TryIter` / `TimeoutIter`:
  `Item = Result<SubscriptionItem<T>, Error>`. `IntoIterator` (both `&` and owned)
  updated to match.
- Add data-only adapters `iter_data` / `try_iter_data` / `timeout_iter_data`
  returning `FilterData<…>` (reuse `subscriptions::sync::{FilterData, SubscriptionItemIterExt}`).
- Track terminal state with the existing `done` `AtomicBool` (plus set it on error)
  so post-error calls return `None`, matching `Subscription::stream_ended`.

### 2. async — `src/market_data/historical/async.rs`
- Drop `error: Option<Error>`, `set_error`, `clear_error`.
- `next(&mut self) -> Option<Result<SubscriptionItem<T>, Error>>`, driven by
  `next_routed`. Add `try_next` via `try_next_routed`.
- Add `filter_data()` returning a `FilterDataStream`-style data-only stream, or
  a `next_data()` convenience — mirror whichever surface the async
  `Subscription<T>` exposes (it implements `Stream`; decide below).
- Async `TickSubscription` currently has **no** `Stream` impl / iterators (only
  `next().await`). Decision: implement `futures::Stream<Item = Result<SubscriptionItem<T>, Error>>`
  so it composes with `StreamExt` and `filter_data()` exactly like
  `Subscription<T>` — this is the consistency the issue asks for. `next().await`
  stays as an inherent convenience returning the same envelope.
- Decode error → `Err` arm instead of `unwrap()`.

### 3. Tests (per rule 6 / rule 10 — exercise production code)
- **sync** `sync_tests.rs`: replace the existing `set_error → next None` test
  (`:1298,:1313`) with one asserting a mid-stream `RoutedItem::Error` surfaces as
  `Some(Err(_))` and subsequent `next()` is `None`. Add: notice passes through as
  `SubscriptionItem::Notice`; `iter_data()` filters notices and still yields the
  `Err`. Use `MessageBusStub` with an ordered response queue containing a tick
  batch + an injected error / notice frame.
- **async** `async_tests.rs`: mirror — error surfaces via `Err`, not silent
  `None`; notice via `Notice` arm; `filter_data()` drops notices, keeps `Err`.
  This is the regression guard for the silent-drop (`async.rs:490`).
- Round-trip a real tick batch through `T::decode` (production decoder) so the
  test traverses production code, not a self-loop.
- Run `just cover` on `market_data/historical`; keep ≥90%.

### 4. Examples (4 sync + 3 async)
`for tick in ticks { … }` / `while let Some(tick) = sub.next().await` now yield
`Result<SubscriptionItem<T>, Error>`. Update each to either:
- `for item in ticks { match item { Ok(SubscriptionItem::Data(t)) => …, Ok(SubscriptionItem::Notice(n)) => …, Err(e) => { eprintln!("{e}"); break } } }`, or
- the data-only adapter `for t in ticks.iter_data() { let t = t?; … }` /
  `let mut data = sub.filter_data(); while let Some(t) = data.next().await { let t = t?; … }`.

Show the canonical happy path with explicit error handling (the whole point of
the fix). Files: `examples/sync/historical_ticks_{trade,mid_point,bid_ask}.rs`,
`examples/async/historical_ticks{,_trade,_midpoint}.rs`.

### 5. Doc-examples on the public API (rule 18)
Every touched `pub fn` (the builder terminals `.trade()/.mid_point()/.bid_ask()`
return `TickSubscription`, plus `next`/`iter`/`iter_data`/etc.) gets a
`# Examples` block showing the envelope match or `iter_data()` form.

### 6. Docs — README + migration guide (mandatory, same PR)
- `docs/migration-3.0.md`: extend the §1 `SubscriptionItem` note (line 7) to say
  `TickSubscription<T>` now shares the shape, with a before/after tick snippet.
  The §25 `historical_ticks` builder section and the line-466 "Subscription
  handles" bullet should cross-link.
- `README.md`: grep shows no tick snippet today; if the canonical example list
  mentions ticks, update — otherwise no change. Re-grep before finalizing.

## Verification (all three feature builds — rule 1)
```
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
just test           # default-async
cargo test --no-default-features --features sync
cargo test --all-features
cargo build -p ibapi-integration-sync  --tests   # touches a Subscription-family wire surface
cargo build -p ibapi-integration-async --tests
just cover          # ≥90% on touched module
```

## PR shape
Single PR (sync + async together, as the issue requests — both features must
stay green anyway). v3 breaking change. Branch off `main`, open via PR.
Not split: the change is mechanical-symmetric across the two files and the
shared `SubscriptionItem` machinery already exists, so there's no "modernize
callers first" precursor needed (rule 23 doesn't apply — no new compile-time
restriction, just a return-type widening with all call sites in-repo).

## Async surface — DECIDED
Full `futures::Stream` + `filter_data()` parity with `Subscription<T>`
(user-confirmed). `next().await` stays as an inherent convenience returning the
same envelope.

## Distillation review (duplication / SRP / composability)

- **Iterator family duplication (accept + follow-up).** The 4 new tick iterator
  structs mirror the 4 `Subscription*Iter` structs. The issue requires the full
  `iter/try_iter/timeout_iter` surface, so the fix is *not* fewer methods but a
  generic family over an internal `EnvelopeSource` trait backing both types —
  a cross-cutting refactor of existing `Subscription` code. Out of scope for this
  breaking-change PR (rule 23); mirror-accept here (rule 13 acceptable mirror),
  open a follow-up issue for the generic consolidation.
- **Routed-classification duplication (accept, do not extract).** The
  Notice/Error/EndOfStream mapping repeats `Subscription::handle_response`, but
  the Response arm differs (single-item decode vs batch+buffer). A shared helper
  would need a Response callback — more indirection than the ~5 shared lines save
  (rule 25). Keep separate.
- **SRP (folded into Design above).** `classify` split out of `fill_buffer`;
  `done` not overloaded — separate `stream_ended` flag.
- **Composability (good, no change).** Reuses `SubscriptionItem`, `filter_notice`,
  `FilterData`/`FilterDataStream`, `*Ext` traits; async `Stream` impl composes
  with `StreamExt`. No parallel item type, no bespoke filter logic.
