# Transport registration-cleanup follow-ups

Deferred from the #773/#778 fix (identity/liveness-gated drop-signal cleanup)
and its /simplify pass. Context: async cleanup removes a registration only when
its channel has `receiver_count() == 0`; sync signals carry the dropped
subscription's sender and remove only the `same_channel` match.
§2 (`execution_channels` never pruned, #880) shipped in #889; numbering kept.

## 1. Cancel paths still remove by key unconditionally

`cancel_order_subscription` (both transports) does `send(Error::Cancelled)` +
`remove(&id)` on whatever is registered under the id — the same shape the
drop-signal fix closed, and worse: it poisons a successor's channel before
deleting it. Latent today because no order decoder overrides `cancel_message`
(default `Err(Error::NotImplemented)` skips the branch). The day one gains a
`cancel_message`, #773 reopens through this untested path. Fix needs the
`MessageBus` trait to receive the caller's identity (or the async liveness
check); that signature change is why it was deferred.

## 3. Unify the two mechanisms once tokio MSRV allows

The sync/async split (identity vs liveness) is forced by today's constraints:
the async subscription must not hold a `Sender`, and crossbeam has no
`receiver_count`. Two paths out:

- `tokio::sync::broadcast::WeakSender` (tokio 1.44+; Cargo.toml declares 1.41)
  would give async `same_channel` identity with no strong sender and no
  `detach_receivers` dance.
- A runtime-agnostic liveness token (`Arc<()>` in the subscription, `Weak<()>`
  beside each registration) would give *sync* the async side's
  replace-if-dead recreation too, retiring the documented sync
  `AlreadySubscribed`-retry caveat on `order_update_stream`.

Either also covers follow-up 1 (cancel paths) more cleanly than per-site
patches.

## Skipped /simplify items (recorded, not planned)

- `SenderHash::remove_if_same` get-then-remove double lookup: `K` is not
  `Clone`-bounded, entry API needs `K` by value; matches the file's existing
  idiom. Cold path.
- `detach_receivers` allocates a throwaway broadcast channel per drop (~3
  allocs, cold path). `Option<..> + take()` would ripple `as_mut()` into the
  hot `TickSubscription::poll_next`; not worth it unless follow-up 3 removes
  the dance entirely.
- Test fixtures that build a signaler without a sender (`stubs.rs`,
  `subscriptions/sync_tests.rs`) now silently skip the drop signal via the
  `Drop` early-return; giving them senders — or moving signal construction
  into `SubscriptionBuilder::build()` so the sender-less state is
  unrepresentable — goes with follow-up 3's restructuring.

## 4. Async shutdown/reset shape

Two related /simplify flags from PR #783:

- Async `request_shutdown` notifies nothing (relies on sender-drop →
  end-of-stream) while sync sends `Error::Shutdown` before clearing — now the
  one remaining sync/async divergence in teardown signalling, noted in a
  comment at the site. Aligning means deciding what the public `Subscription`
  should surface at shutdown.
- The async bus keeps five bare `Arc<RwLock<HashMap<..>>>` fields where sync
  has registry types (`SenderHash`, `SharedChannels` with `notify_all`), so
  every teardown behavior is open-coded per map; shared-sender fan-out exists
  at three sites (`route_to_shared_channel`, `fail_one_shot_channels`,
  `reset_channels`). Async registry types mirroring sync's would collapse
  `reset_channels` and `request_shutdown` both.
