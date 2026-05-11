# Standardize the Streaming Consumer Interface

Parent tracker: [v3-api-ergonomics.md](v3-api-ergonomics.md) § 2 "Streaming surface" (item 2).

## End state

| Side  | Public surface                                                                                  |
|-------|-------------------------------------------------------------------------------------------------|
| Async | `Subscription<T>: Stream<Item = Result<SubscriptionItem<T>, Error>>`. No inherent `next` / `next_data` / `stream` / `data_stream`. Callers go through `StreamExt`. Data-only flow via `SubscriptionItemStreamExt::filter_data()` extension trait. |
| Sync  | Unchanged shape: inherent `next` / `try_next` / `next_timeout` / `next_data` **kept**, plus `IntoIterator` (yields `Result<SubscriptionItem<T>, Error>`), plus `iter` / `iter_data` / `try_iter` / `timeout_iter` adapters. |

Async ends with **one spelling** for both one-shot and loop:

```rust
let item = subscription.next().await;                              // one-shot (StreamExt::next)
while let Some(r) = subscription.next().await { ... }              // loop
let mut data = (&mut subscription).filter_data();                  // data-only stream
let first8 = subscription.take(8).try_collect::<Vec<_>>().await?;  // combinators
```

Sync stays asymmetric on purpose — `&self` inherent `next()` is genuinely cheaper for tests than `(&sub).into_iter().next()` (which needs a mut iterator binding); `IntoIterator` already exists for loops.

## Why a single PR

Workspace stays green throughout because we delete the inherent async methods and add the `Stream` impl in the same commit. There is no halfway state where 158 callers would fail to compile if split.

## Implementation phases (single PR, ordered commits)

### Commit 1 — `Subscription<T>: Stream` for async

**Strategy:** wrap `tokio::sync::broadcast::Receiver` in `tokio_stream::wrappers::BroadcastStream<RoutedItem>`. `BroadcastStream` already gives us `Stream<Item = Result<RoutedItem, BroadcastStreamRecvError::Lagged>>` with a working poll machine, so `Subscription::poll_next` becomes a synchronous decode-and-classify wrapper.

Files:

