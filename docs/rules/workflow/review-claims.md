---
id: review-claims
title: A review claim is a finding only after it's traced to production or checked live
cluster: workflow
status: active
triggers:
  - reviewing a PR, your own or a contributor's
  - about to report a bug found by reading code
  - a PR's motivation is "match another client's request" or "make X reachable"
  - citing a precedent or a "good example" in an issue, review or rule node
symbols: [InternalSubscription, SubscriptionBuilder, fail_all, MessageBusStub]
related: [exercise-production-code, plans-and-issues, timezone-free-timestamps]
precedents: ["#1141", "#1147", "#1148", "#1152", "#1154"]
---

Before a review claim goes into a review, issue or rule, check it against what production
does or what the gateway does. Reading the code and reasoning about it isn't enough.

- **A bug found by reading code:** trace the state that triggers it to a production
  constructor. Test fixtures can build states production never builds, like a sync
  subscription whose channel closes with no terminal error. If only a fixture reaches it, it's
  not a finding.
- **A feature's motivation:** check what a caller *observes* with and without the change, live
  if fixtures and the C# client don't settle it (the `/live-probe` skill). "Reproduces another
  client's request shape" isn't a caller-visible benefit.
- **A cited precedent or good example:** read the cited code and confirm it does what you
  claim, before publishing. A wrong example spreads into the next change that follows it.

## Why

A confident wrong claim costs more than a missed one: the author spends a round on it, or the
change ships built on it. Each case below read correctly from the code and was wrong in
production, or in what TWS sends.

## Precedents

- #1141: the review claimed a closed channel would leave `is_terminated()` false forever.
  Production subscriptions hold their own `Sender` (every `SubscriptionBuilder` in
  `transport/sync.rs` sets `.sender(..)`), so the channel can't close while the handle lives.
  Shutdown and reset arrive as terminal errors through `fail_all`. Only fixtures reach it, so
  the finding was retracted.
- #1147: the PR claimed `useRTH` changes the trading schedule. It was confirmed live before
  merging (`docs/wire-facts/market-data.md#historical-schedule-honors-userth`).
- #1148: declined. Its motivation was matching another client's `formatDate=1` request; live,
  `formatDate=2` already gave callers exact dates and instants.
- #1152: the first draft cited the news time parser as a good zone-free example. Read closely,
  it *assumed* UTC; a probe then confirmed it (#1153).
- #1154: the first draft of the issue cited a "wire optionality" precedent that doesn't exist.
  The recorded lesson is the opposite: wire optionality isn't public-API optionality.
