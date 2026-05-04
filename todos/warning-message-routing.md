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

## Implementation as a series of PRs

Six PRs, each with a clear, standalone deliverable. Merge in order: **PR 1 → PR 2a → PR 2b → PR 3 → PR 4 → PR 5.**

PR 2 is split because the internal channel-envelope refactor (2a) and the public-API widening that closes #487 (2b) are individually reviewable and the combined diff is too large to review well. Land 2a/2b in close succession to minimize the time the codebase carries a defined-but-unused `RoutedItem::Notice` arm.

PR 5 (global notice stream) is an additive API that doesn't depend on PR 4 — it can land any time after PR 3 if PR 4 lags.

### PR 1 — Protobuf Error full decode ✅ open
**Goal:** populate real `error_code` and `error_message` for protobuf-encoded Error messages so downstream classification works on the protobuf path.

**Scope:**
- Define a `prost`-derived `ErrorEnvelope` in `src/transport/routing.rs` (or import generated proto if available) covering `id`, `error_code`, `error_message`.
- Replace the `protobuf_first_int` usage in the Error branch (`routing.rs:69`) with full decode.
- Populate `RoutingDecision::Error { request_id, error_code }` from real values.

**Tests:** unit test that a protobuf Error with code 2100 + request_id=42 produces `RoutingDecision::Error { request_id: 42, error_code: 2100 }`.

**Dependencies:** none. Standalone, mergeable on its own.

**Risk:** finding the right protobuf shape — may need to grep the C# reference client for the Error proto schema.

---

### PR 2a — Internal channel envelope refactor
**Goal:** widen the internal channel from `Result<ResponseMessage, Error>` to a typed `RoutedItem` envelope so subsequent PRs can deliver pre-classified `Notice` and `Error` items without re-classifying inside decoders. **Public `Subscription<T>` API is unchanged.** Examples and integration tests are untouched.

**Scope — new types (`src/subscriptions/common.rs`):**
- `Notice { code, message, request_id }` with `Debug/Clone/PartialEq/Eq/Serialize/Deserialize` derives.
- `impl From<ResponseMessage> for Notice` (mirror existing `impl From<ResponseMessage> for Error` at `src/errors.rs:114`).
- `RoutedItem = Response(ResponseMessage) | Notice(Notice) | Error(Error)`.
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

### PR 2b — Widen `Subscription<T>` public API + migrate consumers (closes #487)
**Goal:** widen the `Ok` payload to `SubscriptionItem<T>`, align sync with async per #487, add iterator adapters, migrate every consumer. After this PR the codebase is ready for PR 3 to actually emit notices.

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

### PR 3 — Warning classification & delivery
**Goal:** the actual feature — route warnings with a real `request_id` to their owning subscription as non-terminal `Notice` items.

**Scope — classification:**
- Add `ErrorDelivery` enum (`LogWarning`/`LogError`/`Notice`/`Error`) and `classify_error_delivery` in `src/transport/routing.rs`.

**Scope — dispatcher rewrite:**
- Sync `dispatch_message` (`src/transport/sync/mod.rs:268-293`): rewrite the `RoutingDecision::Error` arm around `classify_error_delivery`. `RoutedItem::Notice(_)` now actually populated.
- Async `route_error_message` (`src/transport/async.rs:418-458`): same shape.
- Add `deliver_to_request_id` (two parallel implementations, one per transport — `Mutex` vs `RwLock` divergence prevents sharing) with request-channels-first / order-channels-fallback policy.
- **Delete** `error_event` (`sync/mod.rs:600`) and async equivalent — `LogWarning`/`LogError` arms each carry one inline `log!` call.
- **Delete** duplicate `WARNING_CODES` const (`sync/mod.rs:30`).
- Verify whether `Error::Message(code, msg)` is still reachable; delete if dead.

**Tests:**
- Unit: `classify_error_delivery` covering the four variants and boundary codes (2099, 2100, 2169, 2170).
- End-to-end: code 2100 with `request_id=42` → `subscription.next()` yields `Some(Ok(SubscriptionItem::Notice(_)))`; stream stays open.
- End-to-end: code 2100 with `UNSPECIFIED_REQUEST_ID` → only logs; no channel write.
- End-to-end: real error (code 200) with `request_id=42` → `Some(Err(_))`; subsequent `next()` returns `None`.
- Order-channel fallback: code 2100 with an `order_id` that maps to an order subscription only → notice delivered.
- All mirrored in async.

