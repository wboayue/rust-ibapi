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

As shipped:

- **Async**: lag→notice conversion lives in one place —
  `AsyncInternalSubscription::poll_next_routed` — so every consumer that polls
  through the wrapper gets the in-band `SUBSCRIPTION_LAG_CODE` (`-6`) notice
  (built + `warn!`ed by `messages::subscription_lag_notice`); no consumer can
  reintroduce a silent swallow. The legacy `ResponseMessage` projection drops
  the notice by construction (`into_legacy`), so that path is warn-only.
- **Async**: `ClientBuilder::channel_capacity` — a single global per-client
  knob (default 1024, `0` rejected). The notice fan-out channels keep the
  default; step 2's per-class capacities would need a new setter shape.
- **Sync**: watermark warnings on the subscription, shared-channel, and
  order-update send paths (every 10k of queue depth). Semantics untouched.

Non-terminal is deliberate: a terminal error on lag would let a transient blip
kill market-data subscriptions. A non-terminal `Err` item was ruled out — it
breaks the "Err is terminal" contract everywhere.

Step-1 leftovers, deliberately excluded (fold into step 2 or do piecemeal):

- `NoticeStream` (async) lag: upgraded `debug!`→`warn!` only; an in-band
  `subscription_lag_notice` there is a one-liner (`next` returns
  `Option<Notice>`) but changes the stream's contents — decide with step 2.
- `NoticeBroadcaster` (sync notice fan-out) has no watermark.
- Three sibling `test_notice`/`make_notice` helpers exist across test files;
  a shared `#[cfg(test)]` constructor next to `Notice::synthesized` would
  retire them (rule-of-three already tripped).

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
