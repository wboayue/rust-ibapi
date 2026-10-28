---
id: no-block-on
title: Never block_on inside async code
cluster: parity
status: active
triggers:
  - reaching for futures::executor::block_on
  - needing a lock's value inside an async fn
  - a sync and an async path share one rarely-written field
  - an async call hangs with no error
symbols: [block_on, AtomicI32, Ordering, server_version_cache]
related: [dual-feature-types, no-parity-wrappers]
precedents: ["#420", "#422"]
memory: []
---

Never call `futures::executor::block_on()` from an async context. It parks a tokio worker thread,
and if the awaited future needs that runtime, the task deadlocks.

Two remedies, in order of preference:

1. **Make the function `async`** and `.await` the lock.
2. **Use an atomic** for lock-free reads of a small, rarely-written value. Worked example:
   `server_version_cache: AtomicI32` in `connection/async.rs`, written once at connect and read on
   every request with no lock or `.await`.

Use the atomic only for a single small value. Anything needing coordinated multi-field updates
takes the real lock in an `async fn`.

## Why

The failure is a hang: nothing logs, no `Result` goes red, the call site looks correct. It tends
to appear where the sync and async clients share a field, because `block_on` is the shortest path
to reusing sync-shaped accessor code from async.

## Precedents

- #420 — removed `block_on` from the async client; `server_version_cache` became `AtomicI32`.
- #422 — made the `AtomicI32` the single source of truth for `server_version`, so no lock-guarded
  copy could drift.
