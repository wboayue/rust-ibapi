# Async `Subscription` drop outside a runtime — #848

**Implemented** on `async-drop-runtime-handle`. Deviations: the two CHANGELOG
lines merged into one; `MessageBusStub` implements `Drop`, so its constructors
can't use `..Self::default()` — each sets `runtime` explicitly. Shutdown case
confirmed: `Handle::spawn` on a dropped runtime does not panic.

An async `Subscription` dropped on a thread with no Tokio runtime (e.g. moved
into `std::thread::spawn`) skips its cancel. For a shared stream it also skips
the `SharedCounts` release, so since #836 the type's count stays +1 until the
next reconnect and no later subscription of the type writes its cancel.

Fix: the bus captures the runtime handle at construction; `Drop` spawns on it.
`Handle::spawn` works from any thread.

## 1. Bus: store the handle (`src/transport/async.rs`)

- `AsyncTcpMessageBus` gains `runtime: tokio::runtime::Handle`, set to
  `Handle::current()` in `with_channel_capacity`. No new precondition:
  that fn already calls `task::spawn` for the cleanup task, which panics
  outside a runtime the same way.
- `AsyncMessageBus` gains `fn runtime_handle(&self) -> &Handle;`, doc: "the
  runtime the bus was built on; `Drop` impls spawn their async cleanup here".

## 2. Stub (`src/stubs.rs`)

- `MessageBusStub` gains `runtime: Option<Handle>`, `Handle::try_current().ok()`
  in `Default` (sync-only tests build the stub with no runtime).
- `runtime_handle()` → `.expect("MessageBusStub built outside a Tokio runtime")`.
  Only reached when an async `Subscription` drops, so sync tests never hit it.
  Prod trait stays `&Handle`, not `Option` — the `None` exists only in the stub.

## 3. `Subscription::drop` (`src/subscriptions/async.rs`)

```rust
let message_bus = self.message_bus.clone();
self.message_bus.runtime_handle().spawn(async move { ... });
```

Deviation from the issue: always spawn on the bus handle, no
`try_current()` first. One path instead of two; and the bus's runtime is the
one that drives its I/O, so it is the right one even when the drop happens on
some other runtime.

- Delete the `let Ok(runtime) = ... else { warn!(..); return; }` block and the
  "count stays one too high" comment.
- Type docs (lines ~54-59): replace the "Drop the subscription inside a Tokio
  runtime ..." paragraph with: the cancel is sent from a task spawned on the
  client's runtime, so a subscription may be dropped on any thread. If that
  runtime has shut down, nothing is sent (the connection is gone with it).

## 3b. `TickSubscription::drop` (`src/market_data/historical/async.rs`)

Same bug class, worse: it calls bare `tokio::spawn`, which **panics** on a
thread with no runtime (no guard at all). Same one-line fix:
`self.message_bus.runtime_handle().spawn(..)`. Only other async `Drop` that
spawns (`grep -rn "spawn(" src/ | grep -v _tests`).

## 4. Runtime already shut down

`Handle::spawn` on a shut-down runtime drops the future without panicking
(returns a `JoinHandle` that resolves to a cancelled `JoinError`). Bus is dead,
count is moot. Verified by test 5c. If it does panic on the pinned tokio,
stop and revisit — no `catch_unwind` in `Drop`.

## 5. Tests

a. `src/transport/async_tests.rs`, real bus via `make_bus()` +
   `SubscriptionBuilder::send_shared` (mirrors
   `test_shared_subscription_cancel_waits_for_last_subscriber`):
   `test_shared_subscription_dropped_off_runtime_cancels_and_releases_count` —
   one `RequestPositions` subscription, `std::thread::spawn(move || drop(sub)).join()`,
   `wait_for_frames(&stream, &cancel, 1) == 1`. Then subscribe + drop again
   on-runtime and assert a second cancel is written — the user-visible symptom
   of the leak, and it implies the count returned to 0 (no `live()` peek;
   `SharedCounts` unit tests own the counting).

b. Id-routed needs no real bus (no `SharedCounts`), so lightest fixture:
   `src/subscriptions/async_tests.rs`, `#[tokio::test]`
   `test_request_subscription_dropped_off_runtime_cancels` —
   `subscription_with::<CancellableItem>(Some(1), None, ..)`, drop on a plain
   thread, assert `fixture.bus.request_messages` holds `cancel_frame()`.

b2. `src/market_data/historical/async_tests.rs`: mirror
   `test_tick_subscription_sends_cancel_on_drop`, dropping on
   `std::thread::spawn`; today this panics.

c. `src/subscriptions/async_tests.rs`: replace
   `test_drop_outside_runtime_does_not_panic` (a plain `#[test]` whose stub
   now has no handle) with `test_drop_after_runtime_shutdown_does_not_panic`:
   build a current-thread runtime, create the fixture in `rt.block_on`,
   `drop(rt)`, drop the subscription on `std::thread::spawn`, join ok.

`#[tokio::test]` is current-thread: the dropped-off-thread task runs when the
test next awaits (`wait_for_frames`), so no multi-thread flavor needed.

## 6. Docs

- `CHANGELOG.md` `[Unreleased] ### Fixed`: the #836 line "Dropping an async
  subscription outside a Tokio runtime no longer panics: the cancel is skipped
  ..." is unreleased — rewrite in place: "An async subscription can be dropped
  on any thread, including one with no Tokio runtime: the cancel is sent on the
  client's runtime (#836, #848)."
- Also `### Fixed`: "Dropping an async historical-ticks subscription
  (`TickSubscription`) on a thread with no Tokio runtime no longer panics; its
  cancel is sent (#848)." 
- `docs/migration-4.0.md:559`: last sentence ("logs a warning instead of
  panicking") → "can be dropped on any thread; its cancel still goes out."

## 7. Checks

Full pre-PR gate (CLAUDE.md), plus `cargo build -p ibapi-integration-async --tests`
(touches `Subscription`). `just rules-check` for this plan file.

Branch `async-drop-runtime-handle`, one PR, closes #848.

## Lens review (plan-time)

- **Duplication** — second async `Drop` that spawns (`TickSubscription`) was
  missed; added as 3b. Both now spawn on the same bus handle. No shared helper:
  each is one line, and `send_cancel` vs `cancel_subscription` differ.
- **SRP** — `runtime_handle() -> &Handle` on the bus, not a
  `spawn_cleanup(BoxFuture)`: both callers only `.spawn`, and a boxed-future
  trait method adds allocation + indirection for no second use. The bus
  already owns the runtime's tasks (cleanup, process loop), so the handle
  belongs there, not on each `Subscription`.
- **Composability / tests** — id-routed test moved to the stub fixture
  (lightest that works); real bus kept only where `SharedCounts` is under test.
  Dropped the `live()` peek in (a): the re-subscribe cancel is the observable
  contract and implies it.
- **Not changed** — stub `Option<Handle>` + `expect`: the `None` is a stub
  artifact (sync-only tests), kept out of the prod trait.
