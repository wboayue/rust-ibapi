# Warning Message Routing Enhancement

## Status
**Priority:** Medium
**Type:** Enhancement / Bug
**Recommendation:** Option 1 — route warnings with a real `request_id` to their owning subscription
**Bundled with:** [#487 — Align sync `Subscription<T>::next()` with async](https://github.com/wboayue/rust-ibapi/issues/487). Both are breaking changes to `Subscription<T>` on `main`; ship together so consumers migrate once.

## Problem
Warning codes (2100..=2169) carrying a valid `request_id` are diverted to the global error log instead of the subscription that owns them. Callers can't react programmatically. The dispatcher conflates two unrelated cases:

```rust
// src/transport/sync/mod.rs:275, src/transport/async.rs:424
if request_id == UNSPECIFIED_REQUEST_ID || is_warning_error(error_code) {
    error_event(...)  // log only
}
```

`UNSPECIFIED_REQUEST_ID` means "no owner, must log." A warning with a real `request_id` has an owner — split the cases.

## Recommendation: Option 1
- `request_id == UNSPECIFIED_REQUEST_ID` → log only (no owner to deliver to).
- Warning *with* a real `request_id` → deliver to the subscription as a non-fatal **Notice**; subscription doesn't terminate.
- Non-warning error with a real `request_id` → deliver as terminal **Error** (existing behavior, now via the typed envelope).

Rejected:
- **Option 2 (separate warning stream):** doubles wiring for a problem that's "deliver to the existing owner."
- **Option 3 (config flag):** every caller has to know to flip it; default is still wrong.

## Subscription-level design: non-terminal `Notice` variant + #487 alignment

This work and #487 both break `Subscription<T>`'s public API. Doing them in one PR means callers migrate once and the final shape is coherent.

### Combined target signature (sync and async match)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionItem<T> {
    Data(T),
    /// Non-fatal IB notice (warning codes 2100..=2169). Subscription stays open.
    Notice(Notice),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub code: i32,
    pub message: String,
    pub request_id: i32,
}

impl<T> Subscription<T> {
    pub fn next(&self)         -> Option<Result<SubscriptionItem<T>, Error>>;
    pub fn try_next(&self)     -> Option<Result<SubscriptionItem<T>, Error>>;
    pub fn next_timeout(&self, _: Duration) -> Option<Result<SubscriptionItem<T>, Error>>;
}
```

- **From #487:** sync now returns `Option<Result<_, Error>>` instead of `Option<T>` + separate `error()`. Drop the `Mutex<Option<Error>>` field, `error()` accessor, and `should_store_error`/`clear_error` plumbing in `src/subscriptions/sync.rs:33,159,164`.
- **From this issue:** the `Ok` payload widens from `T` to `SubscriptionItem<T>` so warnings deliver as non-terminal `Notice` items.
- Async (`Subscription<T>::next()` already returns `Option<Result<T, Error>>`) widens its `Ok` payload to `SubscriptionItem<T>` symmetrically.

### Convenience accessors for callers that don't care about notices

Most existing call sites just want data values. Add **iterator adapters** that filter notices (logging them) — `next_data()` falls out naturally as `iter_data().next()`, so we don't need three near-identical `next_data` / `try_next_data` / `next_timeout_data` methods:

```rust
impl<T> Subscription<T> {
    pub fn iter_data(&self)         -> impl Iterator<Item = Result<T, Error>> + '_;
    pub fn try_iter_data(&self)     -> impl Iterator<Item = Result<T, Error>> + '_;
    pub fn timeout_iter_data(&self, timeout: Duration) -> impl Iterator<Item = Result<T, Error>> + '_;

    pub fn next_data(&self) -> Option<Result<T, Error>> {
        self.iter_data().next()
    }
}
```

The adapter implementation is one place:

```rust
fn filter_data<I, T>(items: I) -> impl Iterator<Item = Result<T, Error>>
where I: Iterator<Item = Result<SubscriptionItem<T>, Error>>,
{
    items.filter_map(|item| match item {
        Ok(SubscriptionItem::Data(t))   => Some(Ok(t)),
        Ok(SubscriptionItem::Notice(n)) => { warn!(?n, "ib notice"); None }
        Err(e)                          => Some(Err(e)),
    })
}
```

Examples and most internal call sites migrate to `iter_data()` / `next_data()`; consumers that want to react to notices (e.g. show "Market data farm connection is OK" in a UI) use `iter()` / `next()` and pattern-match.

Async mirror — same shape, `Stream` instead of `Iterator`:

```rust
impl<T> Subscription<T> {            // async
    pub fn data_stream(&mut self) -> impl Stream<Item = Result<T, Error>> + '_;
    pub async fn next_data(&mut self) -> Option<Result<T, Error>>;
}
```

The sync `filter_data` and async `filter_data_stream` helpers are structural mirrors, intentionally — sync/async parity is the goal. Not abstractable without a `Stream`/`Iterator` unification, so the small duplication is accepted.

### Why type-driven over inline `error_code` inspection
- Callers can't accidentally treat a notice as fatal data, and reviewers see the distinction at the call site.
- One place to maintain the warning-code policy. Inline `is_warning_error()` checks in every `next()` consumer would drift the moment IB widens the range.
- Notices already exist conceptually in IB's API (2100 series is documented as informational); modeling them in the type system matches the protocol.

## Routing helper + typed channel envelope

The classification helper alone isn't enough. Today the channel carries `Result<ResponseMessage, Error>` (`src/subscriptions/sync.rs:169`) and `T::decode` re-classifies error messages (`src/subscriptions/sync.rs:188`). If we only add `classify_error_delivery` in `routing.rs`, every decoder will *also* need to know about warning codes — classification ends up duplicated. Fix by making the dispatcher emit a typed envelope so classification is consumed, not redone.

### Pure classification (single source of truth)

The classifier returns a four-arm enum so `is_warning_error` is called *once*. The two `Log*` variants pre-decide warn-vs-error log severity:

```rust
// src/transport/routing.rs
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorDelivery {
    /// No subscription owner; log at warn level (warning code).
    LogWarning,
    /// No subscription owner; log at error level.
    LogError,
    /// Non-fatal warning bound to a subscription. Deliver without terminating.
    Notice { request_id: i32 },
    /// Hard error bound to a subscription. Deliver and let the subscription terminate.
    Error { request_id: i32 },
}

