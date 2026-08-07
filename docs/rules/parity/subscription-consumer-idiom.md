---
id: subscription-consumer-idiom
title: Consume an async Subscription without the re-borrow cast
cluster: parity
status: active
triggers:
  - consuming an async Subscription in an example, test, or doc
  - reaching for (&mut sub).filter_data()
  - iterating a Deref wrapper around Subscription
  - an unused_mut warning appears after switching stream adapters
symbols: [Subscription, filter_data, SubscriptionItem, FilterDataStream, DisplayGroupSubscription]
related: [dual-feature-types, no-parity-wrappers]
precedents: ["#550", "#598"]
memory: [feedback_stream_adapter_consume_form, feedback_subscription_for_yields_subscription_item, feedback_result_flatten_drops_errors]
---

Two forms, chosen by whether the subscription outlives the loop:

```rust
// (a) consume — `sub` is not used after the loop
let mut data = sub.filter_data();
while let Some(result) = data.next().await { /* Result<T, Error> */ }

// (b) pattern-match — `sub` is reused (mid-loop cancel, post-loop next(), …)
while let Some(item) = sub.next().await {
    match item {
        Ok(SubscriptionItem::Data(t)) => { /* … */ }
        Ok(SubscriptionItem::Notice(_)) => continue,
        Err(e) => { /* … */ }
    }
}
```

**Never write `(&mut sub).filter_data().next().await`.** The re-borrow cast reintroduces the
API being removed under a different name.

## Why

`filter_data` takes `self`, so the cast exists only to dodge the move — it is a workaround
wearing the shape of an idiom, and it spreads by copy-paste into every new example.

Switching a call site from cast to consume form makes the original binding immutable, so drop
its `mut`. Consuming an immutable binding is legal, but the leftover `mut` fires `unused_mut`,
which CI treats as an error under `-D warnings`.

Through a `Deref` wrapper such as `DisplayGroupSubscription`, `sub.next().await` works
unchanged. The `&mut *sub` reborrow is only needed for `filter_data` adapters reached through
`Deref`.

Two tests legitimately call `filter_data` because the notice-filter contract *is* what they
assert — `filter_data_stream_drops_notices` and
`test_routed_item_notice_skipped_then_response_delivered`. Both use consume form, not the cast.

Related trap: the default `IntoIterator` on the sync side yields `SubscriptionItem`, not `T`,
and `iter_data().flatten()` silently discards `Err` items because `Result` is itself
`IntoIterator`. Match explicitly in anything a user will copy.

## Precedents

- #550 — `impl Stream for async Subscription<T>`; its `/simplify` pass (commit `3b708a7`)
  removed the re-borrow casts and named the reason.
- #598 — canonicalized sync stream iteration on `iter_data()` plus an explicit `Result` match
  across the examples.
