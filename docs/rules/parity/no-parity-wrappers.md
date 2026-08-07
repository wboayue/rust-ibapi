---
id: no-parity-wrappers
title: No wrapper struct when the async runtime already provides the abstraction
cluster: parity
status: active
triggers:
  - wrapping a tokio channel in a struct so both sides look alike
  - the async half of a mirrored pair would only delegate
  - deciding whether sync/async duplication is acceptable
symbols: [BroadcastSender, broadcast::Sender, filter_data, FilterData, FilterDataStream, deliver_to_request_id]
related: [dual-feature-types, subscription-consumer-idiom]
precedents: ["#512"]
memory: []
---

Don't wrap a `tokio::sync::broadcast::Sender` / `mpsc` / similar in a struct just so the async
side mirrors the sync side. Store the runtime type directly on the bus.

The sync side often needs a real wrapper — `Mutex<Vec<crossbeam Sender<Notice>>>` plus manual
pruning of dropped receivers. The async side gets `subscribe()` and auto-prune from tokio, so
the equivalent wrapper would be a no-op delegate. Asymmetry here is correct.

## Why

A pure parity-wrapper costs a type, a file, and a layer of indirection to buy visual symmetry
between two files nobody reads side by side. Worse, it invites callers to assume the two halves
have matching semantics when the async one is only forwarding.

The test is whether **both** sides have real behavior:

- `deliver_to_request_id` — genuine mirror. Both `transport/sync.rs` and `transport/async.rs`
  implement routing logic; the async one is `async fn`. Acceptable duplication.
- `filter_data` — genuine mirror. Both sides define the method, and the return types differ
  because the abstractions differ: sync yields `FilterData<I>` (an `Iterator` adapter), async
  yields `FilterDataStream<S>` (a `Stream` adapter).
- A struct that owns a `broadcast::Sender` and forwards `subscribe()` — not a mirror. Delete
  it and store `broadcast::Sender<RoutedItem>` on the bus.

## Precedents

- #512 — the global `NoticeStream` work, where the sync side kept
  `Mutex<Vec<Sender<Notice>>>` with manual pruning and the async side stored the broadcast
  sender directly rather than growing a matching wrapper.

> **Correction.** The pre-migration `CLAUDE.md` rule named the acceptable mirror as
> "`filter_data` / `filter_data_stream`". There is no `filter_data_stream` method. Both sides
> spell the method `filter_data`; what differs is the returned adapter type,
> `FilterData` vs `FilterDataStream`. (`filter_data_stream_drops_notices` is a *test* name —
> likely where the confusion came from.)