pub fn classify_error_delivery(request_id: i32, error_code: i32) -> ErrorDelivery {
    let is_warning = is_warning_error(error_code);
    match (request_id, is_warning) {
        (UNSPECIFIED_REQUEST_ID, true)  => ErrorDelivery::LogWarning,
        (UNSPECIFIED_REQUEST_ID, false) => ErrorDelivery::LogError,
        (id, true)                       => ErrorDelivery::Notice { request_id: id },
        (id, false)                      => ErrorDelivery::Error  { request_id: id },
    }
}
```

### Typed channel envelope

Replace the bare `ResponseMessage` on subscription channels with `RoutedItem`. **This type lives in `src/subscriptions/common.rs`, not in `routing.rs`** — `routing.rs` stays a leaf module that produces only routing decisions; subscription value types live with subscriptions. The dispatcher does the `ErrorDelivery → RoutedItem` translation:

```rust
// src/subscriptions/common.rs
pub enum RoutedItem {
    /// Normal message; subscription's decoder produces SubscriptionItem::Data(T).
    Response(ResponseMessage),
    /// Pre-classified non-fatal notice; decoder maps straight through.
    Notice(Notice),
    /// Pre-classified hard error; subscription terminates.
    Error(Error),
}
```

Subscription read becomes a fork; decoders no longer inspect `IncomingMessages::Error` or call `is_warning_error`:

```rust
match channel_item {
    RoutedItem::Response(rm) => T::decode(ctx, rm).map(SubscriptionItem::Data),
    RoutedItem::Notice(n)    => Ok(SubscriptionItem::Notice(n)),
    RoutedItem::Error(e)     => Err(e),
}
```

**Why `Response` carries raw bytes but `Notice` and `Error` carry projected fields:** warnings and hard errors can be classified at the dispatcher (just by inspecting `error_code`) without invoking a per-`T` decoder, so they're pre-built into typed values. Data messages still need the subscription's per-`T` decoder, which is why `Response` stays as `ResponseMessage`. The asymmetry is intentional — don't try to "make it consistent" by wrapping data in a typed payload at the dispatcher.

This applies uniformly to request-keyed and order-keyed subscriptions (per Q1: extend the `SubscriptionItem` shape across both).

### Notice / Error constructors (dedupe sync/async)

`Notice` reads its id from the message itself — no separate `request_id` parameter. `Error` already has `impl From<ResponseMessage> for Error` at `src/errors.rs:114`; mirror it for `Notice`:

```rust
// next to the Notice struct (src/subscriptions/common.rs)
impl From<ResponseMessage> for Notice {
    fn from(message: ResponseMessage) -> Self {
        Notice {
            code: message.error_code(),
            message: message.error_message(), // already returns String
            request_id: message.error_request_id(),
        }
    }
}
```

Both dispatchers call `Notice::from(message)` and `Error::from(message)` — same convention, no duplicated field-projection logic, and the routing-extracted `request_id` is only used as the *channel key* (not re-stuffed into the value).

### Sync dispatcher (`src/transport/sync/mod.rs:268`)

```rust
RoutingDecision::Error { request_id, error_code } => {
    let routed = self.send_order_update(&message);
    match classify_error_delivery(request_id, error_code) {
        ErrorDelivery::LogWarning => {
            warn!("Warning - Code: {error_code}, Message: {}", message.error_message());
        }
        ErrorDelivery::LogError => {
            error!("Error - Code: {error_code}, Message: {}", message.error_message());
        }
        ErrorDelivery::Notice { request_id } => {
            self.deliver_to_request_id(request_id, RoutedItem::Notice(Notice::from(message)), routed);
        }
        ErrorDelivery::Error { request_id } => {
            self.deliver_to_request_id(request_id, RoutedItem::Error(Error::from(message)), routed);
        }
    }
}
```

Async (`src/transport/async.rs:418`) is structurally identical. Order-channel fallback (`async.rs:443-449`) is preserved by `deliver_to_request_id` — that helper tries request_channels first, then order_channels.

### `error_event` removal

After the change `error_event` (`src/transport/sync/mod.rs:600`) is only reachable from `LogWarning` / `LogError` arms, which now collapse to one `warn!` / `error!` line each. **Delete it** and rely on the inlined log calls above. Same for the async equivalent.

## Plan refinements (decided 2026-05-04, review-driven)

Forward-looking adjustments after the duplication / SRP / composability lens review. Each refinement names the PR that owns it.

1. **`ErrorDelivery` is a struct, not a flat enum** *(PR 3, breaking change to the original design above)*. The four-variant `LogWarning|LogError|Notice|Error` shape conflates two orthogonal axes — routing (owner-bound vs. unrouted) and severity (warning vs. hard error). PR 5's broadcast filter wants "is unrouted" without caring about severity; PR 3's logger wants "what severity" without caring about routing. Use:
   ```rust
   pub struct ErrorDelivery { pub routing: Routing, pub severity: Severity }
   pub enum Routing  { Unrouted, Owned(i32) /* request_id */ }
   pub enum Severity { Warning, HardError }
   ```
   Each consumer matches on its axis. Defer or skip if PR 3 implementation finds the flat enum reads better — but make the call deliberately, not by inertia.

2. **`Notice::from_decoded(&DecodedError)` preserves `advanced_order_reject_json`** *(PR 3)*. Today `DecodedError` (in `routing.rs`) carries `advanced_order_reject_json` and `error_time` that the existing public `Notice` (`messages.rs:1304`) does not expose. PR 3's `Notice::from(message)` would silently drop the JSON. Add the field to `Notice` so downstream consumers can surface order-rejection details to a UI without us reshaping the public type later. If we don't want to widen `Notice` now, add a `pub(crate)` constructor that captures it for internal use and document the deferral.

3. **`SubscriptionItem<T>::into_data(self) -> Option<T>` accessor** *(✅ shipped in PR 2b, [#504](https://github.com/wboayue/rust-ibapi/pull/504) commit `857890e`)*. The free function `filter_data` in `subscriptions/sync.rs` did this for iterators; a method form is more discoverable from the type, chains naturally, and tightens the test code in PR 3 + 4 that asserts on the inner value. Pulled forward into PR 2b alongside the iterator-adapter collapse.

4. **PR 2c: async `data_stream` Stream adapter** *(new pre-PR before PR 3)*. PR 2b shipped `next_data().await` but deferred the `impl Stream<Item = Result<T, Error>>` mirror. Closes the sync/async composability gap before PR 3 + 4 tests entrench around `next_data().await` loops. Tiny — ~30 lines via `futures::stream::unfold`. Land before PR 3 starts to avoid drift; can land after PR 4 as well if no test in PR 3/4 needs `Stream` combinators.

5. **`dual_test!` macro for sync/async test pairs** *(experiment in PR 3)*. PR 3 says "all mirrored in async"; PR 4 same. Each test pair is structurally identical with different fixtures. Prototype a `dual_test!(name, |client, contract| { /* body */ })` macro that emits sync+async variants. If it doesn't pay for itself by the third pair, abandon. Worth the experiment because PR 4 has 4–5 more test pairs.

6. **`log_unrouted(severity, notice)` helper in PR 3** *(prep for PR 5)*. PR 3's `LogWarning`/`LogError` arms each carry one log line. PR 5 grafts a `broadcast_notice(...)` call onto each. Designing PR 3's arms as `self.log_unrouted(severity, &notice)` from day one means PR 5 adds the broadcast in one place, not two. Costs nothing extra in PR 3.

7. **Extract `NoticeBroadcaster` struct in PR 5** *(SRP)*. Composing `notice_broadcast: NoticeBroadcaster` into `Server` keeps "broadcast lifecycle (subscribe / fan-out / prune)" out of the dispatcher's responsibilities. The broadcaster gets its own unit tests; dispatcher tests don't have to know about it. Folds in the future replay-buffer follow-up cleanly if it ever happens.

8. **Pin down `notice_stream()` return type before PR 5** *(decision needed)*. Returning `Subscription<Notice>` reuses the `iter_data()`/`next_data()`/`next()` API that data subscriptions already expose — strong composition. But `SubscriptionItem<Notice>::Notice(Notice)` ("a notice on a notice stream") is a semantic oddity. Two clean options: (a) keep `Subscription<Notice>` and document that the `Notice` arm of `SubscriptionItem` is unreachable for global streams; (b) introduce `NoticeStream` returning bare `Notice` items. (a) is API-uniform; (b) is semantically cleaner. Decide explicitly in PR 5's design discussion.

## Implementation as a series of PRs

Six PRs, each with a clear, standalone deliverable. Merge in order: **PR 1 → PR 2a → PR 2b → (PR 2c, optional) → PR 3 → PR 4 → PR 5.**

PR 2 is split because the internal channel-envelope refactor (2a) and the public-API widening that closes #487 (2b) are individually reviewable and the combined diff is too large to review well. Land 2a/2b in close succession to minimize the time the codebase carries a defined-but-unused `RoutedItem::Notice` arm.

PR 5 (global notice stream) is an additive API that doesn't depend on PR 4 — it can land any time after PR 3 if PR 4 lags.

### PR 1 — Protobuf Error full decode ✅ merged (#502)
**Goal:** populate real `error_code` and `error_message` for protobuf-encoded Error messages so downstream classification works on the protobuf path.

**Scope:**
- Define a `prost`-derived `ErrorEnvelope` in `src/transport/routing.rs` (or import generated proto if available) covering `id`, `error_code`, `error_message`.
- Replace the `protobuf_first_int` usage in the Error branch (`routing.rs:69`) with full decode.
- Populate `RoutingDecision::Error { request_id, error_code }` from real values.

**Tests:** unit test that a protobuf Error with code 2100 + request_id=42 produces `RoutingDecision::Error { request_id: 42, error_code: 2100 }`.

**Dependencies:** none. Standalone, mergeable on its own.

**Risk:** finding the right protobuf shape — may need to grep the C# reference client for the Error proto schema.

---

### PR 2a — Internal channel envelope refactor ✅ merged ([#503](https://github.com/wboayue/rust-ibapi/pull/503))
**Goal:** widen the internal channel from `Result<ResponseMessage, Error>` (sync) / `ResponseMessage` (async) to a typed `RoutedItem` envelope so subsequent PRs can deliver pre-classified `Notice` and `Error` items without re-classifying inside decoders. **Public `Subscription<T>` API is unchanged.** Examples and integration tests are untouched.

**Scope reductions vs. original plan** (decided 2026-05-03):
- **Reuse existing `Notice`** at `src/messages.rs:1304` instead of adding a new one. The existing shape is `{ code, message, error_time }`; `request_id` rides on routing metadata (not on the value) until PR 3 needs it.
- **Manual `impl Clone for Error`** in `src/errors.rs` so `RoutedItem` can be `Clone` (required by `tokio::sync::broadcast` on the async side). `Io` variant cloned as `io::Error::new(kind, msg)` to preserve kind + message.
- **Decoder `IncomingMessages::Error` arms left alone in 2a.** Dispatcher pre-classification into `RoutedItem::Error` makes them unreachable but harmless. Cleanup happens in PR 3 alongside the dispatcher classification rewrite.
- **Legacy-shape compatibility shim at `InternalSubscription::next`.** Channel item is `RoutedItem` internally, but the public `next/try_next/next_timeout` methods return `Option<Result<ResponseMessage, Error>>` via the `RoutedItem::into_legacy` method (Notice → recv-next loop). Result: zero migration cost in domain consumers (contracts, news, scanner, historical, …). Subscription<T>'s `handle_response` stays in legacy shape too. PR 2b/3 widens the public API when ready.

**As-shipped diff:** 11 files, +234/-97. Touches `errors.rs`, `market_data/historical/mod.rs` (Clone derive), `stubs.rs`, `subscriptions/{common,mod,sync,async}.rs`, `transport/{mod,sync/mod,async}.rs`. Verified: fmt clean, all three clippy configs clean, full test suite green (sync 910 lib + 124 doc, async 907 + 73, all-features 1094 + 148). One new unit test: `subscriptions::sync::tests::test_routed_item_error_terminates_subscription`.

**Scope — new types (`src/subscriptions/common.rs`):**
- `RoutedItem = Response(ResponseMessage) | Notice(Notice) | Error(Error)` with `Debug` + `Clone` derives. The `Notice` variant is `#[allow(dead_code)]` in 2a — defined but never written by the dispatcher.
- `SubscriptionItem<T>` is **not** introduced yet — it's a public-API type that belongs with the API change in 2b.

