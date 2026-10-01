# PR 2: `buffer_limit` + `cancel_and_drain` (#876 parts 3–4)

Parts 3 and 4 of [contract-details-stream](contract-details-stream.md). PR 1
(#878) shipped the builder, decoder, `collect_to_end` and the bond fix.
Additive and 4.x-safe.

- **Part A**: fail-closed `buffer_limit`, bounded memory for a stalled reader.
- **Part B**: `Subscription::cancel_and_drain(deadline)`, confirmed cleanup.

The two are independent features but meet in one place: a drain after an
overflow (see "Interplay").

# Part A: `buffer_limit`

## Goal

Bound the memory a stalled reader can pin, without silently losing rows.

Today:

- **Async** request channels are `broadcast` with the global
  `channel_capacity` (default `BROADCAST_CHANNEL_CAPACITY` = 1024): bounded,
  but overflow evicts the oldest item and surfaces only a non-terminal `-6` lag
  notice (#779 step 1). A slow enumeration reader gets a silently partial list
  unless it inspects notices.
- **Sync** request channels are `crossbeam::unbounded`, with watermark warnings
  only. A stalled reader grows without bound.

`buffer_limit(n)` adds a third channel class, *enumeration*: bounded, lossless,
and fail-closed. Market data stays lossy, and orders stay as they are.

## API

```rust
let subscription = client
    .contract_details_stream(&contract)
    .buffer_limit(512)      // max unread rows before the stream fails
    .subscribe()?;
```

- `ContractDetailsBuilder::buffer_limit(mut self, n: usize) -> Self`.
  Infallible, like other builder setters; `subscribe()` validates.
- **Optional.** Unset means today's behaviour exactly (async 1024 lossy, sync
  unbounded).
- **Valid range** `1..=MAX_BUFFER_LIMIT`. Anything else makes `subscribe()`
  return `InvalidArgument` before anything is written. The cap exists because
  tokio's `broadcast::channel` allocates its slots up front, rounded to a power
  of two: a huge limit is a huge allocation, and `capacity > usize::MAX / 2`
  panics. Proposal: `MAX_BUFFER_LIMIT = 65_536`, a `pub const` next to the
  builder, documented.
- **Overflow is terminal.** When `n` items sit unread and another frame
  arrives for the request, the dispatcher delivers
  `Err(Error::BufferLimitExceeded { limit: n })` after the queued items and
  queues nothing more for that id. The reader gets every queued row, in order,
  then the error, then `None`. Nothing is dropped silently.
- **The dispatcher never blocks:** the check is a length comparison.
- **The limit counts unread items, not total ones.** A reader that keeps up
  never trips it, however many rows arrive.
- **After overflow, dropping the subscription sends the native cancel.** The
  error doesn't set `ended_natively`. Frames still arriving for the id are
  discarded quietly (see "Overflowed routes").

`Error::BufferLimitExceeded { limit: usize }` is a new variant (`Error` is
`#[non_exhaustive]`), with an arm in the manual `impl Clone for Error`. Display:
`"subscription buffer limit exceeded ({limit} unread items)"`. Typed on main;
no v2-stable port ([[feedback_per_branch_error_variant]]).

## Mechanism

### Plumbing the limit

- **Bus traits.** Add a method to both `MessageBus` (sync) and
  `AsyncMessageBus`:
  ```rust
  fn send_request_bounded(&self, request_id: i32, message: &[u8], limit: usize) -> Result<InternalSubscription, Error>;
  ```
  (async: `Vec<u8>`, `async`). Keep `send_request` as is rather than adding an
  `Option` parameter to its ~40 callers. The two share a private body on each
  bus, parameterised by the route's bound.
- **`SubscriptionBuilder`** (`client/builders/{sync,async}.rs`): add
  `send_with_request_id_bounded(self, request_id, message, limit)` next to
  `send_with_request_id`, building the subscription the same way.
- **`contracts::{sync,async}::contract_details_stream`** gains a
  `buffer_limit: Option<usize>` parameter (it's builder-fed, so it's under the
  param-budget exception, [[feedback_rule19_builder_fed_helpers_exception]]).
  It validates the range, then picks the bounded or the plain send.
- **`MessageBusStub`** implements both new methods by recording the request
  like `send_request` and ignoring the limit. Overflow is tested on the real
  buses (below), not on the stub.

### Sync (`transport/sync.rs`)

`requests` is a `SenderHash<i32, RoutedItem>`, the generic type also used for
orders and executions. Store the bound next to the sender, so request routing
stays a single lookup:

```rust
struct Entry<V> { sender: Sender<V>, bound: Option<Bound> }
struct Bound { limit: usize, overflowed: AtomicBool }
```

- `SenderHash::insert` stays as is (no bound). Add
  `insert_bounded(id, sender, limit)`; only `send_request_bounded` calls it.
- The request delivery paths (`process_response_with_id`,
  `deliver_to_request_id`) go through a new
  `SenderHash::send_bounded(&id, item, overflow: impl FnOnce(usize) -> V)`:
  - `bound` is `None`: send as today.
  - `overflowed` is set: drop the item at `trace!`.
  - `sender.len() >= limit`: set `overflowed`, then send `overflow(limit)` (the
    channel is unbounded, so it always fits).
  - otherwise: send.
- Orders and executions never set a bound, so their paths don't change.
- `notify_all` (reset or shutdown) skips overflowed entries, because the
  stream has already ended in an error.

### Async (`transport/async.rs`)

> **As built:** `broadcast::Sender::len()` can't measure unread items: every
> `AsyncInternalSubscription` holds a `template_receiver` (for `Clone`) that
> never reads, so `len()` only grows. The bounded route instead counts what it
> sent, and the original subscription counts what it read (a shared
> `AtomicUsize`); unread is the difference. Clones aren't counted.

`request_channels: HashMap<i32, BroadcastSender>` becomes
`HashMap<i32, RequestRoute>` with `RequestRoute { sender, bound: Option<Bound> }`.

- **The channel** is created with capacity `limit + 1`. The extra slot is
  reserved for the terminal error, so delivering it never evicts a row (and so
  never emits a `-6` lag notice).
- **`route_to_request_channel` / `deliver_to_request_id`**: the same logic as
  sync, using `broadcast::Sender::len()`. That counts values not yet seen by
  every live receiver, so with clones the slowest clone governs the limit;
  document that.
- **The dispatcher is the only producer** for request channels, so
  check-then-send can't race.
- **`reset_channels` / shutdown** skip overflowed routes. For a route at
  exactly `limit` (not overflowed), the reset error takes the reserved slot.
- **Cleanup**: `remove_if_unreferenced` and the cleanup signal key on `id` and
  the sender. Check that they compile against `RequestRoute` unchanged
  semantically.

### Overflowed routes

The route stays registered after overflow, and further frames are dropped at
`trace!`. Removing it would send every late frame to `log_orphan` and the
unrouted-frame warning, which for a broad query is hundreds of lines.
Cleanup is unchanged: dropping the subscription removes the route and sends
the cancel, as for any errored stream.

### Channel classes (#779 step 2)

Add *enumeration (bounded, lossless, fail-closed, opt-in per request)* as a
third class in `plans/broadcast-lag-visibility.md`, next to order-class and
market-data-class.

## Tests

Overflow uses the real buses: `MemoryStream` + `make_bus()` on async,
`TcpMessageBus` over its memory stream on sync, as the existing
`transport/{sync,async}_tests.rs` routing tests do
([[feedback_lightest_test_fixture]]). Builder and validation tests use
`MessageBusStub`.

| Test | Asserts |
| --- | --- |
| overflow (sync + async) | limit 2, 3 `ContractData` frames routed, nothing read → 2 rows, `BufferLimitExceeded { limit: 2 }`, then `None` |
| keeps up (sync + async) | limit 2, 6 frames each read before the next → 6 rows + end, no error |
| late frames dropped | after overflow, 5 more frames + `ContractDataEnd` → no orphan log path; still error then `None` |
| cancel after overflow | drop after overflow → `cancelContractData` written (server ≥ 215) |
| no eviction (async) | at overflow no `-6` lag notice is emitted; the first row is still the first frame sent |
| reset after overflow | reset → overflowed route gets no second error; a non-overflowed bounded route gets `ConnectionReset` without eviction |
| unbounded unchanged | `send_request` route with 3 frames behaves as before (regression) |
| validation (stub) | `buffer_limit(0)` and `MAX_BUFFER_LIMIT + 1` → `InvalidArgument`, 0 requests written |
| plumbing (stub) | `buffer_limit(n)` reaches `send_request_bounded` with `n`; unset uses `send_request` |
| `Error` | `BufferLimitExceeded` Display + Clone |

Integration (`integration/{sync,async}/tests/contracts.rs`), live:

- SPY calls, one month two months out (`yyyymm_months_from_now(2)`):
  `buffer_limit(5)`, `subscribe`, sleep ~2s without reading, then read. Expect
  5 rows, then `BufferLimitExceeded { limit: 5 }`. Afterwards a small AAPL
  query on the same client succeeds.

## Docs

- Rustdoc on `buffer_limit`: semantics, the range, the clone caveat (async),
  and when to use it: a reader that may stall (batch writes, back-pressured
  downstream).
- Mention `buffer_limit` in `ContractDetailsBuilder`'s type doc and in the
  sync/async `subscribe` examples? No: keep the examples minimal and add one
  sync+async pair on `buffer_limit` itself
  ([[feedback_per_method_sync_async_doc_pairing]]).
- CHANGELOG Added: `ContractDetailsBuilder::buffer_limit`,
  `Error::BufferLimitExceeded`, `MAX_BUFFER_LIMIT`.
- Update the main plan's part 3 section with what shipped; update
  `broadcast-lag-visibility.md`.

# Part B: `cancel_and_drain`

## Goal

Tell a caller when TWS has finished with a request, so it can reuse a request
slot or keep strict pacing. Drop's cancel is fire-and-forget and can't tell
it that.

## What TWS does after a cancel (live, server 225, 2026-09-30)

- **Prepared result** (SPY calls for one month, 714 rows): after reading 3
  rows and sending `cancelContractData`, TWS still sent the other 711 rows,
  then `ContractDataEnd`, in the same ~1.1s as without a cancel.
- **Incremental result** (all SPY options): **the cancel doesn't stop it
  either.** Live check, 2026-09-30: 100 rows read (8.4s), cancel, then 8,316
  more rows over 150s with no end marker before the deadline. The gateway
  stayed healthy. Docs now say TWS keeps sending after the cancel, and that a
  drain should be sized for the whole result.

## API

On both `Subscription` types, generic over `T: StreamDecoder<T>`:

```rust
// sync
pub fn cancel_and_drain(self, deadline: std::time::Instant) -> Result<Drained, Error>;
// async
pub async fn cancel_and_drain(self, deadline: tokio::time::Instant) -> Result<Drained, Error>;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Drained {
    /// TWS sent the end marker: the request is over at TWS.
    Ended,
    /// TWS sent an error for this request id: the request is over at TWS.
    Rejected(Notice),
    /// No terminal evidence: the deadline passed, or the stream had already
    /// failed locally (buffer overflow, decode error) so TWS's state can't be
    /// observed. Treat the id as possibly still live.
    Unconfirmed,
}
```

`#[non_exhaustive]` is deliberate ([[feedback_non_exhaustive_caseby_case]]).
`Unconfirmed` may later split by reason (deadline vs local failure) without
breaking callers. `Drained` lives in `subscriptions` and is re-exported like
`SubscriptionItem`.

## Semantics

1. **Already ended natively** (`ended_natively`): `Ok(Ended)`, nothing written.
2. **Already ended with an error** (as built: always `Ok(Unconfirmed)`, since
   the subscription doesn't keep the error it already returned; `Drop` writes
   the cancel). The original sketch:
   - Session errors (`ConnectionReset`, `Shutdown`): `Err`, the same error.
     The request died with the session.
   - Anything else (overflow, decode error, TWS error): write the cancel as
     `Drop` would and return `Ok(Unconfirmed)`. A TWS error that was already
     *read* by the caller is not remembered by the subscription, so it isn't
     reported as `Rejected` here. Callers who saw it already know.
3. **Live stream:**
   - Write the decoder's `cancel_message` if it has one. Mark the subscription
     `cancelled`, so `Drop` doesn't write it again. No cancel message (server
     < 215, or a decoder without one) writes nothing; the drain then waits for
     a natural end.
   - Read and discard until the end marker (`Ended`), a TWS error for the id
     (`Rejected`), a session error (`Err`), or the deadline (`Unconfirmed`).
     Data items and notices are discarded.
