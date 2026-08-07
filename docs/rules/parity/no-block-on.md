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

Never call `futures::executor::block_on()` from an async context. It parks a tokio worker
thread, and if the future it waits on needs that same runtime, the task deadlocks.

Two remedies, in order of preference:

1. **Make the function `async`** and `.await` the lock.
2. **Use an atomic** for lock-free reads of a rarely-written value — `AtomicI32` for
   `server_version_cache`, and similar.

## Why

The failure is a hang, not an error. Nothing logs, no `Result` goes red, and the call site
looks correct — which is why this is a rule rather than a code review preference. It also
tends to appear in exactly the places where the sync and async clients share a field, because
`block_on` is the shortest path to reusing sync-shaped accessor code from an async path.

An atomic is the better fix when the value is written once at connect and read on every
request: no lock, no `.await`, and the same field serves both clients. `AtomicI32` with
`Ordering` on `server_version_cache` in `connection/async.rs` is the worked example.

Reach for the atomic only when the value is genuinely small and rarely written. For anything
that needs coordinated multi-field updates, make the function `async` and take the real lock.

## Precedents

- #420 — removed `futures::executor::block_on` from the async client and moved
  `server_version_cache` to `AtomicI32`, closing the deadlock scenario.
- #422 — made the `AtomicI32` the single source of truth for `server_version`, so no
  lock-guarded copy could drift from it.