**Scope — channel transport:**
- Channel item changes from `Result<ResponseMessage, Error>` to `RoutedItem`.
- Dispatcher (sync `dispatch_message` and async `route_error_message`) adapts existing write sites: data → `RoutedItem::Response`, hard error → `RoutedItem::Error(Error::from(message))`. The `RoutedItem::Notice` variant is defined but **never written by the dispatcher** in 2a — warnings continue through the legacy `error_event` log path.
- `handle_response` (sync.rs:169) and the async equivalent pattern-match on `RoutedItem`. The `Response(rm)` arm reuses the existing flow: `process_decode_result(T::decode(...))` → `ProcessingResult::{Success, Skip, EndOfStream, Error}` translated as today (see "Skip handling" below). The `Notice(_)` arm is unreachable in 2a; log defensively and skip. The `Error(e)` arm stores the error in `Mutex<Option<Error>>` exactly as the current `Some(Err(_))` path does.
- Decoders no longer inspect `IncomingMessages::Error` or call `is_warning_error` (the dispatcher pre-classified those into `RoutedItem::Error`).

**Skip handling (option a — preserves decoder contract).** Keep `process_decode_result` and `ProcessingResult`. The new match site looks like:
```rust
match channel_item {
    RoutedItem::Response(mut rm) => match process_decode_result(T::decode(&self.context, &mut rm)) {
        ProcessingResult::Success(t)   => /* existing Return(Some(t)) path */,
        ProcessingResult::Skip         => NextAction::Skip,
        ProcessingResult::EndOfStream  => /* existing stream-ended path */,
        ProcessingResult::Error(e)     => /* existing store-and-return-None path */,
    },
    RoutedItem::Notice(_)        => { warn!("notice arm unreachable in 2a"); NextAction::Skip }
    RoutedItem::Error(e)         => /* same as ProcessingResult::Error path */,
}
```
Decoder signatures stay `Result<T, Error>`. `process_decode_result` and `should_store_error` survive 2a — they're deleted in 2b when the `Mutex<Option<Error>>` field goes.