4. **It consumes `self`**, so nothing can read after a drain. The route is
   removed when the subscription drops at the end of the call, as for any
   drop.
5. **Async cancellation is safe.** Dropping the future mid-drain drops the
   subscription. The cancel is already written and `cancelled` blocks a
   second one. The session stays up.
6. **Streams without an end marker** (market data) return `Unconfirmed` at the
   deadline. Documented, not restricted.

## Mechanism

- **Write the cancel without unregistering the route.** The existing paths
  differ by transport:
  - Sync `cancel()` calls `MessageBus::cancel_subscription`, which writes,
    pushes `Error::Cancelled` into the channel, and **removes the route**
    (`transport/sync.rs:852`). A drain through it would see `Cancelled`, then
    nothing; it could never observe TWS's end.
  - Async `cancel()` writes via `send_message` and leaves the route.

  So `cancel_and_drain` writes the cancel with `send_message` on both
  transports. It sets `cancelled` through the same atomic swap as `cancel()`
  and `Drop`, and leaves route removal to `InternalSubscription`'s `Drop`
  signal (`transport/mod.rs:291`). Extract the "which cancel to write" choice
  shared by `cancel()`, `Drop` and the drain into one private helper per side
  (the async `pending_cancel` already does this; add the sync twin) rather
  than forking it.
