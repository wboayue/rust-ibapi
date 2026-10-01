# Contract details as a stream (#876)

Counter-proposal to PR #877. It meets the issue's goals with a request builder
and the existing `Subscription` types instead of a parallel query API.

## Goals (from #876)

| Need | Met by |
| --- | --- |
| Request id before any I/O | builder allocates the id when it's built; `.request_id()` before `.subscribe()` |
| A way to stop | drop / `cancel()` → native `cancelContractData` (server 215+) |
| Local row bound | `.take(n)` on the iterator / stream, then drop |
| Bounded memory for a stalled reader | `.buffer_limit(n)`: fails closed on overflow (part 3) |
| Confirmed cleanup within a deadline | `Subscription::cancel_and_drain(deadline)` (part 4) |
| Know how the request ended | `None` = native end; `Err(..)` = error / notice / reset |
| No hidden retry | `.subscribe()` sends once; `matching_symbols` retry documented (part 2) |

Parts 1–2 ship first ([PR 1 plan](contract-details-stream-pr1.md)). Parts 3–4 are independent follow-up PRs, each a generic
`Subscription`/transport feature that contract details is the first user of.

## Non-goals

Out of scope; each would be a separate issue if someone shows a need:

- A caller-supplied request id (`.request_id(id)` setter). It needs a
  duplicate-id check in `send` (`AlreadySubscribed`) that today's map insert
  lacks.
- Byte/frame/decode budgets. The existing 16 MiB frame ceiling stays the only
  per-frame cap.
- A lossy (drop-oldest) per-subscription capacity. That's market-data
  territory and belongs to #779 step 2 (`plans/broadcast-lag-visibility.md`).
  Part 3 adds only fail-closed.
- Cancel-safe writes (a cancelled async `write_all` leaving a partial frame).
  This affects every request, so it needs a transport fix of its own.
- Retiring the session on drop / uncertain write.

## Part 1: `contract_details_stream` builder

### API

Additive, so it can ship in 4.x. The shape follows `option_chain(..)` →
`OptionChainBuilder` → `.subscribe()`; the `_stream` suffix follows
`order_update_stream`.

```rust
// both clients; C = sync or async Client
pub fn contract_details_stream<'a>(&'a self, contract: &'a Contract) -> ContractDetailsBuilder<'a, Self>;

impl<'a, C> ContractDetailsBuilder<'a, C> {
    pub fn request_id(&self) -> i32;
}
// sync
pub fn subscribe(self) -> Result<subscriptions::sync::Subscription<ContractDetails>, Error>;
// async
pub async fn subscribe(self) -> Result<subscriptions::r#async::Subscription<ContractDetails>, Error>;
```

Usage:

```rust
let request = client.contract_details_stream(&contract); // id allocated, nothing sent
let id = request.request_id();                           // log / admit / pace by id
let subscription = request.subscribe().await?;           // one send, no retry
let mut rows = subscription.filter_data().take(64);
while let Some(row) = rows.next().await {
    let row = row?;
    // ...
}
// dropping `rows` cancels natively if TWS hasn't ended the request
```

Semantics:

- **Allocation is eager.** `ContractDetailsBuilder::new` calls
  `client.next_request_id()`, so the id is fixed and visible with no I/O.
- **A dropped unsent builder skips an id.** Nothing is registered or written,
  so there is nothing to clean up. Ids are never reused, and the allocator
  ceiling is `i32::MAX-1`.
- **No routing gap.** No frame can arrive for an unsent id. `.subscribe()`
  registers the response channel inside `send`, before the write, as every
  request-id subscription already does.
- **Construction can't fail**, matching `option_chain`. `verify_contract`,
  the version check and encoding run in `.subscribe()`, still before any write.
  So a `subscribe()` error is either a validation error (nothing written) or an
  I/O error.
- **One builder, one send.** `subscribe(self)` consumes the builder, so a retry
  takes a new builder and therefore a new id.

Considered and rejected:

- **Turning `contract_details(&c)` itself into this builder.** It breaks 17
  call sites in `examples/` + `docs/` for no gain over an additive method.
  Revisit for 5.0 only if more builder options turn up.
- **Fallible construction** (`contract_details_stream(&c)?` running
  `verify_contract` up front). It's inconsistent with `option_chain`, and an
  early validation error buys nothing since `subscribe()` validates before
  writing anyway.

