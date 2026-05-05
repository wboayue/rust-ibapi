# Warning Message Routing Enhancement

## Status
**Priority:** Medium
**Type:** Enhancement / Bug
**Recommendation:** Option 1 — route warnings with a real `request_id` to their owning subscription
**Bundled with:** [#487 — Align sync `Subscription<T>::next()` with async](https://github.com/wboayue/rust-ibapi/issues/487). Both are breaking changes to `Subscription<T>` on `main`; ship together so consumers migrate once.

## Maintaining this plan

Update each PR's section as it moves through the chain — the plan is a record of *intent + outcomes*, not a synced spec.

- **In flight:** append `(#NNN)` to the section heading (link to the open PR).
- **Merged:** append `✅ merged ([#NNN](URL))` to the heading; promote any as-shipped notes (diff size, scope reductions, late-stage refactors discovered during review) into the section so downstream PRs can reason against the actual shipped shape, not the original plan. See PR 1 / PR 2a / PR 2b for the format.
- **Divergence:** when a PR's design diverges from this doc, capture the divergence in that PR's section (under "Scope reductions" / "Scope additions" / "As-shipped diff") rather than rewriting upstream sections. Future-you needs the trail of decisions.

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

Eight PRs, each with a clear, standalone deliverable. Merge in order: **PR 1 → PR 2a → PR 2b → PR 2c → PR 3 → PR 4 → PR 5 → PR 6.**

PR 2 is split because the internal channel-envelope refactor (2a) and the public-API widening that closes #487 (2b) are individually reviewable and the combined diff is too large to review well. Land 2a/2b in close succession to minimize the time the codebase carries a defined-but-unused `RoutedItem::Notice` arm.

PR 5 (global notice stream) is an additive API that doesn't depend on PR 4 — it can land any time after PR 3 if PR 4 lags.

PR 6 (docs / migration guide) lands last because it consolidates the user-facing story across PR 3 (per-subscription notices), PR 4 (verified behavior), and PR 5 (global notice stream). Inline doc-comments + examples that ship with each feature PR are still mandatory; PR 6 is for the README narrative + migration guide that ties them together.

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

### PR 2c — Async `data_stream` Stream adapter ✅ merged ([#505](https://github.com/wboayue/rust-ibapi/pull/505))
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

**Scope additions during self-review (2026-05-04):**
- **Return type widened to `+ Unpin`** via `Box::pin(unfold(...))`. The plan's `impl Stream + '_` is `!Unpin` (unfold's future is self-referential), forcing every caller into `pin_mut!` boilerplate. One heap alloc per `data_stream()` call buys ergonomic `.collect()`/`.next()` chaining; alloc is amortized over the entire stream consumption. The same `Box::pin` shape applies to the new `stream()` below.
- **Added async `stream()`** returning `impl Stream<Item = Result<SubscriptionItem<T>, Error>> + Unpin + '_` — async mirror of sync `iter()`. Without it, notice-aware consumers had Stream combinators only on the data-only path. Symmetric to sync.
- **Extracted `filter_notice()` helper** at `src/subscriptions/common.rs`. Sync `FilterData::next` and async `next_data` had identical match expressions (`Ok(Data) → t`, `Ok(Notice) → log+skip`, `Err → propagate`). Both now delegate.
- **Dropped `T: Send` bound** from `data_stream` — was over-restricting; `next_data` only requires `T: 'static` and `Box::pin` adds no `Send` requirement.
- **Direct unit test for `filter_notice`** in `common_tests.rs`. The Notice arm wasn't reachable from any other test in this PR's state — `AsyncInternalSubscription::next` filters `RoutedItem::Notice` via `into_legacy()` before the user-facing `Subscription::next()`. PR 3 adds the end-to-end Notice flow; until then the helper itself needs direct coverage.

**As-shipped diff:** 6 files, +190/-20 LOC.

---

### PR 3 — Warning classification & delivery (in flight)
**Goal:** the actual feature — route warnings with a real `request_id` to their owning subscription as non-terminal `Notice` items.

**As-shipped diff (local branch):** 17 files, +551/-108 LOC. fmt clean; clippy clean across default, sync-only, and `--all-features` with `-D warnings`; full test suite green (sync 1113 lib + 143 doc, async 921 lib + 1 doc, all-features path 1113 sync). New tests: 5 dispatcher-level (sync) + 5 dispatcher-level (async) covering owned warning, unrouted warning, hard error, order-channel fallback, plus a `Subscription<T>` end-to-end Notice surface test in both sync/async.

**Scope reductions vs. original plan** (decided 2026-05-04):
- **`Notice` widened in-place rather than via a separate constructor.** The plan offered a choice between widening the public `Notice` struct or keeping a `pub(crate)` richer constructor. Shipped: added `advanced_order_reject_json: String` to the public `Notice` struct (with `#[serde(default, skip_serializing_if = "String::is_empty")]`) and a `pub(crate) fn notice_from_decoded(&DecodedError) -> Notice` in `routing.rs` that captures both the JSON and `error_time` (millis → `OffsetDateTime`). Touched 13 existing `Notice { .. }` literals across messages tests, subscription tests, and `connection/common.rs` to add the new field — mechanical.
- **No `dual_test!` macro experiment** (refinement #5). Each pair of sync/async tests is structurally similar but has enough fixture divergence (crossbeam channels vs `tokio::broadcast::channel`, sync vs async receiver wait) that a macro wouldn't have shaved meaningful lines. Skipped.
- **No `SubscriptionItem::into_data` in this PR** (refinement #3) — already shipped in PR 2b commit `857890e`.
- **Per-T `Notice` enum variants left intact** (`PlaceOrder::Message(Notice)`, `Orders::Notice(Notice)`, `MarketDepths::Notice(Notice)`, `TickTypes::Notice(Notice)`, etc.). The dispatcher pre-classification means decoders' `IncomingMessages::Error` arms are unreachable on the production path, but `MessageBusStub`-based tests still feed raw `RoutedItem::Response` messages through the decoder, so those arms remain reachable in tests. Removing the public enum variants would be a separate, large public-API change touching ~10 examples and several test files; deferred. Cleanup is a candidate for a follow-up PR if/when the per-T variants are confirmed truly redundant with `SubscriptionItem::Notice`.
- **`Error::Message(code, msg)` kept reachable.** It's now constructed by the dispatcher (`RoutedItem::Error(Error::Message(payload.error_code, payload.error_message.clone()))`) for hard errors with a real `request_id`. The plan's "verify whether `Error::Message` is still reachable; delete if dead" check resolves to: still reachable, keep it.

**As-shipped — classification (`src/transport/routing.rs`):**
- `ErrorDelivery { routing: Routing, severity: Severity }` struct + `classify_error_delivery(request_id, error_code) -> ErrorDelivery` (refinement #1, struct shape).
- `notice_from_decoded(&DecodedError) -> Notice` constructor (preserves JSON + converts ms→`OffsetDateTime`).
- Removed `DecodedError::is_log_only` — all callers migrated to `classify_error_delivery`.

**As-shipped — dispatcher rewrite:**
- Sync `dispatch_message` and async `route_error_message` rewritten around `classify_error_delivery`. Pre-classified `RoutedItem::{Notice, Error}` written to channels; `RoutedItem::Response` still flows for non-error messages.
- New `deliver_to_request_id` on each transport (sync `TcpMessageBus`, async `AsyncTcpMessageBus`) — request channel first, order channel fallback. Two parallel impls (sync `RwLock<HashMap>` vs async `RwLock<HashMap>` with broadcast senders) — same intentional sync/async mirror as `filter_data` / `filter_data_stream`.
- `log_unrouted(severity, &notice)` helper on each transport (refinement #6 prep) — PR 5 will add the broadcast call here.
- Removed `transport/common.rs::log_error_payload` and the entire `transport/common.rs` `log` import. `error_event` and `WARNING_CODES` were already absent in the pre-PR codebase.
- `Error::Message(code, msg)` reachability check resolved: still reachable — the dispatcher constructs it for hard errors with a real `request_id`. Kept.

**As-shipped — value type tightening:**
- `Notice` widened in-place with `advanced_order_reject_json: String` (`#[serde(default, skip_serializing_if = "String::is_empty")]`). Updated 13 existing literal constructions across messages tests, subscription tests, and `connection/common.rs`.
- New `ResponseMessage::advanced_order_reject_json()` accessor with `server_version >= ERROR_TIME` guard. `extract_text_error` in `routing.rs` migrated to use it (single source of truth).
- `SubscriptionItem::into_data` already shipped in PR 2b commit `857890e` (refinement #3) — no work in this PR.

**As-shipped — subscription wiring:**
- `InternalSubscription::{next_routed, try_next_routed, next_timeout_routed}` and `AsyncInternalSubscription::next_routed` expose the typed `RoutedItem` envelope. Legacy `next/try_next/next_timeout` retain `Result<ResponseMessage, Error>` shape via `into_legacy()` for direct consumers (`orders/async/mod.rs` etc.). `Subscription<T>::handle_response` (sync) and `Subscription<T>::next` (async) pattern-match `RoutedItem`, surfacing `Notice → Ok(SubscriptionItem::Notice)` and `Error → Err(_)`.
- Dropped `#[allow(dead_code)]` from `RoutedItem::Notice` — now reachable.

**As-shipped — tests:**
- `classify_error_delivery`: four combinations + boundary codes (2099, 2100, 2169, 2170).
- `notice_from_decoded`: rich-payload preservation + missing-optionals path.
- Sync and async dispatcher: owned warning, owned hard error (terminal), unrouted warning (no channel write), order-id fallback. Stream stays open after Notice (verified via follow-up Response).
- Sync and async `Subscription<T>`: end-to-end Notice surfaces as `SubscriptionItem::Notice` without terminating.
- No `dual_test!` macro experiment (refinement #5) — sync/async fixture divergence (crossbeam vs `tokio::broadcast`, sync vs async receiver) didn't justify the macro infrastructure.

**Scope deferrals:**
- **Per-T `Notice` enum variants left intact** (`PlaceOrder::Message(Notice)`, `Orders::Notice(Notice)`, `MarketDepths::Notice(Notice)`, `TickTypes::Notice(Notice)`, etc.). Dispatcher pre-classification makes the decoders' `IncomingMessages::Error` arms unreachable on the production path, but `MessageBusStub`-based tests still feed raw `RoutedItem::Response` messages through the decoder — those arms remain reachable in tests. Removing the public enum variants would touch ~10 examples and several test files; deferred to a follow-up PR.
- **README narrative + migration guide deferred to PR 6.** PR 3's doc-comments on `Subscription::next` already mention the `SubscriptionItem::Notice` arm with an inline doctest, but the cross-cutting README story ("how to observe vs. filter notices") waits until PR 5's global stream is also live so the docs cover both halves of the notice API at once.

**Dependencies:** PR 1 (real protobuf error_code), PR 2a (`RoutedItem` channel), PR 2b (`SubscriptionItem` public type). PR 2c was not strictly required (no `Stream` combinators in PR 3 tests).

---

### PR 4 — End-to-end Subscription tests for Notice delivery (in flight)
**Goal:** wire the dispatcher → subscription path under tests that actually drive `Subscription::next()`, plus an opt-in live-gateway smoke test for release verification. PR 3's tests prove dispatcher-level classification; this PR proves the full Subscription consumer path.

**As-shipped diff (local branch):** 14 files, +397/-30 LOC. fmt clean; clippy clean across default, sync-only, and `--all-features` with `-D warnings`; full test suite green (sync 924 lib + 119 doc, async 927 lib + 73 doc, all-features 1125 lib + 143 doc). 5 new sync e2e tests + 5 new async e2e tests, plus 6 live-gateway integration tests (3 sync, 3 async).

**Scope reductions vs. original plan** (decided 2026-05-04):
- **Synthesized e2e tests live in `src/transport/{sync,async}_tests.rs`, not `tests/notice_delivery.rs`.** The plan named `tests/notice_delivery.rs` but the tests need `MemoryStream` / `TcpMessageBus` / `SubscriptionBuilder` (all `pub(crate)`); putting them in `tests/` would have required widening visibility just for the test. Appended to the existing PR 3 dispatcher tests instead — same `make_bus()` / `body()` / `TICK` helpers, one logical unit.
- **Live-gateway tests live in `integration/{sync,async}/tests/notice_delivery.rs`**, not `tests/notice_delivery_integration.rs`. The repo already has a workspace pattern at `integration/` (`ibapi-test` shared helpers, ClientId pool, rate limiter, `serial_test` for shared-state ordering) — using it instead of `#[ignore]`'d top-level tests gets the rate limiter + client-id pooling for free and matches the rest of the integration suite.
- **No `dual_test!` macro experiment** (refinement #5) — already skipped in PR 3; left out of PR 4 for the same reason.

**Workspace-red fix folded in:** `integration/{sync,async}/tests/{realtime_data,contracts,scanners,accounts,orders}.rs` were never migrated to PR 2b's `Subscription::next() -> Option<Result<SubscriptionItem<T>, Error>>` shape — `cargo build -p ibapi-integration-{sync,async} --tests` was red on `main` before this PR. Mechanical fix: switched data-only callers to `next_data()` / `iter_data()` / `timeout_iter_data()`, kept the order-status loop's pattern-matching shape but layered an `Ok(_)` arm. Per the "fix workspace-red in scope" rule.

**As-shipped — sync e2e tests** (`src/transport/sync/tests.rs`):
- `test_subscription_notice_delivery_request_keyed`: code 2104 + req_id=42 → `Some(Ok(SubscriptionItem::Notice(_)))`; follow-up data arrives normally (stream stays open).
- `test_subscription_hard_error_terminates_stream`: code 200 + req_id=42 → `Some(Err(Error::Message))`, then `None`.
- `test_subscription_notice_delivery_order_keyed`: code 2109 + order_id=7 → notice delivered via order-channel fallback.
- `test_subscription_unspecified_notice_not_delivered`: code 2104 + req_id=-1 → `try_next()` returns `None` (no channel write).
- `test_subscription_iter_data_filters_notices`: `[Data, Notice, Data]` stream → `iter_data()` yields exactly the two data items.

**As-shipped — async e2e tests** (`src/transport/async_tests.rs`): structural mirror of the sync set, with `data_stream()` instead of `iter_data()` for the filter test.

**As-shipped — live-gateway tests** (`integration/{sync,async}/tests/notice_delivery.rs`):
- `market_data_surfaces_notice`: subscribes to AAPL with `generic_ticks(&["233"])`, drains up to 20 items asserting at least one `SubscriptionItem::Notice` (typically code 2104 farm-status).
- `invalid_contract_terminates_with_error`: subscribes to symbol `DOES_NOT_EXIST_XYZ` and asserts the subscription terminates with `Some(Err(_))`.
- `outside_rth_order_surfaces_notice`: places a non-transmit outside-RTH market order on AAPL, drains the subscription expecting a Notice (logs a non-fatal warning if the gateway suppresses it for that session). `#[serial(orders)]`.

**As-shipped — example update:** added a 4th `example_observe_notices` function to both `examples/sync/market_data.rs` and `examples/async/market_data.rs` that uses `iter()` / `next()` (full `SubscriptionItem`) instead of `iter_data()` / `next_data()`, with explicit pattern-matching on the `Data` and `Notice` arms. Comments call out which farm-status codes are common (2104/2107/2108).

**As-shipped — prelude:** added `pub use crate::subscriptions::SubscriptionItem;` to `src/prelude.rs` so `use ibapi::prelude::*;` makes the type discoverable for callers who pattern-match `next()`.

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
- `integration/sync/tests/conditional_orders.rs` and `integration/async/tests/conditional_orders.rs` go through the dispatcher; run against a live gateway to confirm no order-routing regressions.

**Scope — minimal docs touch:**
- Update at least one runnable example (`examples/sync/market_data.rs` and the async mirror are good candidates) to demonstrate the `SubscriptionItem::Notice` arm explicitly — current examples already match on `TickTypes::Notice` per-T variants, but with the dispatcher now emitting `SubscriptionItem::Notice` they should additionally show the `match sub.next() { Some(Ok(SubscriptionItem::Notice(n))) => ... }` form. Bigger README narrative + migration guide deferred to PR 6.

**Dependencies:** PR 1 + PR 2a + PR 2b + PR 3 all merged.

**Risk:** synthesized tests verify wiring, not real TWS packet shapes. Mitigation: PR 1's protobuf-decode unit test + PR 3's classification tests already cover packet-shape parsing; PR 4's synthesized tests cover end-to-end consumer behavior. The `#[ignore]`'d live tests are the safety net for protocol-level regressions and are expected to be hand-run before each release.

**Recordings deferred.** Capturing real TWS bytes for replay tests is not in scope for PR 4. If a future regression is missed by the synthesized tests but caught by the live tests, add recordings then — don't pre-build the fixture infrastructure.

---

### PR 5 — Global notice stream ✅ merged ([#512](https://github.com/wboayue/rust-ibapi/pull/512))

**As-shipped — niche power-user API.** Lands as `Client::notice_stream() -> Result<NoticeStream, Error>` (dedicated `NoticeStream`, option (b) from refinement #8 — see plan history in commit `b03319c`). Live testing during implementation revealed the API only sees notices that arrive *after* `process_messages` starts — which excludes the 2104/2106/2158 farm-status snapshot that fires during the handshake. Result: `notice_stream` is genuinely useful for runtime events (connectivity loss/restore, farm flips, auto-reconnect notices) but is not the right API for the canonical "show current connection state in a UI" need. **Will not be advertised in the README or getting-started guides** (PR 6 docs scope adjusted accordingly — see "PR 6 follow-up" below). Doc-comments on the API itself are kept so rustdoc users can still find it.

**Follow-up shipped on a separate branch (`startup-message-typed-callback`):** extends `ConnectionOptions` with a typed `StartupMessage` enum + `StartupNoticeCallback` so the canonical "react to handshake-time notices" use case has a first-class API that doesn't require subscribing to a live stream. That work supersedes the role PR 5 was originally intended to fill.

---

**Original goal:** expose IB's globally routed notices (codes with `request_id = -1`, e.g. `1100` lost connectivity, `2104` market-data farm OK) as a programmatic `Subscription<Notice>` so consumers can drive UI status indicators and reconnection logic, instead of scraping logs.

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

**Scope — example + inline docs:**
- Add `examples/notice_stream.rs` (sync) and `examples/notice_stream_async.rs` (async) showing a connection-status monitor: subscribe to `notice_stream()`, log/print as `1100`/`1102` arrive. Examples should pattern-match on `notice.code` for connectivity codes (1100/1101/1102) and farm-status codes (2104/2105/2106/2107/2108) at minimum.
- Comprehensive doc-comment on `Client::notice_stream()` — call out that this is for *globally routed* notices (no `request_id`), distinct from per-subscription notices that arrive on `Subscription<T>::next()` as `SubscriptionItem::Notice`. Cross-link both directions from the `notice_stream` doc and the `Subscription::next` doc so callers don't have to discover the distinction empirically.
- README narrative + migration guide deferred to PR 6 (which consolidates PR 3's per-subscription notice handling with PR 5's global stream into one user-facing story).

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

---

### PR 6 — Documentation: notification handling & v3.0 migration guide

**Scope adjusted post-PR-5:** when PR 6 ships, the README's "Handling notifications" section should treat `ConnectionOptions::startup_callback` (typed `StartupMessage`) + `ConnectionOptions::startup_notice_callback` (typed `Notice`) as the canonical API for connection-state UIs and handshake-time observability. `Client::notice_stream()` should *not* feature in the README or getting-started examples — at most one line cross-linking to its rustdoc for users who specifically want runtime-only unrouted-notice events. Reason: live testing of PR 5 proved `notice_stream` doesn't see the handshake snapshot (2104/2106/2158 farm-status), which is what the typical "show farm status" use case wants. The separate PR on `startup-message-typed-callback` filled that gap with an API that *does* see the snapshot.

---

**Goal:** make the user-facing story for notification handling discoverable and self-contained. By the time PR 5 lands, three things have changed for callers:
1. `Subscription<T>::next()` returns `Option<Result<SubscriptionItem<T>, Error>>` (PR 2b widening) — the wrapping `SubscriptionItem<T>` enum is new.
2. Per-subscription notices (warning codes 2100..=2169 with a real `request_id`) now arrive as `SubscriptionItem::Notice(_)` instead of being log-only (PR 3).
3. Globally routed notices (`request_id == -1`) are accessible programmatically via `Client::notice_stream()` (PR 5).

Without a top-level docs touch, callers have to discover this from a mix of changelog entries, doc-comments on `Subscription::next`, and reading `examples/`. PR 6 fixes that.

**Why a separate PR.** Keeping the docs work distinct from feature PRs has two benefits: (a) review focus — feature PRs stay reviewable for correctness without prose getting in the way; (b) the docs work needs the *whole shipped story* visible to write a coherent narrative — that's only true after PR 5 merges.

**Scope — README:**
- Add a "Handling notifications" section to `README.md` with three subsections:
  - **Per-subscription notices**: the `SubscriptionItem::Data(t) | SubscriptionItem::Notice(n)` pattern, with a short sync + async code sample. Show both `iter_data()` (filter notices) and `iter()` (observe notices) shapes side by side.
  - **Filtering vs. observing**: when to use `iter_data()` / `next_data()` (most call sites) vs. `iter()` / `next()` pattern-matching (UI status indicators, custom logging).
  - **Global notice stream**: `Client::notice_stream()` for connectivity (1100/1101/1102) and farm-status (2104/2105/2106) notices. Code sample showing a connection-status monitor.
- Each code sample compiles as a doc-test (`cargo test --doc`) so it doesn't bit-rot.

**Scope — migration guide:**
- New section in `docs/` (e.g. `docs/migration-3.0.md`) or expand the existing v3.0 release notes to cover the breaking changes for notification handling:
  - Sync `Subscription::next()` shape change (`Option<T>` + separate `error()` accessor → `Option<Result<SubscriptionItem<T>, Error>>`). The mechanical migration is `iter_data()` for callers that don't care about notices.
  - Async symmetry: `SubscriptionItem<T>` wrapping `T`.
  - Removed APIs: `Subscription::error()`, `Subscription::clear_error()`, the per-T `Notice` enum variants if/when those get retired (cross-link to follow-up PR).
- Provide a "before / after" code snippet for the three most common subscription patterns: `market_data`, `place_order`, `account_summary`.

**Scope — doc-comment hardening:**
- Audit doc-comments on `Subscription<T>::next`, `next_data`, `iter_data`, `iter`, `try_iter`, `timeout_iter`, async `data_stream`, `stream` and confirm each one explicitly mentions the `Notice` arm and links to the relevant alternative (data-only iterator vs. full iterator).
- `Client::notice_stream` doc cross-links to `Subscription::next` and vice versa (already partially done in PR 5; PR 6 completes the matrix).

**Scope — examples sweep:**
- Confirm that all examples updated by PR 2b's mechanical sweep still demonstrate idiomatic notice handling. Promote any that still use `next_data()` to `iter()` if the example's purpose is to show user-facing observability (e.g. market-data and order-status examples).
- Each updated example gets a 1-2 line header comment explaining what notice behavior it demonstrates ("filters notices via `iter_data()`" vs. "shows both data and notices via `iter()`").

**Tests:**
- Doc-test compilation (`cargo test --doc`) covers the README + migration code samples.
- No new unit tests — this is a docs PR.

**Dependencies:** PR 3 + PR 5 merged. Optional dependency on PR 4 (live-gateway behavior verified before promising it to users).

**Risk:** doc rot. Mitigation: every code block in README/migration is a doc-test, not a free-form snippet; `just test` catches silent breakage.

**Out of scope:**
- Tutorial-style "getting started with subscriptions" content — belongs in `docs/quick-start.md`, not the notice-handling docs.
- Architecture-level write-up of the dispatcher → subscription envelope (`RoutedItem`, `ErrorDelivery`) — internal-only, not in user-facing docs. If wanted, lives in `docs/architecture.md` as a separate task.

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

User-facing docs (PR 6):
- `README.md` — new "Handling notifications" section: per-subscription notices via `SubscriptionItem::Notice`, filter-vs-observe guidance, `Client::notice_stream()` for global notices. Each code block compiles as a doc-test.
- `docs/migration-3.0.md` (or extend the v3.0 release notes) — before/after snippets for the three breaking changes: sync `Subscription::next` shape, `Subscription::error()`/`clear_error` removal, `SubscriptionItem<T>` wrapping `T` on both sync/async.
- Doc-comment audit on `Subscription<T>::next` / `next_data` / `iter_data` / `iter` / `try_iter` / `timeout_iter` (sync) and `next` / `next_data` / `data_stream` / `stream` (async) — confirm each names the `Notice` arm and cross-links the data-only vs. full-iterator alternatives.
- `Client::notice_stream` ↔ `Subscription::next` cross-link — establish in PR 5, finalize in PR 6.
