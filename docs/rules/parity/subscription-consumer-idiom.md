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
  - collecting a Subscription into a Vec inside a Client method
symbols: [Subscription, filter_data, SubscriptionItem, FilterDataStream, DisplayGroupSubscription]
related: [dual-feature-types, no-parity-wrappers, exercise-production-code]
precedents: ["#550", "#598", "#735", "#1081"]
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

**Never write `(&mut sub).filter_data().next().await`.**

## Why

`filter_data` takes `self`; the cast exists only to dodge the move, and it spreads by copy-paste
into every new example.

Switching a call site to consume form leaves the original binding's `mut` unused, which fires
`unused_mut`, an error under CI's `-D warnings`. Drop it.

Through a `Deref` wrapper such as `DisplayGroupSubscription`, `sub.next().await` works unchanged;
the `&mut *sub` reborrow is only needed to reach `filter_data` through `Deref`.

Tests that assert the notice-filter contract (`filter_data_stream_drops_notices`,
`routed_item_notice_skipped_then_response_delivered`) call `filter_data` in consume form too.

Sync-side trap: the default `IntoIterator` yields `SubscriptionItem`, not `T`, and
`iter_data().flatten()` silently discards `Err` items because `Result` is itself `IntoIterator`.
Match explicitly in anything a user will copy.

## Inside a Client method, ask whether it consumes `Err`

Worse in library code, because the caller cannot see it. Collecting with
`while let Some(Ok(item))` or `if let Ok(item)` turns a routed `RoutedItem::Error` into a normal
loop exit and returns an empty `Vec`, so a TWS rejection looks like "nothing matched". Blocking
`matching_symbols` shipped that while its async twin returned the `Err`; `OrderBuilder::analyze`
had it on both sides, on the one API where rejection is routine.

**No shared layer answers this.** The dispatcher produces the `Err`; whether it reaches the caller
is decided per public entry point, so it is a per-method review question and a per-method test.
See [exercise production code](../testing/exercise-production-code.md).

## Precedents

- #550 — `impl Stream for async Subscription<T>`; its `/simplify` pass removed the re-borrow
  casts and named the reason.
- #598 — canonicalized sync stream iteration on `iter_data()` plus an explicit `Result` match
  across examples.
- #735 — swept `Some(Ok(..))` consumption sites after finding two that dropped routed errors.
- #1081 — finished #598's sweep in sync doc examples, where `for x in &sub` had survived: thirteen
  sites moved to `iter_data()` with an explicit `Err` arm.