### Implementation

1. **Builder.** `src/contracts/contract_details_builder.rs` holds
   `ContractDetailsBuilder<'a, C> { client: &'a C, contract: &'a Contract,
   request_id: i32 }`. It sits next to `option_chain_builder.rs`, with the same
   per-feature terminal `impl` blocks and `#[must_use]`, and is re-exported from
   `contracts/mod.rs`.
2. **Send with a pre-allocated id.** The internal `RequestBuilder`
   (`client/builders/{sync,async}.rs`) allocates its own id in `client.request()`.
   `RequestBuilder::with_id(client, request_id)` already exists on both sides
   (currently `#[allow(dead_code)]`). `.subscribe()` uses it, then
   `builder.send::<ContractDetails>(packet)`. No new transport code.
3. **Decoder.** `impl StreamDecoder<ContractDetails> for ContractDetails` in
   `src/contracts/common/stream_decoders.rs`:
   - `RESPONSE_MESSAGE_IDS = [ContractData, ContractDataEnd]`
   - `decode`: `ContractData` → `decoders::decode_contract_details`; `ContractDataEnd` → `Err(Error::EndOfStream)`
   - `cancel_message`: `check_version(server_version, Features::CANCEL_CONTRACT_DATA)`, then
     `encode_cancel_contract_data(request_id)`. On older servers return
     `Err(NotImplemented)` so drop is a no-op, matching the `StreamDecoder` default.
   - `ContractData` / `ContractDataEnd` routing is already registered
     (`messages/tests.rs:185,192`).
4. **Vec wrapper.** Rewrite `contract_details` as
   `contract_details_stream(c).subscribe()` + collect. This removes the
   hand-rolled `send_raw` loop on both sides. Error behaviour must stay the same:
   code 200 → `Err`, closed channel → `UnexpectedEndOfStream`.
5. **Don't cancel after the native end.** Today `Subscription::cancel`/`Drop`
   skips only on `snapshot_ended`. After `EndOfStream` it would send
   `cancelContractData` for a request that has already finished, and every Vec
   call would hit this. Skip the cancel when `stream_ended` was set by
   `EndOfStream`, on both sides. The async `stream_ended` is per clone, so either
   check it on the dropping handle or promote it to a shared `Arc` like
   `snapshot_ended`; decide while implementing. Other ended streams benefit too.
   Grep `cancel_message` impls to confirm none relies on the post-end cancel.

### Outcome mapping (rustdoc)

| Wire | Caller sees |
| --- | --- |
| `ContractData` | `Some(Ok(Data(details)))` |
| `ContractDataEnd` | `None` |
| error 200 (no definition) | `Some(Err(Error::Notice(..)))`, then `None` |
| warning-class notice | `Some(Ok(Notice(..)))` |
| connection reset | `Some(Err(Error::ConnectionReset))`, no retry; caller builds a new request (new id) |
| shutdown | `Some(Err(Error::Shutdown))` |

Check the 200 row against the live gateway or the C# reference before writing
it into docs.

### Tests

Use `MessageBusStub` via `create_test_client` / `create_blocking_test_client`,
with fixtures from `testdata/builders/contracts.rs`.

- builder: `request_id()` before `subscribe()` equals the id in the encoded
  request; no bytes are written before `subscribe()`
- builder dropped unsent → nothing written; the next builder gets a higher id
- invalid contract → `subscribe()` returns the `verify_contract` error, nothing written
- N rows + end → N data items, then `None`; drop writes no cancel (the step 5 guard)
- `.take(1)` of 3 rows → drop writes `cancelContractData(id)` (server ≥ 215)
- the same at server < 215 → no cancel written
- error 200 → `Err(Notice)` with code 200
- `contract_details` (Vec) regression: existing tests stay green unchanged
- the `response_message_ids_tests.rs` agreement test covers the new decoder automatically

## Part 2: `matching_symbols` retry semantics

There's no API change. `one-shot-narrowing.md` (#741) deliberately removed the
retry axis, and the retry is narrow:

- only on `Error::ConnectionReset`, at most 3 retries, 4 attempts total (`DEFAULT_MAX_RETRIES`)
- waits for `wait_connected()` before each retry, so it never writes into a dead session
- each attempt allocates a fresh request id

