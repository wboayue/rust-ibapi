# Broadcast lag: visible loss now, per-class semantics later (#779)

Decision record from the 2026-09-01 discussion of issue #779. Agreed direction:
failure must be visible on both transports; sync/async should be consistent in
*semantics per channel class*, not necessarily in mechanism; the issue's
unbounded-everywhere proposal is rejected (it converts market-data lag from
invisible drops into invisible memory growth, and an mpsc rewrite would unwind
PR #782's `receiver_count()`-based cleanup, which is broadcast-specific).

## Verified state (from the #779 triage)

The transports sit at opposite corners, both silent:

- **Async**: bounded + lossy + silent. `BROADCAST_CHANNEL_CAPACITY = 1024`;
  overflow evicts oldest; `Lagged(n)` is swallowed at three production sites —
  `AsyncInternalSubscription::next` (transport), `Subscription::poll_next`
  (subscriptions), and `TickSubscription::poll_next` in
  `market_data/historical` (the site the issue missed). Producer side cannot
  observe overflow (`broadcast::send` succeeds by evicting). Only the notice
  stream logs lag, at `debug!`.
- **Sync**: unbounded + lossless + silent. Crossbeam queues grow without
  limit; the failure mode is memory creep → OOM, equally invisible in-band.

## Step 1 — visibility only, no semantic change (SHIPPED — see the PR that carries this file)

- **Async**: at each of the three swallow sites, replace the silent
  `Err(_lagged) => continue` with a `warn!` carrying the skipped count **and**
  an in-band synthesized gap notice — `SubscriptionItem::Notice` with a new
  client-side code (next in the -2..-5 series, via `Notice::synthesized()`),
  message naming the dropped-frame count. Consumers already have Notice arms;
  the reconcile story is the same contract #777 documented for reconnect gaps
  (`filter_data()`/`iter_data()` drop notices — document, as with order
  rejections).
- **Async**: expose the broadcast capacity through `ClientBuilder` (default
  unchanged at 1024).
- **Sync**: watermark warnings — the dispatcher checks `len()` on its queues
  and `warn!`s when depth crosses thresholds ("consumer stalling, queue at N
  and growing"). Semantics untouched.

Non-terminal is deliberate: a terminal error on lag would let a transient blip
kill market-data subscriptions. A non-terminal `Err` item was ruled out — it
breaks the "Err is terminal" contract everywhere.

## Step 2 — per-class unification (deferred; needs a decision)

Converge both transports on the same behavior per channel class:

- **Order-class** (order channels, `order_update_stream`, executions):
  completeness is the contract and rates are low — lossless everywhere. Async
  either gets an effectively-unbounded order path or keeps broadcast with a
  capacity high enough that a gap notice there is a five-alarm signal, not an
  operating mode.
- **Market-data-class** (ticks, bars, depth): freshness beats completeness —
  bounded + lossy + gap-notice everywhere. Dropping *oldest* under lag is the
  right policy; the defect was only the silence.

**Open question blocking step 2**: is sync becoming lossy on market-data
channels acceptable? It is a real behavior change (sync users today never
drop, they accumulate). Fallback if not: leave sync unbounded but loudly
watermarked, make async's gap notice the documented cross-transport contract
for "you fell behind", and accept mechanically different but observably honest
transports.

Related: [[transport-cleanup-followups]] item 3 (WeakSender / liveness-token
unification) touches the same channel machinery; if both land, sequence them
so the cleanup mechanism is settled before channels change shape.