**Tests:**
- Existing subscription tests adapt to the new internal channel item type but assert the same outward behavior.
- New unit: dispatcher writes `RoutedItem::Error` for a terminal error; `Subscription::next()` returns `None` and `Subscription::error()` returns `Some`.

**Dependencies:** none. Can land in parallel with PR 1.

**Risk:** internal-only; no consumer migration. `MessageBusStub`-based tests need their channel item type updated, but that's mechanical.

---

### PR 2b — Widen `Subscription<T>` public API + migrate consumers (closes #487) ✅ merged ([#504](https://github.com/wboayue/rust-ibapi/pull/504))
**Goal:** widen the `Ok` payload to `SubscriptionItem<T>`, align sync with async per #487, add iterator adapters, migrate every consumer. After this PR the codebase is ready for PR 3 to actually emit notices.

**As-shipped diff:** 78 files, +842/-575. New public type `SubscriptionItem<T>` at `ibapi::subscriptions::SubscriptionItem` plus `SubscriptionItem::into_data() -> Option<T>` accessor. Sync drops `error()` accessor + `Mutex<Option<Error>>` field; errors flow via `Err` arm. New `next_data()` / `iter_data()` / `try_iter_data()` / `timeout_iter_data()` filter notices for callers that don't care. Notice arm structurally present but unreachable until PR 3 emits notices from the dispatcher. All 3 clippy configs clean; tests green at sync 911 lib + 119 doc, async 910 lib + 73 doc, all-features 1098 lib + 143 doc.

**Iterator-adapter collapse (added 2026-05-04, commit `857890e`):** Three near-identical `SubscriptionDataIter` / `SubscriptionTryDataIter` / `SubscriptionTimeoutDataIter` structs (~120 lines) replaced by one `FilterData<I>` adapter + `SubscriptionItemIterExt::filter_data()` extension trait. Any iterator yielding `Result<SubscriptionItem<T>, Error>` (including user-defined ones) can compose `.filter_data()` to yield `Result<T, Error>`. `iter_data()` / `try_iter_data()` / `timeout_iter_data()` keep their public method signatures; their return types are now `FilterData<SubscriptionIter<...>>` etc. `Subscription::next_data()` (sync) literally delegates to `iter_data().next()` — its docstring's claim of equivalence is now true. Public exports (`subscriptions/mod.rs`) drop `Subscription*DataIter` and add `FilterData` + `SubscriptionItemIterExt`.

