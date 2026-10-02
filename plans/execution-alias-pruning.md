# Execution-alias pruning (#880)

## Problem

Exec-id → sender alias maps (async `execution_channels`, sync `executions`) only clear on
reconnect/shutdown. One entry per fill for the life of the session; each holds a sender
clone, so a dropped subscription's channel (and buffered items) stays allocated.

## Approach: prune on subscription cleanup

Tie alias lifetime to the owning channel. Bounds the map to executions of *live*
subscriptions (a single order's fills / one `executions()` snapshot) — no wire assumptions.

Rejected for now: drop alias once its `CommissionsReport` is routed. Needs wire evidence
that IB sends at most one report per exec id outside `reqExecutions` replays (which
re-insert via fresh `ExecutionData` anyway). Not needed to bound growth. Follow-up only if
a live capture confirms (see memory: verify wire before constraining).

### Sync (`src/transport/sync.rs`)

- Add `SenderHash::remove_all_same(&self, sender: &Sender<V>) -> usize` —
  `retain(|_, e| !e.sender.same_channel(sender))`, returns removed count.
- `clean_request` / `clean_order`: after `remove_if_same`, call
  `self.executions.remove_all_same(sender)`; include count in the existing `debug!`.
- Prune unconditionally (not gated on `removed`): a stale signal whose key was re-registered
  still owns its own aliases, and `same_channel` keeps the newer registration's aliases.

### Async (`src/transport/async.rs`)

- Cleanup task: clone `execution_channels` alongside `request_channels`/`order_channels`.
- `CleanupSignal::Request` / `Order`: after `remove_if_dead`, run
  `prune_dead_aliases(&execution_channels)` —
  `retain(|_, s| s.receiver_count() > 0)`. Same liveness rule as `remove_if_dead`
  (authoritative because `detach_receivers` runs before the signal). Unconditional, same
  reasoning as sync; also sweeps aliases of other dead subs whose signals are still queued.
- O(n) per cleanup signal; n is now bounded by live subscriptions' fills.

### Known residual race (document, don't fix)

Dispatcher copies the order/request sender, cleanup thread/task removes channel + prunes,
dispatcher then inserts the alias → one dead alias until the next cleanup (async: swept by
the next signal's dead-sender pass) or reset (sync). Bounded to the in-flight frame; note it
in a comment at the prune site.

## Tests

Existing `execution_data_body` helpers in both test files.

Sync (`sync_tests.rs`):
1. `test_execution_alias_pruned_when_order_subscription_dropped` — place order, route
   `ExecutionData` (order id), assert `executions.len() == 1`, drop sub, wait for cleanup,
   assert empty.
2. Same for request route (`execution_data_body(99, 0, ..)`).
3. `test_execution_alias_survives_stale_drop_signal` — stale signal for an old sender on a
   re-registered id leaves the new registration's alias intact.
(`remove_all_same` mixed-sender behavior covered by 3; no separate unit test.)

Async (`async_tests.rs`):
1. Order + request variants: map populated, drop, poll until `execution_channels` empty
   (bounded timeout).
2. Live replacement under same id keeps its alias after the old sub's signal.
(Commission after prune = unmapped path, already covered by
`test_commission_report_without_mapping_dropped`.)

## Docs

- Drop "never pruned" implication from any comments; update the `execution_channels` field
  doc: "pruned when the owning subscription is cleaned up".
- No public API change → no migration-5.0 entry. Changelog line under fixes.

## Checklist

- `cargo fmt`, clippy per CI flags (sync + async features), `just test`.
- PR body: `Closes #880`; note the deferred commission-removal option.