- **Reading.** Sync: loop on `next_timeout(deadline - now)`. Async:
  `tokio::time::timeout_at(deadline, self.next())` in a loop.
  `SubscriptionItem::Data`/`Notice` are discarded. `None` with
  `ended_natively()` is `Ended`. `Some(Err(Error::Notice(n)))` is
  `Rejected(n)`. `Some(Err(ConnectionReset | Shutdown))` is `Err`. Any other
  `Err`, or `None` without the end marker, is `Unconfirmed`.
- **Async clones.** One clone's drain writes the shared cancel and flips
  `cancelled` for all clones. Other clones keep reading their own receivers
  and see the end or the error. Pin this in a test.

## Interplay with `buffer_limit`

After an overflow the route drops every later frame, including the end marker,
so a drain can't see TWS finish. By semantics 2, `cancel_and_drain` after an
overflow writes the cancel and returns `Unconfirmed` straight away; it doesn't
wait out the deadline for evidence that can't arrive.

Possible follow-up, not in this PR: let overflowed routes still forward
terminal frames. That needs the end-marker type in `Bound` (the transport
doesn't know it) and capacity `limit + 2` on async. Add it only if a caller
needs `Ended` after an overflow.

## Live check (first step of Part B)

One temporary integration test, deleted before merge, with a `server_time`
check after it. Unfiltered SPY options: read ~100 rows, `cancel_and_drain`
with a 60s deadline, and log the outcome, the rows discarded and the time. Two
possible results:

- **`Ended` quickly with few discarded rows:** the cancel stops incremental
  work. Document "cancel stops an in-progress enumeration; a prepared one is
  delivered in full".
- **`Ended` after the full ~113s, or `Unconfirmed`:** the cancel doesn't stop
  it. Document that, and recommend narrow queries.

Record the result in the main plan.

## Tests (Part B)

`MessageBusStub` for sequencing (it records writes); the real buses where route
lifetime matters.

| Test | Asserts |
| --- | --- |
| already ended | rows + end read, then drain → `Ended`, 0 cancels written |
| live → end | 2 rows queued, end marker after → cancel written once, `Ended` |
| live → TWS error | error 200 for the id after the cancel → `Rejected(notice)` with code 200 |
| deadline | nothing after the cancel → `Unconfirmed` at the deadline (short deadline); cancel written once, not again on drop |
| old server | server < 215 → nothing written; scripted natural end → `Ended` |
| already failed | decode error read first, then drain → cancel written, `Unconfirmed` immediately |
| after overflow | bounded route overflowed, then drain → `Unconfirmed` immediately, cancel written (real bus) |
| session error | `ConnectionReset` during drain → `Err(ConnectionReset)` |
| route survives the cancel (sync, real bus) | after the drain writes the cancel, a routed `ContractDataEnd` still reaches it (`Ended`); the route is gone after return |
| async future dropped | drain future dropped mid-wait → exactly one cancel written |
| async clones | clone A drains; clone B still receives the end marker |

Integration (live, both clients): SPY calls one month out (prepared result),
read 3 rows, then `cancel_and_drain(now + 30s)` → `Ended`.

## Docs (Part B)

- Rustdoc on `cancel_and_drain` (sync + async paired) with the outcome table
  and the live finding, and on `Drained`.
- CHANGELOG Added: `Subscription::cancel_and_drain`, `Drained`.
- Main plan: mark part 4 shipped; record the live check.

## Commit sequence

Part A:

1. `Error::BufferLimitExceeded`.
2. Sync bus: `Entry`/`Bound`, `send_request_bounded`, overflow tests.
3. Async bus: `RequestRoute`, `send_request_bounded`, overflow tests.
4. Builder plumbing + `buffer_limit` + validation, stub tests.

Part B:

5. Live check (temporary test, not committed); record the result.
6. Shared cancel-choice helper (sync twin of `pending_cancel`), `Drained`,
   `cancel_and_drain` on both sides, unit tests.

Both:

7. Docs, CHANGELOG, integration tests, plan updates.

## Open questions

- **Split into two PRs?** Parts A and B touch different layers (transport vs
  `Subscription`) and only meet at "Interplay". One PR is about 400 + 550
  lines. If that's too big to review, ship A first; B's "after overflow" test
  then lands with B.
- **`MAX_BUFFER_LIMIT` value.** 65,536 rows of contract details is roughly
  100–300 MB if fully queued, which arguably defeats the point. The cap
  guards the async allocation, not memory policy. Alternatively derive it from
  the global `channel_capacity`. Proposal: keep 65,536 and let callers pick
  small numbers.
- **Expose on other builders now?** `option_chain` is the obvious second user,
  but no caller has asked ([[feedback_no_speculative_test_infra]]).
  Proposal: not in this PR; it's one setter when someone asks.
- **Should Vec `contract_details` set a limit?** No: it reads as fast as
  frames arrive, and adding a limit could turn a slow consumer machine into a
  spurious error. Leave it unbounded.

## Size

| Part | Source | Tests |
| --- | --- | --- |
| A: `buffer_limit` | ~200 | ~300 |
| B: `cancel_and_drain` | ~150 | ~250 |
| Total | ~350 | ~550 |