**Bug-fix shipped alongside (commit `2726d59`):** Async `Subscription<T>::next` `PreDecoded` arm now flips `stream_ended` on terminal `Err` so subsequent calls return `None` deterministically; previously it would re-poll the receiver. New regression test `test_pre_decoded_error_terminates_stream`.

**Scope reductions vs. original plan** (decided 2026-05-04):
- **`SubscriptionItem<T>` derives `PartialEq` only, not `Eq`.** The existing public `Notice` (at `src/messages.rs:1304`) only derives `PartialEq` because `OffsetDateTime` doesn't impl `Eq`. Adding `Eq` to `Notice` would have wider implications; deferring keeps the change scoped.
- **Async `data_stream` (`impl Stream<Item = Result<T, Error>>`) not shipped.** Async users have `next_data().await` which covers the `while let Some(x) = sub.next_data().await { ... }` pattern that `data_stream` would have made into `while let Some(x) = sub.data_stream().next().await`. The `Stream` trait wrapper requires `futures::Stream` plumbing for marginal idiomatic gain. Add as a follow-up if a real consumer asks. The ergonomic equivalence:
  ```rust
  // PR 2b — what shipped:
  while let Some(item) = sub.next_data().await { handle(item?); }
  // What `data_stream` would have offered:
  let mut s = sub.data_stream();
  while let Some(item) = s.next().await { handle(item?); }
  ```

**Scope — new public type (`src/subscriptions/common.rs`):**
- `SubscriptionItem<T> = Data(T) | Notice(Notice)` with `Debug/Clone/PartialEq/Eq/Serialize/Deserialize` derives.