- **`Cargo.toml`** — add `tokio-stream = { version = "0.1", features = ["sync"], optional = true }` to the `async` feature set. (`sync` feature gates `BroadcastStream`.)
- **`src/transport/async.rs`** — `AsyncInternalSubscription`:
  - Field change: hold `BroadcastStream<RoutedItem>` instead of (or alongside) `broadcast::Receiver<RoutedItem>`. Keep the raw `Receiver` only if needed for `Clone`'s `resubscribe()` call (`src/transport/async.rs:78`); preference is to plumb the `Sender` reference through so Clone builds a fresh receiver + stream from the sender end.
  - Keep `next_routed(&mut self) -> Option<RoutedItem>` as a `pub(crate)` helper that drives the inner Stream — **kept because `src/transport/async_tests.rs` stub fixtures call it directly** (6 sites at lines 231, 248, 260, 274, 311, 742); not transitional. It becomes a thin `self.stream.next().await` wrapper that filters out `Err(Lagged)` (matches today's `continue` arm at `src/transport/async.rs:119,132`).
- **`src/subscriptions/async.rs`** — restructure `Subscription<T>`:
  - **Delete** inherent `pub async fn next` (line 226), `pub async fn next_data` (line 284), `pub fn stream` (line 301), `pub fn data_stream` (line 318). Delete the note at line 387-391.
  - **Collapse the three near-identical constructors.** `Subscription::with_decoder` (line 103), `Subscription::new_with_decoder` (line 134), `Subscription::with_decoder_components` (line 151) are byte-for-byte the same body; the latter two only exist to be called by tests named after them in `src/subscriptions/async_tests.rs` (`test_subscription_new_with_decoder` line 58, `test_subscription_with_decoder_components` line 78) — self-loop tests per CLAUDE.md rule 10. Drop the two wrappers + their two self-loop tests in this commit (CLAUDE.md rule 9 — already touching this module).
  - **Add** `impl<T: Send + 'static> futures::Stream for Subscription<T>` with `Item = Result<SubscriptionItem<T>, Error>`. The `poll_next` body is a translation of today's `next()` loop body:
    1. Check `stream_ended` short-circuit (same as line 230).
    2. Dispatch on `inner`:
       - `WithDecoder { stream, decoder, context }` — `match ready!(Pin::new(stream).poll_next(cx))` and apply the existing `ProcessingResult` arms (lines 244-258).
       - `PreDecoded { receiver }` — `match ready!(receiver.poll_recv(cx))` and apply the existing pre-decoded arm (lines 271-277). `mpsc::UnboundedReceiver` has a public `poll_recv`; no wrapper needed.
    3. The `Skip` / `Lagged` cases `continue` the outer loop (call `cx.waker().wake_by_ref()` is **not** needed — the inner stream's wake fires when more data arrives; we just loop within this poll only to drain immediately-ready items).
  - Pin projection: use `pin_project_lite::pin_project!` (lightweight, no proc macro) over the struct, OR hand-rolled `unsafe { self.get_unchecked_mut() }` since only `inner` needs pin-projection (atomics and `Option<Arc<…>>` are `Unpin`). Pick `pin_project_lite` for the new dep aligning with project style.
  - `Cargo.toml` — add `pin-project-lite = "0.2"` to the `async` feature set.
- **`src/subscriptions/common.rs` or `src/subscriptions/async.rs`** — add the data-only adapter mirroring `SubscriptionItemIterExt` (`src/subscriptions/sync.rs:321-330`):

  ```rust
  #[cfg(feature = "async")]
  pub trait SubscriptionItemStreamExt: Stream + Sized {
      fn filter_data<T>(self) -> FilterDataStream<Self>
      where Self: Stream<Item = Result<SubscriptionItem<T>, Error>>;
  }
  ```

  `FilterDataStream<S>` is the `Stream` analogue of the existing `FilterData<I>` iterator at `src/subscriptions/sync.rs:296-314` — same `filter_notice` (`src/subscriptions/common.rs`) under the hood.
- **`src/subscriptions/mod.rs` + `src/prelude.rs`** — re-export `SubscriptionItemStreamExt` next to the existing `SubscriptionItemIterExt` (`src/prelude.rs:54` area).

**Tests for this commit:**

- New test in `src/subscriptions/async_tests.rs`: `subscription_impls_stream` — pump a few `RoutedItem`s through a stubbed bus, consume via `StreamExt::next` and via `(&mut sub).take(N).collect()`. Verify Lagged is skipped.
- New test: `filter_data_stream_drops_notices` — feed Data + Notice + Data, verify only Data items reach the consumer, Notice is logged at `warn!` via `filter_notice`.
- New test: `pre_decoded_subscription_polls` — exercise `SubscriptionInner::PreDecoded` (matches today's `Subscription::new(receiver)` constructor at line 200).
- Existing `src/subscriptions/async_tests.rs` cases that call `.next_data()` get rewritten in commit 2 (mechanical).

### Commit 2 — Sweep async call sites (`src/`, `examples/async/`, `docs/`, `README.md`)

Mechanical rewrite. Strict transform rules to avoid 158 per-site judgment calls:

**Tests (`src/**/*tests.rs`)** — always rewrite to assert on `SubscriptionItem::Data` explicitly:

```
let order_data = subscription.next_data().await;
//   ↓
let next = subscription.next().await;
assert_matches!(next, Some(Ok(SubscriptionItem::Data(_))));
// or: let Some(Ok(SubscriptionItem::Data(order_data))) = subscription.next().await else { panic!(...) };
```

Rationale: surfacing notices in tests catches issues `next_data()` previously hid (memory: `examples_expose_test_gaps`). Tests **never** use `filter_data()` — the filter is the API we're deleting; recreating it in tests under a different name defeats the migration.

**Examples (`examples/async/*.rs`)** — use `subscription.next().await` if the example's purpose is demonstrating the streaming surface; use `(&mut subscription).filter_data().next().await` only when surfacing notices would muddy the teaching point (e.g. a "show me 8 bars" demo).

**Threshold check** — if Commit 2 finds **more than 5 tests** genuinely need `filter_data()` to stay green (i.e. they care about notice-filtered counts, not specific payload shapes), treat that as a signal to reconsider the API choice and bring `next_data()` back as an inherent shortcut before merging.

Files (158 sites total — split below by domain for the commit message body, not separate commits):

- `src/orders/async/tests.rs` (~14 sites)
- `src/accounts/async/tests.rs` (~12 sites)
- `src/accounts/async/mod.rs` (3 sites — rustdoc `# Examples` blocks)
- `src/market_data/realtime/async/tests.rs` (~7 sites)
- `src/market_data/realtime/builder.rs` (rustdoc example at line 107)
- `src/market_data/historical/{sync.rs, sync_tests.rs, async_tests.rs}` (rustdoc + tests)
- `src/display_groups/async.rs` (3 tests at lines 98, 116, 161)
- `src/news/async_tests.rs`, `src/scanner/async_tests.rs`, `src/wsh/async_tests.rs`, `src/transport/async_tests.rs`
- `src/orders/builder/async_impl.rs`
- `src/subscriptions/{async.rs, common.rs, async_tests.rs, notice_stream_tests.rs}` — own-module rustdoc cleanup
- `examples/async/*.rs` (~22 files using `.next_data()`)
- `examples/async/market_data.rs:165` — already uses `.stream().take(8)`; switch to `(&mut subscription).take(8)` (drop the `.stream()` call).
- `docs/api-patterns.md`, `docs/quick-start.md`, `docs/examples.md`, `docs/migration-3.0.md`, `README.md` — every fenced `rust` block referencing the old methods.

Mechanical risk: `.md` snippets aren't compile-checked (memory: `md_doc_snippets_rot_silently`; CLAUDE.md "Maintaining Documentation"). After grepping, **read each remaining hit and visually verify** the snippet compiles against the new API.

### Commit 3 — Sync example/doc sweep (canonical loop = `for item in subscription.iter_data()`)

Sync inherent methods are **kept**, so this commit is much smaller — only docs/examples that hand-roll `while let Some(...) = sub.next_data()` loops in places where `for item in sub.iter_data()` reads more naturally.

Target files:

- `examples/sync/positions.rs:18`, `examples/sync/scanner_subscription_*.rs` (2 files), `examples/record_interactions.rs:313,361`
- README.md and `docs/*.md` — any sync stream snippet
- Memory note `subscription_for_yields_subscription_item` is the gotcha: prefer `for item in subscription.iter_data()` (yields `Result<T, Error>`) over `for item in &subscription` (yields `Result<SubscriptionItem<T>, Error>`) in examples where the goal is data-only.

Sync test files (`src/**/sync/tests.rs`) — **leave alone**. They use `next_data()` for one-shot reads and that's the canonical shape per the sync surface decision.

### Commit 4 — Migration guide + parent plan

- **`docs/migration-3.0.md`** — new section documenting the async breaking change:
  - Old: `subscription.next_data().await` → New: `subscription.next().await` (now yields `SubscriptionItem<T>`).
  - Old: `subscription.stream()` → New: just use `subscription` (it *is* a Stream).
  - Old: `subscription.data_stream()` → New: `(&mut subscription).filter_data()`.
  - Note: needs `use futures::StreamExt;` to call `.next()` / `.take()` / etc.
  - **`&mut` gotcha for combinator reuse.** `subscription.take(8).collect()` consumes the subscription — you can't call `.cancel()` or further `.next().await` after. To keep the subscription usable, borrow: `(&mut subscription).take(8).collect()`. Standard `StreamExt` shape, but bites first-time users.

    ```rust
    // CONSUMES subscription (only do this if you don't need it after):
    let first8: Vec<_> = subscription.take(8).try_collect().await?;

    // BORROWS — subscription still usable afterwards:
    let first8: Vec<_> = (&mut subscription).take(8).try_collect().await?;
    subscription.cancel().await;
    ```
- **`plans/v3-api-ergonomics.md`** — mark the §2 item `[x]` shipped with PR link, leave the bullet for ~one cycle (per the plan-doc convention at the top of v3-api-ergonomics.md), then prune.

## Risks / open questions

1. **`Subscription: Clone` semantics under `Stream`.** Today `WithDecoder` clones via `AsyncInternalSubscription::clone()` which calls `broadcast::Receiver::resubscribe()` (`src/transport/async.rs:78`). With a stored `BroadcastStream`, clone has to rebuild the stream from a fresh receiver — easiest path is to keep the raw `Receiver` field AND build `BroadcastStream` on first poll (`Option<BroadcastStream>` lazily initialized). Decide during implementation; either way the clone is grep-able and rare (one production caller at `src/subscriptions/async.rs:72`).

2. **Notices reach more callers by default.** Previously, `next_data()` filtered notices and logged them at `warn!`. After the sweep, tests that switch to `subscription.next().await` will see `Ok(SubscriptionItem::Notice(_))` they didn't before. Each test rewrite needs to either:
   - Pattern-match on `SubscriptionItem::Data(_)` explicitly (best for assertions on payload shape), OR
   - Wrap with `(&mut subscription).filter_data()` to preserve the old behavior (best when the test is asserting on counts / iterations).

   This is the "examples expose test gaps" memory in action — the rewrite will likely surface a small number of tests that previously hid notices.

3. **`AsyncInternalSubscription::next_routed` callers.** The sync-mirrored sites that today call `next_routed().await` (only inside `Subscription::next` itself) all collapse into the new `poll_next`. Verify no other crate code reaches in — `Explore` agent or `grep -rn "next_routed" src/` to confirm before deleting.

4. **`pin-project-lite` vs hand-rolled `unsafe`.** New dep is small (~no codegen, no proc macro), already common in tokio ecosystem. Hand-rolled `unsafe { get_unchecked_mut() }` is acceptable but adds an audit surface. Recommend `pin-project-lite`.

5. **`tokio-stream` adds a transitive surface.** It's maintained by the tokio team and tracks tokio versions. Low risk; aligns with our existing tokio usage.

6. **Integration crates.** Per CLAUDE.md rule 11, this PR touches `Subscription`'s public shape — must run `cargo build -p ibapi-integration-sync --tests` and `-p ibapi-integration-async --tests` (+ matching clippy) before opening the PR. Sync-side integration tests should not change (sync surface preserved); async-side will need the same `.next_data() → .next()` sweep applied.

## Validation checklist (before PR open)

Per CLAUDE.md § Key Points 1, 7, 11, and "Quick Commands":

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
just test
cargo build -p ibapi-integration-sync  --tests
cargo build -p ibapi-integration-async --tests
cargo clippy -p ibapi-integration-sync  --tests -- -D warnings
cargo clippy -p ibapi-integration-async --tests -- -D warnings
just cover                # confirm touched modules stay ≥90%
```

Coverage rule (CLAUDE.md § Key Points 6): every new `pub` / `pub(crate)` function (`SubscriptionItemStreamExt::filter_data`, `FilterDataStream::<S>` `poll_next`, the new `Stream` impl) needs a unit test. The three tests sketched in Commit 1 cover the new surface; verify post-sweep that `src/subscriptions/` line coverage doesn't drop.

Doc-example rule (CLAUDE.md § Key Points 18): the new `impl Stream for Subscription<T>` block needs a rustdoc `# Examples` showing `while let Some(r) = sub.next().await` and a `(&mut sub).filter_data()` example. Mirror the structure of today's `Subscription` rustdoc at `src/subscriptions/async.rs:20-38`.

## Out of scope

- Sync API changes (intentional — sync stays as-is).
- `NoticeStream` API (separate item, already shipped per the parent tracker).
- The `for item in subscription` README pseudo-code fix is in-scope for this PR's docs sweep, but the broader README ergonomics audit is a different parent-tracker item.

## Follow-up `/simplify` candidates (not this PR)

- **`SubscriptionInner::PreDecoded` is test-only.** `Subscription::new(mpsc::UnboundedReceiver<Result<T, Error>>)` (`src/subscriptions/async.rs:200`) has zero production callers; the 6 call sites all live in `src/subscriptions/async_tests.rs` (lines 101, 117, 410, 466, 481, 570). The variant exists today and Commit 1's `poll_next` must still dispatch on it (tests rely on it), but a follow-up could either gate the constructor `#[cfg(test)] pub(crate)`, or rewrite those 6 tests onto the `WithDecoder` path with a trivial decoder and delete the variant entirely.
- **`SubscriptionItemStreamExt::filter_notices()` symmetric adapter.** Intentionally not added in this PR — `NoticeStream` already covers the cross-subscription notice case, and no caller has asked for a per-subscription notices-only filter. If a future caller wants it, add symmetrically to `filter_data` (same shape, `filter_notice` → keep `Notice`, drop `Data`).
