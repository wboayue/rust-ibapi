# Sync shared queues: stale resets and frames — follow-up to #817 / #818

Expands [transport cleanup follow-ups §4](transport-cleanup-followups.md#4-sync-stale-connectionreset-buffered-in-idle-streaming-shared-queues).
PR #818 documented the symptom as a caveat on `TRANSPORT_RECONNECT_CODE`
(`src/messages.rs`) and in the `## [Unreleased]` changelog; this plan removes
it.

## 1. Problem

On the blocking client, a streaming shared-channel request (`open_orders`,
`positions`, `account_updates`, ...) can read items that predate it:

- **Stale resets.** After a socket drop, a resubscribe — the documented
  recovery on `TRANSPORT_RECONNECT_CODE` — can read `Error::ConnectionReset` as
  its first item and end immediately, before its own responses.
- **Stale frames.** An idle open-orders queue buffers `OpenOrder` /
  `OrderStatus` frames delivered for other requests, so the next
  `open_orders()` can start with frames it did not ask for.

The async client is not affected: every async subscription takes a fresh
broadcast receiver (`resubscribe`), so nothing sent before it subscribed
reaches it.

## 2. Root cause

Sync shared channels are one persistent unbounded crossbeam queue per
*request* type (`SharedChannels` in `src/transport/sync.rs`), and nothing
clears a queue between subscriptions except the one-shot drain in
`send_shared_request`. Two fan-outs fill them:

**a. Resets are sent once per response type, not once per queue.**
`SharedChannels::register` files the request's single sender under every
response type it maps to, and `notify_all` iterates the senders map by
response type. Each streaming queue therefore receives one reset per response
type. From `CHANNEL_MAPPINGS`:

```bash
awk '/CHANNEL_MAPPINGS: &\[ChannelMapping\] = &\[/,/^\];/' src/messages/shared_channel_configuration.rs \
  | tr -d '\n' \
  | grep -o 'request: OutgoingMessages::[A-Za-z]*, *responses: &\[[^]]*\], *one_shot: false' \
  | sed -E 's/request: OutgoingMessages::([A-Za-z]+).*responses: &\[([^]]*)\].*/\1 \2/' \
  | awk '{n=gsub(/IncomingMessages::/,""); print $1, n}'
```

| Queue | Resets per `reset()` |
|---|---|
| `RequestAccountData` | 4 |
| `RequestOpenOrders`, `RequestAllOpenOrders`, `RequestAutoOpenOrders` | 3 each |
| `RequestPositions`, `RequestPositionsMulti`, `RequestCompletedOrders` | 2 each |
| `RequestNewsBulletins` | 1 |

A sync subscription ends at its first error (`src/subscriptions/sync.rs`,
`RoutedItem::Error` arm), so a subscription that *was* live at the drop still
leaves the remaining copies queued. An idle queue keeps all of them.

**b. Frames fan out to every queue sharing a response type.** The three
open-orders requests map to the same three response types, so every
`OpenOrder` / `OrderStatus` / `OpenOrderEnd` reaches all three queues, and
order activity with no order registration is routed there as well
(`dispatch_message`, the `contains_sender(IncomingMessages::OpenOrder)` arm).
Only the queue whose request is in flight has a reader.

## 3. Fix

Two small changes in `src/transport/sync.rs`, one per cause.

### 3a. Notify once per queue

Keep one sender per registration alongside the by-response-type map, and have
`notify_all` iterate that:

```rust
struct SharedChannels {
    senders: HashMap<IncomingMessages, Vec<Arc<Sender<RoutedItem>>>>,
    receivers: HashMap<OutgoingMessages, Arc<Receiver<RoutedItem>>>,
    /// One sender per queue, for fan-outs that must reach each queue once.
    queues: Vec<Arc<Sender<RoutedItem>>>,
}
```

`register` pushes the `Arc` it already builds; `notify_all` loops over
`queues`. Covers `reset()` and `request_shutdown()` alike.

### 3b. Drain a streaming queue nobody is reading

In `send_shared_request`, drain when the request is one-shot **or** no live
subscription holds the queue:

```rust
let shared_receiver = self.shared_channels.get_receiver(message_type);

// Two strong refs: the `SharedChannels` map and this local. Any more is a
// live subscription of the same type, whose buffered items must survive.
let idle = Arc::strong_count(&shared_receiver) == 2;
if shared_channel_configuration::is_one_shot_request(message_type) || idle {
    while shared_receiver.try_recv().is_ok() {}
}
```

Sound because the receiver `Arc` is held only by the map and by each
`InternalSubscription` (`shared_receiver` field in `src/transport/mod.rs`);
verify before implementing with
`grep -rn "shared_receiver" src/transport src/subscriptions --include=*.rs | grep -v _tests.rs`.
A finished-but-undropped subscription still holds its `Arc`, so the drain is
skipped until it is dropped — conservative, never lossy.

The risk recorded in §4 of the parent plan (a drain discarding items buffered
for a concurrent subscription) applies only when another subscription holds
the queue, which is exactly when this skips. Two threads subscribing to the
same type at the same instant can still race; that pattern is already
unsupported ("Avoid concurrent requests of the same type",
`docs/architecture.md`) and already splits items between readers today.

3b alone would hide 3a, but only for the next subscriber; 3a keeps the fan-out
itself correct.

## 4. Tests (`src/transport/sync_tests.rs`)

Alongside `test_one_shot_shared_request_drains_stale_buffered_error` and
`test_reset_notifies_all_channel_categories`:

- **Reset reaches each queue once**: subscribe `RequestOpenOrders`, `reset()`,
  assert exactly one `ConnectionReset` then `try_next_routed()` is `None`.
- **Idle streaming queue is drained**: `reset()` with no subscriber, then
  `send_shared_request(RequestOpenOrders)`; the first item must be the
  response pushed afterwards, not a reset.
- **Stale frames drained**: dispatch an `OpenOrder` frame with no subscriber,
  subscribe, assert the buffered frame is gone.
- **Live subscriber protected**: keep the existing
  `test_streaming_shared_request_does_not_drain_buffered_items` green, adjusted
  so a live subscription holds the queue while the second request is made
  (today it relies on no drain at all for streaming types).
- **Integration**: extend the sync reconnect test in
  `src/connection/sync_tests.rs` to resubscribe `open_orders` after the
  reconnect and assert its first item is not `ConnectionReset`.

## 5. Docs to remove or update in the same PR

- `src/messages.rs` — drop the blocking-client caveat from the
  `TRANSPORT_RECONNECT_CODE` doc.
- `CHANGELOG.md` — drop the caveat sentence from the #816 bullet if still
  unreleased; otherwise add a `### Fixed` entry.
- `plans/transport-cleanup-followups.md` §4 — mark resolved, pointing at the PR.
- `docs/architecture.md` — the "Shared Channels" design note can mention that
  an idle sync queue is drained on subscribe.

## 6. Out of scope

- **Per-subscription sync shared channels** (the async model). Removes the
  class entirely but restructures `SharedChannels` and routing; revisit with
  parent plan §3.
- **Async parity for 3a.** `reset_channels` also loops the by-response-type
  senders map (`shared_channel_senders.values().flatten()`), so a live async
  subscriber receives duplicate resets. Harmless — its receiver drops the
  extras — so only worth doing if 3a's shape is shared across transports.
