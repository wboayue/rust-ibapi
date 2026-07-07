# Fix #694 — request-less errors hang shared-channel one-shot callers

> On execution, copy this plan to `plans/694-shared-oneshot-error-delivery.md` in the repo (project convention), branch off `main`, then implement.

## Context

When TWS answers a **shared-channel** request with an error frame carrying no request id
(`request_id == -1` / `UNSPECIFIED_REQUEST_ID`), both transports log it and forward it to the
`NoticeStream`, but never deliver it to the awaiting shared-channel subscription. The one-shot
caller (`next_valid_order_id`, `request_fa`, `market_rule`, `managed_accounts`, …) hangs forever.
Reproduced live under `READ_ONLY_API=yes` (error 321) and unknown market rule (error 322).

Root cause — the `UNSPECIFIED_REQUEST_ID` arm sends to Notice only:
- async `src/transport/async.rs:472-475` (`route_error_message`)
- sync `src/transport/sync.rs:322-325` (`dispatch_message`)

Shared channels are keyed by **message type**, not request id, and the error frame has no
correlation key — so precise routing is impossible. We fail-fast the **one-shot** shared channels
(single request → terminating response) while leaving **streaming** shared channels untouched, so
an unrelated error never terminates a live positions/orders/account stream.

Accepted, bounded imprecision (same as issue's fallback, minus stream-kill risk): concurrent
one-shots of different types fail together on one error, and a genuinely global hard error can
spuriously fail an in-flight one-shot. Both are strictly better than an infinite hang.

Scope: **`main` only** (async shared routing is v3.x; per current maintenance policy). No wire/proto
change. `ChannelMapping` is in a `pub(crate) mod` → no public API break.

## Design rationale (duplication / SRP / composability)

- **One source of truth for the fail-fast set.** `one_shot_error_response_types()` is computed once
  (a `LazyLock` set-difference) in `shared_channel_configuration.rs` and consumed by both transports.
  No per-transport copy of "which channels are one-shot."
- **Explicit `one_shot` data, not derived from `*End`.** Deriving "streaming" from "responses contains
  an `*End` variant" would *misclassify* `NewsBulletins` and `WshEventData`, which stream without an
  End marker. The field carries a real semantic property that isn't recoverable from the response list,
  so it earns its place (not redundant).
- **Two transport helpers are acceptable mirror duplication (rule 13).** `fail_one_shot_shared_channels`
  differs by async-ness and lock/sender type (tokio `broadcast` under `RwLock` vs sync sender under the
  blocking guard) — the same shape as the existing mirrored `deliver_to_request_id` /
  `route_to_shared_channel` fan-out. Both sides carry real behavior; a shared wrapper would be a no-op
  delegate. The *decision data* is shared; only the delivery mechanics mirror.
- **SRP for the arm.** The `UNSPECIFIED_REQUEST_ID` branch stays readable by delegating fail-fast to a
  named helper rather than inlining a nested loop next to the log + Notice send.
- **`next_valid_order_id` cannot reuse `one_shot_request`.** That helper maps `None → Ok(default())`;
  `next_valid_order_id` needs `None → Err(UnexpectedEndOfStream)` and has no default, so it stays
  hand-rolled — but its `match` is normalized to the exact shape its sibling one-shot consumers
  (`news_providers`, `scanner_parameters`, `market_rule`) already use.
- **Persistent-receiver harmlessness.** Broadcasting an `Error` to a one-shot channel with no in-flight
  request lands only in the stored receiver held for `resubscribe()`; tokio `resubscribe()` sees only
  post-subscribe messages, so a future one-shot never observes a stale error. No cleanup needed.

## Verified facts (from exploration)

- `DecodedError` derives `Clone`; `IncomingMessages` derives `Copy + Hash + Eq`. `LazyLock` is an
  established project idiom (`src/lib.rs:324`, `src/stubs.rs:45`).
- `RoutedItem::Error(e)` → `Some(Err(e))` at every read layer via `RoutedItem::into_legacy`
  (`src/subscriptions/common.rs:75`). A one-shot reader takes the first item and drops the sub.
  A streaming sub sets `stream_ended` and terminates on any `Error` != `EndOfStream`
  (`src/subscriptions/{sync,async}.rs`) — hence streaming channels must be excluded.
- Shared senders map: `shared_channel_senders: HashMap<IncomingMessages, Vec<BroadcastSender>>`,
  built from `CHANNEL_MAPPINGS` at bus construction (async `async.rs:245`, sync `sync.rs:49`).
- Sync error-masking sites (drop `Some(Err(e))`): blocking `one_shot_request`
  (`src/common/request_helpers.rs:70`) and hand-rolled `next_valid_order_id`
  (`src/orders/sync.rs:198-207`). All other one-shot sync consumers already do `Some(Err(e)) => Err(e)`.

## Changes

### 1. Classify one-shot vs streaming shared channels
`src/messages/shared_channel_configuration.rs`
- Add `pub one_shot: bool` to `ChannelMapping`; set it on all 23 `CHANNEL_MAPPINGS` entries
  (named-field construction — Rust errors on any omission; no other consumer breaks).
- `one_shot: true` for genuine single-response one-shots: `RequestIds→NextValidId`,
  `RequestFamilyCodes→FamilyCodes`, `RequestMarketRule→MarketRule`,
  `RequestManagedAccounts→ManagedAccounts`, `RequestMarketDataType`, `RequestMktDepthExchanges`,
  `RequestCurrentTime`, `RequestCurrentTimeInMillis`, `RequestNewsProviders`,
  `RequestScannerParameters`, `RequestWshMetaData`, `RequestFA→ReceiveFA`, `VerifyRequest`,
  `VerifyMessage`.
- `one_shot: false` for streaming: the `*End`-bearing mappings (`RequestPositions`,
  `RequestPositionsMulti`, `RequestOpenOrders`, `RequestAllOpenOrders`, `RequestAutoOpenOrders`,
  `RequestCompletedOrders`, `RequestAccountData`) **and** the two semantically-streaming ones
  without an `*End` marker: `RequestNewsBulletins→NewsBulletins` and `RequestWshEventData→WshEventData`.
  (Verify these two against C# `EClient`/behaviour — both are ongoing subscriptions, cancelable.)
- Add a precomputed set of response types that belong **exclusively** to one-shot mappings
  (set-difference guards against any type shared with a streaming mapping):
  ```rust
  static ONE_SHOT_ERROR_RESPONSE_TYPES: LazyLock<HashSet<IncomingMessages>> = LazyLock::new(|| {
      let streaming: HashSet<_> = CHANNEL_MAPPINGS.iter().filter(|m| !m.one_shot)
          .flat_map(|m| m.responses.iter().copied()).collect();
      CHANNEL_MAPPINGS.iter().filter(|m| m.one_shot)
          .flat_map(|m| m.responses.iter().copied())
          .filter(|r| !streaming.contains(r)).collect()
  });
  pub(crate) fn one_shot_error_response_types() -> &'static HashSet<IncomingMessages> {
      &ONE_SHOT_ERROR_RESPONSE_TYPES
  }
  ```

### 2. Deliver request-less hard errors to in-flight one-shot channels (both transports)
In the `request_id == UNSPECIFIED_REQUEST_ID` arm, keep the existing log + NoticeStream send, then
— **only when `!is_warning`** (warnings stay Notice-only) — fan a fresh `RoutedItem::Error` out to
every sender registered for each `one_shot_error_response_types()` message type. `payload`
(`DecodedError`) is `Clone`, so build the `Notice` from `payload.clone()` and reuse `payload` for the
error path; construct `Error::from(payload.clone())` per send (avoids requiring `Error: Clone`).

- async `src/transport/async.rs` `route_error_message` — add `self.fail_one_shot_shared_channels(&payload).await;`
  and a private helper reading `shared_channel_senders.read().await`.
- sync `src/transport/sync.rs` `dispatch_message` `UNSPECIFIED_REQUEST_ID` branch — mirror via
  `SharedChannels::notify_one_shot` (per "sync drop tax = two sister bugs" — fix both sides together).

**Implementation deviation (sync-only): drain-before-write.** The sync side shares one crossbeam
queue per request type (`SharedChannels`), unlike async tokio-broadcast whose `resubscribe` starts
fresh. A request-less error fanned out to a one-shot channel while no request is in flight would
buffer and poison the *next* one-shot call. Fix: `send_shared_request` drains the shared receiver
(`while shared_receiver.try_recv().is_ok() {}`) before writing, giving sync the same "fresh per
request" semantics as async. Async needs no drain.

### 3. Surface the real error on sync one-shot consumers (currently masked)
- `src/common/request_helpers.rs` blocking `one_shot_request` (`:70`): replace
  `if let Some(Ok(m)) = sub.next() { .. } else { Err(UnexpectedEndOfStream) }` with a `match` that
  adds `Some(Err(e)) => Err(e)` — matches its own retry siblings and the async version.
- `src/orders/sync.rs` `next_valid_order_id` (`:198-207`): same `match` conversion so the TWS error
  (e.g. `[321] Read-Only mode`) reaches the caller instead of `UnexpectedEndOfStream`.
- (async consumers already propagate correctly — no change.)

### 4. Changelog
`CHANGELOG.md` under `## [Unreleased] → Fixed`: request-less TWS errors on shared one-shot requests
(e.g. read-only-mode 321, unknown market rule 322) now fail the awaiting call fast with the real
error instead of hanging; streaming shared subscriptions are unaffected (#694).

## Tests

Follow the existing MemoryStream routing-test pattern (`make_bus()` + `stream.push_inbound(error_frame(-1, code, msg))`
+ `bus.dispatch()` / `bus.read_and_route_message().await`; helpers in `src/transport/{sync,async}_tests.rs`
and `src/common/test_utils.rs` — `error_frame`, `next_message`, `next_routed`, `try_next_routed`).

Add mirrored sync + async tests:
1. **Fail-fast + isolation**: open a one-shot shared sub (`send_shared_request(RequestIds, …)`) and a
   streaming shared sub (`send_shared_request(RequestPositions, …)`); push `error_frame(-1, 321, "…Read-Only…")`;
   dispatch. Assert the one-shot sub yields `Some(Err(..))` with the real message/code, and the
   streaming sub receives nothing (`try_next_routed().is_none()` — not terminated).
2. **Warning is Notice-only**: `error_frame(-1, 2104, FARM_OK_MSG)` does **not** deliver to the
   one-shot sub (extends existing `test_warning_with_unspecified_id_is_log_only`).
3. **NoticeStream preserved**: the request-less hard error still reaches `notice_subscribe()`.
4. Unit test `one_shot_error_response_types()` in `messages/tests.rs`: contains `NextValidId`,
   `MarketRule`, `ManagedAccounts`, `FamilyCodes`; excludes `OpenOrder`, `OrderStatus`, `Position`,
   `PositionMulti`, `AccountValue`, `NewsBulletins`, `WshEventData`.
5. Consumer read-path (covers the `Some(Err) => Err` arms in `next_valid_order_id` and the
   `one_shot_request` helper): the transport fail-fast tests read the one-shot subscription via the
   **legacy `next()` projection** (`sub.next_timeout(..)` / `sub.next().await`) — the exact path
   those consumers use — and assert `Some(Err(Notice{321}))` surfaces. **Deviation:** the plan's
   Client-level test was dropped rather than extend `MessageBusStub` with error-injection, which
   would have broken 68 struct-literal construction sites (see "keep test helpers minimal").

## Verification (execution phase)

```bash
cargo fmt
cargo test --lib transport:: request_helpers:: messages::
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
just test
# routing touches Subscription-adjacent surface — cheap insurance:
cargo build -p ibapi-integration-sync --tests && cargo build -p ibapi-integration-async --tests
```
End-to-end (optional, live gateway with `READ_ONLY_API=yes`): `next_valid_order_id().await` and the
sync equivalent should now return the `[321]` error promptly instead of hanging.

## Out of scope
- Precise per-request correlation of request-less errors (no wire key exists).
- Broad sweep of unrelated `UnexpectedEndOfStream` sites — only the two one-shot-masking sites are touched.
- `v2-stable` backport.