Add a `# Retries` section to the `matching_symbols` rustdoc (sync + async)
stating the above. A caller that paces its requests can count resets through the
notice stream (reconnect notice). If the issue author shows that a retry after a
reconnect actually breaks gateway pacing, reopen the question with a
rule-sanctioned opt-out. Don't add `.no_retry()` speculatively.

## Part 3: fail-closed `buffer_limit` (follow-up PR)

The memory bound for a reader that stalls. Today the async channels are bounded
but lossy: global `BROADCAST_CHANNEL_CAPACITY` = 1024, drop-oldest, with a
non-terminal lag notice `-6` (#779 step 1). The sync channels are unbounded.
Neither suits enumeration: a silently partial list is wrong, and so is
unbounded growth.

### API

```rust
let subscription = client
    .contract_details_stream(&contract)
    .buffer_limit(512)        // max unread items before failing
    .subscribe()
    .await?;
```

- **`.buffer_limit(n)` is optional.** Without it, behaviour is today's (async
  1024 lossy, sync unbounded). `n == 0` is rejected in `subscribe()` with
  `InvalidArgument`.
- **Overflow is terminal.** When `n` items sit unread, the dispatcher stops
  queueing for that request id and delivers a terminal
  `Err(Error::BufferLimitExceeded { limit })`. It's a new variant; `Error` is
  `#[non_exhaustive]`. The reader gets every item queued before the error, in
  order, then the error. Nothing is dropped silently.
- **The dispatcher never blocks.** Its check is a length comparison, so it
  never waits for a slow reader.
- **Cancel still fires.** Overflow doesn't set the end-of-stream guard from
  part 1 step 5, so dropping the subscription (or calling part 4) sends the
  native cancel. Further frames for that id are dropped as unrouted.

### Implementation

- **Mechanism.** The internal `RequestBuilder` gets an optional `buffer_limit`,
  carried to the bus with the registration (`send_request`). Store it next to
  the sender (`Route { sender, limit: Option<usize> }` or similar), not in a
  second map, so the routing paths stay single-lookup.
- **Sync.** Before `send`, check `sender.len() >= limit` (crossbeam
  `Sender::len`). If it's full, send the terminal error (the channel is
  unbounded, so it always fits) and remove the route.
- **Async.** Create the broadcast with capacity `limit + 1`. Before `send`,
  check `sender.len() >= limit` (tokio `broadcast::Sender::len`, which counts
  values not yet seen by every receiver). If it's full, send the terminal
  error, which takes the reserved slot so nothing is evicted, and remove the
  route.
- **Builder surface.** Add the public setter only to `ContractDetailsBuilder`
  for now; the mechanism lives in `RequestBuilder`, so other builders can opt in
  with one line.
- **#779 step 2.** This is a third channel class ("enumeration": bounded +
  lossless + fail-closed) next to order-class and market-data-class. Record it
  in `plans/broadcast-lag-visibility.md`.

### Tests

- sync + async: limit 2, 3 rows unread → 2 rows, then `BufferLimitExceeded`, then `None`
- reader keeping up with limit 2 and 10 rows → all 10 + end; the limit counts unread items, not total
- after overflow, drop → `cancelContractData` written (server ≥ 215)
- async: no `-6` lag notice is emitted on the overflow path (no eviction)
- `buffer_limit(0)` → `InvalidArgument`, nothing written

## Part 4: `Subscription::cancel_and_drain(deadline)` (follow-up PR)

Confirmed cleanup. A caller that pools request slots, or keeps strict pacing,
needs to know TWS has finished with a request before reusing the slot. Drop's
fire-and-forget cancel can't tell it that.

### API

On both `Subscription` types, generic over `T: StreamDecoder<T>`:

```rust
// sync
pub fn cancel_and_drain(self, deadline: std::time::Instant) -> Result<Drained, Error>;
// async
pub async fn cancel_and_drain(self, deadline: tokio::time::Instant) -> Result<Drained, Error>;

#[derive(Debug)]
pub enum Drained {
    /// TWS sent the end marker.
    Ended,
    /// TWS sent an error for this request id; the request is over at TWS.
    Rejected(Notice),
    /// No terminal frame arrived before the deadline. Treat the id as possibly
    /// still live: don't assume a quiet socket means the request is finished.
    DeadlineElapsed,
}
```