**Dependencies:** PR 1 (real protobuf error_code), PR 2a (`RoutedItem` channel), PR 2b (`SubscriptionItem` public type). PR 3 is the first PR where the dispatcher actually writes `RoutedItem::Notice`.

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
  - Mirror sync and async.

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

**Scope — public API:**
```rust
impl Client {                           // sync and async
    /// Subscribe to globally routed notices (code/message with no request owner).
    /// Each call returns a fresh subscription; late subscribers do not see prior notices.
    /// The stream closes when the client disconnects.
    pub fn notice_stream(&self) -> Result<Subscription<Notice>, Error>;
}
```

**Filter scope: both `LogWarning` and `LogError`.** Connection-loss errors (e.g. `2110`) matter at least as much as connection-OK warnings; consumers can pattern-match `notice.code` if they only care about a subset.

**Scope — dispatcher:**
- Add a `notice_broadcast` field to `Server` (sync: `Arc<Mutex<Vec<Sender<Notice>>>>`; async: `tokio::sync::broadcast::Sender<Notice>`). Sync uses fan-out over a `Vec<Sender>` because the existing sync transport is `crossbeam_channel`-based and adding `tokio::sync::broadcast` for one channel isn't worth the runtime dependency. Acceptable mirror divergence (same shape as the `filter_data` / `filter_data_stream` precedent).
- `LogWarning` and `LogError` arms in PR 3's dispatcher rewrite gain a fan-out call:
  ```rust
  ErrorDelivery::LogWarning => {
      let notice = Notice::from(message);
      warn!("Warning - Code: {}, Message: {}", notice.code, notice.message);
      self.broadcast_notice(notice);
  }
  ErrorDelivery::LogError => {
      let notice = Notice::from(message);
      error!("Error - Code: {}, Message: {}", notice.code, notice.message);
      self.broadcast_notice(notice);
  }
  ```
- `broadcast_notice` is best-effort: dropped subscribers (`SendError`) are pruned from the `Vec`; there is no buffering for late subscribers (matches the broadcast-channel semantics).

**Scope — `Client::notice_stream()`:**
- Sync: registers a fresh `Sender<Notice>` in `notice_broadcast`, returns a `Subscription<Notice>` whose internal channel reads from the matching `Receiver`. `cancel()` removes the sender. `StreamDecoder<Notice>` for this subscription is trivial — the dispatcher already produces `Notice` values; the decoder is identity.
- Async: subscribes to the broadcast channel via `Sender::subscribe()`; the resulting `Receiver` drives a `Subscription<Notice>` whose stream yields each `Notice`.
- `Notice` is already a `StreamDecoder`-friendly type after PR 2a (it has the projection from `ResponseMessage`); add a `StreamDecoder<Notice>` impl that returns the value passed through (since the dispatcher pre-constructed it).

**Scope — example:**
- Add `examples/notice_stream.rs` (sync) and `examples/notice_stream_async.rs` (async) showing a connection-status monitor: subscribe to `notice_stream()`, log/print as `1100`/`1102` arrive.

**Tests:**
- Synthesized fan-out: dispatcher receives 2 packets at codes `2104` (warning) and `1100` (warning); two subscribers each call `notice_stream()`; both receive both notices in order.
- Late subscriber: subscriber registers after a notice has been dispatched; receives only subsequent notices.
- Pruning: subscriber drops; subsequent notice doesn't error and the dropped sender is removed from the `Vec` (sync) or handled by broadcast's built-in cleanup (async).
- Hard-error `UNSPECIFIED_REQUEST_ID` (e.g. code `504` "Not connected") delivered as `Notice` — confirms `LogError` filter is included.
- Sync and async mirrors.

**Dependencies:** PR 3 (the `LogWarning`/`LogError` arms must exist as classification points). Does **not** depend on PR 4 — can land any time after PR 3.

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
