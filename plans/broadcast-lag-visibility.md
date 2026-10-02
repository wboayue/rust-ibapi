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

## Step 1 — visibility only (shipped)

Async lag surfaces as non-terminal notices (`SUBSCRIPTION_LAG_CODE` `-6`,
`NOTICE_STREAM_LAG_CODE` `-7`); sync warns at queue-depth watermarks.
Non-terminal is deliberate: a terminal error on lag would let a transient blip
kill market-data subscriptions, and a non-terminal `Err` item breaks the "Err
is terminal" contract.

Step-1 leftovers, deliberately excluded (fold into step 2 or do piecemeal):

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

- **Enumeration-class** (request-scoped streams that end, e.g. contract
  details; opt-in via `buffer_limit`, #876): completeness *and* bounded
  memory, so bounded + lossless + fail-closed. Shipped on both transports:
  past the cap the stream ends with `Error::BufferLimitExceeded`, and nothing
  is evicted (async keeps a spare slot for the error). Unset, a request keeps
  its transport's default class.

**Open question blocking step 2**: is sync becoming lossy on market-data
channels acceptable? It is a real behavior change (sync users today never
drop, they accumulate). Fallback if not: leave sync unbounded but loudly
watermarked, make async's gap notice the documented cross-transport contract
for "you fell behind", and accept mechanically different but observably honest
transports.

Related: [[transport-cleanup-followups]] item 3 (WeakSender / liveness-token
unification) touches the same channel machinery; if both land, sequence them
so the cleanup mechanism is settled before channels change shape.
