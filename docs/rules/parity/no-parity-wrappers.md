---
id: no-parity-wrappers
title: No wrapper struct when the async runtime already provides the abstraction
cluster: parity
status: active
triggers:
  - wrapping a tokio channel in a struct so both sides look alike
  - the async half of a mirrored pair would only delegate
  - deciding whether sync/async duplication is acceptable
symbols: [BroadcastSender, broadcast::Sender, filter_data, FilterData, FilterDataStream, deliver_to_order_or_request, NoticeBroadcaster]
related: [dual-feature-types, subscription-consumer-idiom]
precedents: ["#512", "#872"]
memory: []
---

Don't wrap a `tokio::sync::broadcast::Sender` / `mpsc` / similar in a struct just so the async
side mirrors the sync side. Wrap a channel only when the wrapper adds behavior the bare type
lacks (`close`, end-on-arrival). A struct that only forwards is the anti-pattern: delete it and
store the `broadcast::Sender<RoutedItem>` directly.

## Why

A pure parity wrapper costs a type and a layer of indirection to buy visual symmetry, and invites
callers to assume matching semantics when one half only forwards.

The test is whether **both** sides have real behavior:

- `deliver_to_order_or_request` — both `transport/sync.rs` and `transport/async.rs` implement the
  routing. Acceptable duplication.
- `NoticeBroadcaster` — the sync half needs a wrapper anyway
  (`Mutex<Option<Vec<crossbeam Sender<Notice>>>>` plus pruning of dropped receivers). tokio's
  `broadcast` gives async `subscribe()` and auto-prune for free, so the async half earns its
  wrapper only through `close`, which ends every `NoticeStream`, including one subscribed after
  the close (#872).
- `filter_data` — both sides define the method; return types differ because the abstractions do:
  `FilterData<I>` (`Iterator` adapter) vs `FilterDataStream<S>` (`Stream` adapter). There is no
  `filter_data_stream` method; that name is only a test (`filter_data_stream_drops_notices`).

## Precedents

- #512 — global `NoticeStream`: the async side stored the broadcast sender directly rather than
  growing a matching wrapper.
- #872 — the async side gained `NoticeBroadcaster` because shutdown needed `close`. Not a
  counter-example: the wrapper came with behavior, not for symmetry.