Semantics:

1. **Already ended** (`EndOfStream` seen): return `Ended` without writing.
2. **Otherwise**, send the decoder's `cancel_message` if it has one, and mark
   the subscription cancelled so `Drop` doesn't send it again. No cancel message
   (an older server, or a decoder without one) means nothing is written; the
   method then just waits for a natural end.
3. **Read and discard** until the end marker, a TWS error for the id, or the
   deadline. Notices are discarded; data items are discarded.
4. **Session errors** (`ConnectionReset`, `Shutdown`) → `Err`. The request died
   with the session.
5. **It consumes `self`**, so no one can read after a drain.
6. **Async cancellation is safe.** Dropping the future mid-drain drops the
   subscription; the cancel is already written and the `cancelled` flag stops a
   second one. There is no session retirement, unlike #877.
7. **Streams without an end marker** (market data) always return
   `DeadlineElapsed` after their cancel. Document this; don't restrict the method.

### Implementation

- **Sync.** Loop on `next_timeout(deadline - now)`.
- **Async.** `tokio::time::timeout_at(deadline, ..)` around a `next()` loop.
- **The cancel step** reuses the body of `cancel()` but returns the write
  result instead of logging it. Extract a shared private helper; don't fork it.
- **Async clones.** The async `Subscription` is `Clone`. A drain on one clone
  sends the shared cancel, and the other clones see the stream end (or the error).
  Pin this down in a test.

### Tests

- 2 rows queued + end → `Ended`, no cancel written
- pending + scripted `ContractDataEnd` after the cancel → cancel written, `Ended`
- pending + error frame for the id → `Rejected(notice)`
- pending + silence → `DeadlineElapsed` at the deadline, cancel written once (not again on drop)
- server < 215 → nothing written; scripted natural end → `Ended`
- async: drain future dropped mid-wait → exactly one cancel written

## Follow-ups

- **Id before I/O on other builders** (e.g. `option_chain(..).request_id()`,
  with allocation moved into `OptionChainBuilder::new`). It's the same change
  each time, so add it per builder when a caller needs it, not as a sweep.
- **`buffer_limit` on other builders** (option chain, historical data), when a
  caller asks.

## Open questions

- **What TWS sends after `cancelContractData`: answered live (2026-10-01,
  server 225).** SPY calls for 202611 (714 rows): after reading 3 rows and
  sending the cancel, TWS still sent the remaining 711 rows over about 1.1s,
  then `ContractDataEnd`. That's the same count and timing as with no cancel.
  So the cancel doesn't stop a result TWS has already prepared, but the end
  marker does still arrive. For part 4, `cancel_and_drain` should normally
  return `Ended` after discarding the tail; `DeadlineElapsed` means a stalled
  request, not the normal case.
- **Unfiltered query (all SPY options), read to the end, 2026-10-01.** TWS
  produces it incrementally: first row at 8.4s, 4,340 rows by about 113s,
  then nothing for 60s (end marker not seen in that window). The gateway
  stayed healthy. Earlier, four such runs killed mid-stream within a few
  minutes wedged it until restart. Open for part 4: does a cancel stop an
  incremental result? Test it once (read ~100 rows, cancel, watch for the
  tail and the end), and keep unfiltered queries out of CI.
- **`BondContractData` (msg 18).** C# decodes it under protobuf (the same
  `ContractData` proto, `isBond = true`); we don't route it, so bond results are
  likely dropped silently. This is a separate issue; see
  [PR 1 plan](contract-details-stream-pr1.md#out-of-scope-found-while-planning).
- **`next_request_id()` is public.** A caller can already allocate ids. Do the
  builder docs need a note that ids from it are not for this builder (no setter)?
- **Async Vec collector over 1024 rows.** The existing async `contract_details`
  rides the lossy 1024 channel. A fast collector never lags in practice, but
  once part 3 exists, should the Vec wrapper set its own `buffer_limit` to turn
  a theoretical silent gap into an error?

## Size

| PR | Source | Tests |
| --- | --- | --- |
| Parts 1–2: builder, decoder, Vec rewrite, cancel-after-end guard, retry docs | ~200 | ~200 |
| Part 3: `buffer_limit` | ~120 | ~150 |
| Part 4: `cancel_and_drain` | ~120 | ~150 |
| Total | ~440 | ~500 |