**Scope — public `Subscription<T>` API:**
- Sync `next` / `try_next` / `next_timeout` return `Option<Result<SubscriptionItem<T>, Error>>` (gain `Result`, drop the separate `error()` accessor and `Mutex<Option<Error>>` field, drop `should_store_error` / `clear_error` and `process_decode_result`'s storage-side caller).
- Async `next` widens `Ok` to `SubscriptionItem<T>` symmetrically.
- Iterator adapters: `iter_data` / `try_iter_data` / `timeout_iter_data` plus single shared `filter_data` impl. `next_data()` falls out as `iter_data().next()`. Async mirror: `data_stream`, async `next_data`.
- Order/account subscriptions get the same shape automatically — every accessor returns `Subscription<T>`, the same generic, so widening it widens all of them. No separate type to design. Test coverage in PR 3 picks one representative per channel-keying class:

  | Routing | Representatives | Notice possible? | PR 3 test coverage |
  |---|---|---|---|
  | Request-keyed | `MarketData`, `RealTimeBars`, `HistoricalData` | Yes (e.g. 2104) | One test (market-data) |
  | Order-keyed | `PlaceOrder`, `CancelOrder`, `OrderUpdate` | Yes (order warnings) | One test (place_order) |
  | Shared-keyed | `Orders` (`open_orders`, `completed_orders`), `account_summary` | No — shared channel has no per-subscription error path | Skip |

  Shared-channel subscriptions never receive notices by design (the dispatcher has no `request_id` to route by). Document this in the PR 2b body.

**Scope — consumer migration:** all 53 `.next()` / `.error()` call sites across 39 files in `examples/` migrate to `iter_data()` / `next_data()` (or pattern-match `next()` for consumers that care about notices). Doc-tests on `Subscription::next` (sync.rs:113-134, 220-244) and similar — kept as `no_run`, mechanical sed sweep. `tests/*.rs` does not consume subscription `next()` and is unaffected. Closes #487.

**Tests:**
- Iterator adapters: `iter_data()` over `[Data, Data]` (no notices possible yet — dispatcher still doesn't emit them) yields both data items; `iter()` likewise.
- `iter_data()` after a terminal `Err` yields the error then ends.
- All existing subscription tests adapt to the new return shape.

**Dependencies:** PR 2a.

**Risk — largest blast radius.** Touches every example. Some examples are hand-verified against a live gateway only; flag for manual smoke-test before merge. Mitigation: land immediately after 2a so the diff is mechanical and the `RoutedItem::Notice` defensive-log arm is short-lived.

---

### PR 2c — Async `data_stream` Stream adapter (optional pre-PR before PR 3)
**Goal:** close the sync/async composability gap deferred from PR 2b. Sync has `iter_data()` returning `impl Iterator<Item = Result<T, Error>>`; async users currently have only `next_data().await`. Add an async mirror returning `impl Stream<Item = Result<T, Error>>`.

**Scope:**
```rust
impl<T> Subscription<T> {  // async
    pub fn data_stream(&mut self) -> impl Stream<Item = Result<T, Error>> + '_;
}
```
Implement via `futures::stream::unfold` over `self.next_data().await`. ~30 lines.

**Tests:** stream over `[Data, Data]` collects to two items; stream after terminal `Err` yields the error then ends.

**Dependencies:** PR 2b.

**Risk:** none — purely additive. Skip entirely if PR 3/4 don't end up needing `Stream` combinators.

**Decision rule:** land before PR 3 starts iff PR 3's planned tests use `Stream` combinators (`.take(n)`, `.collect()`, `.filter()`); otherwise defer or drop.

---

### PR 3 — Warning classification & delivery
**Goal:** the actual feature — route warnings with a real `request_id` to their owning subscription as non-terminal `Notice` items.

**Scope — classification:**
- Add `ErrorDelivery` and `classify_error_delivery` in `src/transport/routing.rs`. Per refinement #1, prefer the struct shape:
  ```rust
  pub struct ErrorDelivery { pub routing: Routing, pub severity: Severity }
  pub enum Routing  { Unrouted, Owned(i32) }     // request_id when Owned
  pub enum Severity { Warning, HardError }       // Warning iff is_warning_error(code)

  pub fn classify_error_delivery(request_id: i32, error_code: i32) -> ErrorDelivery {
      ErrorDelivery {
          routing:  if request_id == UNSPECIFIED_REQUEST_ID { Routing::Unrouted } else { Routing::Owned(request_id) },
          severity: if is_warning_error(error_code)         { Severity::Warning  } else { Severity::HardError },
      }
  }
  ```
  Each consumer matches on its axis. If PR 3 implementation finds the flat enum reads better, fall back to it — but make the call deliberately.

**Scope — dispatcher rewrite:**
- Sync `dispatch_message` (`src/transport/sync/mod.rs:268-293`): rewrite the `RoutingDecision::Error` arm around `classify_error_delivery`. `RoutedItem::Notice(_)` now actually populated.
- Async `route_error_message` (`src/transport/async.rs:418-458`): same shape.
- Add `deliver_to_request_id` (two parallel implementations, one per transport — `Mutex` vs `RwLock` divergence prevents sharing) with request-channels-first / order-channels-fallback policy.
- Per refinement #6, **factor a `log_unrouted(severity, &notice)` helper** so PR 5's broadcast call grafts onto one helper rather than two parallel arms:
  ```rust
  fn log_unrouted(&self, severity: Severity, notice: &Notice) {
      match severity {
          Severity::Warning   => warn!("warning: {notice}"),
          Severity::HardError => error!("error: {notice}"),
      }
      // PR 5 adds: self.broadcast_notice(notice.clone());
  }
  ```
- **Delete** `error_event` (`sync/mod.rs:600`) and async equivalent.
- **Delete** duplicate `WARNING_CODES` const (`sync/mod.rs:30`).
- Verify whether `Error::Message(code, msg)` is still reachable; delete if dead.

**Scope — value type tightening (refinements #2, #3):**
- Add `Notice::from_decoded(&DecodedError)` (or extend `impl From<&ResponseMessage> for Notice`) to capture `advanced_order_reject_json` and `error_time` from `DecodedError`. Either widen the public `Notice` struct with the new field or keep a `pub(crate)` richer constructor — decide based on whether the PR scope can accommodate the public-type change.
- Add `impl<T> SubscriptionItem<T> { pub fn into_data(self) -> Option<T> }`. Tightens PR 3 + 4 test code that asserts on the inner value.

**Tests:**
- Unit: `classify_error_delivery` covering the four shape combinations and boundary codes (2099, 2100, 2169, 2170).
- End-to-end: code 2100 with `request_id=42` → `subscription.next()` yields `Some(Ok(SubscriptionItem::Notice(_)))`; stream stays open.
- End-to-end: code 2100 with `UNSPECIFIED_REQUEST_ID` → only logs; no channel write.
- End-to-end: real error (code 200) with `request_id=42` → `Some(Err(_))`; subsequent `next()` returns `None`.
- Order-channel fallback: code 2100 with an `order_id` that maps to an order subscription only → notice delivered.
- All mirrored in async. Per refinement #5, prototype a `dual_test!(name, |fixture| { /* body */ })` macro that emits sync+async variants from one body. Abandon if it doesn't pay for itself by the third pair.

**Dependencies:** PR 1 (real protobuf error_code), PR 2a (`RoutedItem` channel), PR 2b (`SubscriptionItem` public type). Optional dependency on PR 2c if PR 3's tests want `Stream` combinators. PR 3 is the first PR where the dispatcher actually writes `RoutedItem::Notice`.

**Risk:** smallest of the three. Order-channel fallback test setup is the trickiest piece.

---

### PR 4 — End-to-end Subscription tests for Notice delivery
**Goal:** wire the dispatcher → subscription path under tests that actually drive `Subscription::next()`, plus an opt-in live-gateway smoke test for release verification. PR 3's tests prove dispatcher-level classification; this PR proves the full Subscription consumer path.

**Approach: synthesized for CI + live `#[ignore]`'d for release smoke (no recordings).**

**Scope — synthesized CI tests** (`tests/notice_delivery.rs`, runs in default `cargo test`):
- Use the lightest fixture per the project rule (`MessageBusStub` likely sufficient — promote to `MemoryStream` only if dispatcher routing isn't exercised). Hand-craft text-format Error packets at codes 2104 (request-keyed notice) and 200 (request-keyed hard error); push through the production dispatcher; assert via `Subscription::next()` and `iter_data()`.
- Cases to cover:
  - Request-keyed notice: code 2104 + request_id=42 → `Some(Ok(SubscriptionItem::Notice(_)))`; subscription stays open.
  - Request-keyed terminal: code 200 + request_id=42 → `Some(Err(_))`; subsequent `next()` returns `None`.
  - Order-keyed notice: order warning code (e.g. 399) + order_id=7 → notice delivered to a `Subscription<PlaceOrder>`.
  - Unspecified: code 2104 + `UNSPECIFIED_REQUEST_ID` → no channel write (assert via fixture's no-message expectation).
  - Mirror sync and async (reuse PR 3's `dual_test!` macro if it survived).

**Scope — live-gateway smoke tests** (`tests/notice_delivery_integration.rs`, every test marked `#[ignore]`, run manually before release):
- Trigger 2104 by opening a market-data subscription; drain a few items; assert at least one `SubscriptionItem::Notice`.
- Trigger 200 with an invalid contract; assert `Some(Err(_))` and stream ends.
- Order-channel path: place order with parameters that trigger a non-fatal warning (e.g. order outside RTH → 404 / 399); assert notice arrives on the place_order subscription.
- Both sync and async if the existing integration-test layout supports it.

**Scope — verification of existing integration tests** (manual hand-run before merge, no new code):
- `tests/conditional_orders_integration.rs`, `tests/order_builder_integration.rs`, `tests/test_wsh_async.rs` go through the dispatcher; run against a live gateway to confirm no order-routing regressions.

**Dependencies:** PR 1 + PR 2a + PR 2b + PR 3 all merged.

**Risk:** synthesized tests verify wiring, not real TWS packet shapes. Mitigation: PR 1's protobuf-decode unit test + PR 3's classification tests already cover packet-shape parsing; PR 4's synthesized tests cover end-to-end consumer behavior. The `#[ignore]`'d live tests are the safety net for protocol-level regressions and are expected to be hand-run before each release.

**Recordings deferred.** Capturing real TWS bytes for replay tests is not in scope for PR 4. If a future regression is missed by the synthesized tests but caught by the live tests, add recordings then — don't pre-build the fixture infrastructure.

---

### PR 5 — Global notice stream
**Goal:** expose IB's globally routed notices (codes with `request_id = -1`, e.g. `1100` lost connectivity, `2104` market-data farm OK) as a programmatic `Subscription<Notice>` so consumers can drive UI status indicators and reconnection logic, instead of scraping logs.

**Motivation.** After PR 3, notices with a real `request_id` reach their owning subscription. But the connectivity and farm-status notices (`1100/1101/1102/2103/2104/2105/2106/2107/2108/2110/2158`) all arrive with `UNSPECIFIED_REQUEST_ID` and the dispatcher's `LogWarning`/`LogError` arms only `warn!`/`error!` them. A UI can't show "🔴 market data farm down" without parsing log output.

**Decision before implementation (refinement #8):** does `notice_stream()` return `Subscription<Notice>` (uniform with data subscriptions, but `SubscriptionItem<Notice>::Notice(Notice)` is semantically odd) or a dedicated `NoticeStream` returning bare `Notice` items (semantically cleaner, but a second consumer-facing API surface)? Pick one in the PR 5 design discussion before writing the implementation. The skeletons below use `Subscription<Notice>` for now.

**Scope — public API:**
```rust
impl Client {                           // sync and async
    /// Subscribe to globally routed notices (code/message with no request owner).
    /// Each call returns a fresh subscription; late subscribers do not see prior notices.
    /// The stream closes when the client disconnects.
    pub fn notice_stream(&self) -> Result<Subscription<Notice>, Error>;
}
```

**Filter scope: severity-agnostic for unrouted only** (using PR 3's `ErrorDelivery` struct shape). The broadcast fires whenever `delivery.routing == Routing::Unrouted`, regardless of severity. Connection-loss errors (e.g. `2110`) matter at least as much as connection-OK warnings; consumers can pattern-match `notice.code` if they only care about a subset.

**Scope — dispatcher:**
- Per refinement #7, **extract a `NoticeBroadcaster` struct** rather than scattering broadcast logic across `Server`. `NoticeBroadcaster` owns the subscriber list and exposes `broadcast(notice)`, `subscribe() -> Receiver<Notice>`, and `prune_dropped()`. `Server` composes it: `notice_broadcaster: NoticeBroadcaster`. The broadcaster gets its own unit tests; dispatcher tests don't have to reach into broadcast internals.
- Sync `NoticeBroadcaster` wraps `Arc<Mutex<Vec<Sender<Notice>>>>` (crossbeam-based to match the rest of the sync transport). Async `NoticeBroadcaster` wraps `tokio::sync::broadcast::Sender<Notice>`. Acceptable mirror divergence (same shape as the `filter_data` / `filter_data_stream` precedent).
- The `log_unrouted` helper from PR 3 (refinement #6) gains the broadcast call here:
  ```rust
  fn log_unrouted(&self, severity: Severity, notice: &Notice) {
      match severity {
          Severity::Warning   => warn!("warning: {notice}"),
          Severity::HardError => error!("error: {notice}"),
      }
      self.notice_broadcaster.broadcast(notice.clone());
  }
  ```
  PR 3's two log arms become one helper call; PR 5 grafts the broadcast onto the helper, not onto two parallel arms.
- `NoticeBroadcaster::broadcast` is best-effort: dropped subscribers are pruned; there is no buffering for late subscribers (matches the broadcast-channel semantics).

**Scope — `Client::notice_stream()`:**
- Sync: calls `notice_broadcaster.subscribe()`, wraps the resulting `Receiver<Notice>` in a `Subscription<Notice>`. `StreamDecoder<Notice>` is identity since the dispatcher pre-constructs the `Notice` value.
- Async: same shape, backed by `tokio::sync::broadcast::Sender::subscribe()`.
- `Notice` already has the `From<&ResponseMessage>` projection from PR 3; the `StreamDecoder<Notice>` impl simply passes it through.

**Scope — example:**
- Add `examples/notice_stream.rs` (sync) and `examples/notice_stream_async.rs` (async) showing a connection-status monitor: subscribe to `notice_stream()`, log/print as `1100`/`1102` arrive.

**Tests:**
- `NoticeBroadcaster` unit tests (don't go through the dispatcher): `broadcast` fan-out to 2 subscribers; late `subscribe` after a `broadcast` receives only subsequent notices; subscriber `Drop` followed by `prune_dropped` removes the dead sender; `broadcast` after all subscribers dropped is a no-op.
- Synthesized end-to-end fan-out: dispatcher receives 2 packets at codes `2104` (warning) and `1100` (warning); two subscribers each call `notice_stream()`; both receive both notices in order.
- Hard-error `UNSPECIFIED_REQUEST_ID` (e.g. code `504` "Not connected") delivered as `Notice` — confirms severity-agnostic filter.
- Sync and async mirrors via `dual_test!` if it survived PR 3.

**Dependencies:** PR 3 (the `log_unrouted` helper must exist as the broadcast attach point — refinement #6 makes this a single edit, not two). Does **not** depend on PR 4 — can land any time after PR 3.

**Risk:** broadcast-fan-out lifecycle bugs (subscribers leaking, ordering assumptions). Mitigation: keep `broadcast_notice` minimal — no per-subscriber filtering, no buffering, no replay. Any of those features can be a follow-up if asked for.

**Out of scope (follow-up PR if asked):**
- Typed `ConnectionStatus` enum for `1100/1101/1102/2110` specifically (option (c) from the design discussion). `notice.code` pattern-matching is sufficient until a real consumer asks for the typed shape.
- Replay buffer for late subscribers.
- Per-subscriber code filtering.

## Cleanups folded in
- `src/transport/sync/mod.rs:30` defines `WARNING_CODES = 2100..=2169` — duplicates `WARNING_CODE_RANGE` in `src/messages.rs:1318`. Drop the local copy; route everything through `is_warning_error`.
- `error_event` in `src/transport/sync/mod.rs:600` is **deleted**; the `LogWarning` and `LogError` arms each carry a single inline `log!` call.

## Tests (per project rule #11 + rule #15)
End-to-end through the dispatcher, not self-loop:

**Routing classification:**
- Unit test for `classify_error_delivery` covering the four variants (`LogWarning`, `LogError`, `Notice`, `Error`) and boundary codes (2099, 2100, 2169, 2170).
- Protobuf Error path (after Step 1): a protobuf Error with code 2100 + request_id=42 produces `RoutingDecision::Error { request_id: 42, error_code: 2100 }`, which classifies as `Notice`.

**Dispatcher → subscription end-to-end:**
- Code 2100 with `request_id=42` → `subscription.next()` yields `Some(Ok(SubscriptionItem::Notice(_)))`; follow-up `next()` is still pending (stream open).
- Code 2100 with `request_id=UNSPECIFIED_REQUEST_ID` → only logs; no channel write.
- Real error (e.g. 200) with `request_id=42` → `subscription.next()` yields `Some(Err(_))`; subsequent `next()` returns `None`.
- **Order-channel fallback:** code 2100 with an `order_id` that maps to an order subscription only (no entry in request_channels) → notice is delivered to the order subscription, not dropped.
- Same set mirrored in async dispatch tests.

**Iterator adapters:**
- `iter_data()` over a stream of `[Data, Notice, Data]` yields the two data items and drops the notice.
- `iter()` over the same stream yields all three.
- `iter_data()` after a terminal `Err` yields the error then ends.

## Branches
**`main` only.** Both #487 and the `Notice` variant are breaking changes to `Subscription<T>` that belong in 3.0. v2-stable keeps current behavior (separate `error()` accessor, log-only warnings).

## Affected code (current paths)

Routing / dispatcher:
- `src/transport/routing.rs:69` — protobuf Error decode (Step 1: full decode, not first-int)
- `src/transport/routing.rs:148` — `is_warning_error` (extend with `classify_error_delivery` + four-arm `ErrorDelivery` enum)
- `src/transport/sync/mod.rs:268-293` — sync `dispatch_message` rewritten around `classify_error_delivery`
- `src/transport/sync/mod.rs:600` — **delete** `error_event`; one `warn!` line in `LogWarning`, one `error!` line in `LogError` (PR 5 adds the `broadcast_notice` fan-out call alongside)
- `src/transport/sync/mod.rs:30` — **delete** duplicate `WARNING_CODES` const
- `src/transport/async.rs:418-458` — async `route_error_message` rewritten symmetrically; `warn!` lives in `LogWarning`, `error!` in `LogError`
- `src/messages.rs:1318` — canonical `WARNING_CODE_RANGE` (unchanged, sole source)
- `src/errors.rs:114` — existing `impl From<ResponseMessage> for Error` reused (no new constructor needed)

Subscription value types (new module-local, not in routing):
- `src/subscriptions/common.rs` — new `RoutedItem` envelope, `Notice` type, `impl From<ResponseMessage> for Notice`, `SubscriptionItem<T>` (with `Debug`/`Clone`/`PartialEq`/`Eq`/`Serialize`/`Deserialize` derives — `serde` is already used elsewhere in the crate)
- `deliver_to_request_id` (request_channels-first, order_channels-fallback) — **two parallel implementations**, one method on each transport's `Server` (sync uses `Mutex`, async uses `RwLock`, can't be a single shared function). Same intentional sync/async mirror as `filter_data` / `filter_data_stream`.

Cleanup verification:
- `src/errors.rs` — confirm `Error::Message(code, msg)` is still reachable after the change. Today it's constructed by decoders for IB error messages (via the `IncomingMessages::Error` path in `T::decode`); after the dispatcher pre-builds `RoutedItem::Error`, decoders never see those messages. Either the variant goes dead (delete) or it's kept as the inner shape inside the pre-built `Error` value built by `impl From<ResponseMessage> for Error`. Resolve during Step 2.

Subscription API (#487 + Notice):
- `src/subscriptions/sync.rs:33` — drop `error: Mutex<Option<Error>>` field
- `src/subscriptions/sync.rs:138,247,288` — change `next` / `try_next` / `next_timeout` signatures to return `Option<Result<SubscriptionItem<T>, Error>>`
- `src/subscriptions/sync.rs:159,164` — remove `error()` accessor and `clear_error`
- `src/subscriptions/sync.rs:169-207` — `handle_response` simplifies: pattern-match on `RoutedItem`, decoder no longer classifies errors
- `src/subscriptions/common.rs:36` — drop `process_decode_result` / `should_store_error` error-storage path
- `src/subscriptions/sync.rs:417-478` — iterator impls yield `Result<SubscriptionItem<T>, Error>`; add `iter_data` / `try_iter_data` / `timeout_iter_data` adapters
- `src/subscriptions/async.rs` — widen `Ok` payload to `SubscriptionItem<T>`; mirror `iter_data` / `next_data` / async stream adapter
- Order-subscription consumers (executions, open-orders) — same `SubscriptionItem` shape per Q1=a
- All examples and integration tests under `examples/` and `tests/` that call `subscription.next()` / `.error()` — migrate to `iter_data()` / `next_data()` (or pattern-match `next()` if they care about notices)
