# Bounded contract queries

Use the prepared API when a service needs explicit request ownership, local
memory limits, caller-controlled pacing and cleanup. The existing
`contract_details`, `matching_symbols` and option-chain subscription APIs
remain available; they do not acquire these new bounded-query semantics.

## Prepare, admit, write, read, clean up

The async client exposes:

```rust,no_run
use ibapi::contracts::{Contract, QueryLimits, SecurityType};

async fn example(client: &ibapi::Client) -> Result<(), ibapi::Error> {
    let details = client.prepare_contract_details(
        &Contract::stock("SYNTH").build(), QueryLimits::default(),
    )?;
    let symbols = client.prepare_matching_symbols("SYNTH", QueryLimits::default())?;
    let parameters = client.option_chain("SYNTH", SecurityType::Stock, 12345)
        .prepare(QueryLimits::default())?;

    // These own local registrations and expose IDs; nothing was written yet.
    println!("{} {} {}", details.request_id(), symbols.request_id(), parameters.request_id());
    drop((details, symbols, parameters));
    Ok(())
}
```

The examples use synthetic identifiers, not a valid-instrument promise. Resolve
the actual underlying before requesting its parameters. For stock underlyings,
leave the option-chain builder's futures-option exchange unset. Preserve native
groups and their reported trading class/multiplier; parameter arrays are not a
Cartesian list of valid listed contracts.

Prepare allocates one ID and an owned local registration, but does no I/O.
`start()` submits once. There are no retries, hidden background writers, local
filtering, pagination or pacing. Callers admit and pace each actual attempt.
A prepared query invalidated by a reconnect before `start` fails without
writing. A `start` refused because the session is reconnecting returns
`Error::ConnectionReset` without writing or retiring anything: the query stays
`NotSubmitted` and can be started once the client is connected again
(`Client::is_connected`, or the reconnect notice on the notice stream). Never
reuse a query for a retry after a write: create a new one with a fresh ID.

`next().await` returns a data item, an in-band notice, a failure, or `None`.
Data is one `ContractDetails`, one native `OptionChain` group, or one vector of
native symbol descriptions. An empty symbol vector requires an actual empty
symbol-samples frame. After a terminal/local read error, reads are fused; the
following `None` does not undo the error. Keep every already-returned item and
record why collection stopped.

Retirement does not make a buffered prefix inaccessible: after a failed cancel
or submission write, reads can still yield already-received rows/notices before
the terminal error. `drain_until` is the explicit choice to discard unread
items. Already-returned values remain owned by the caller in either case.

## Budgets

`QueryLimits` values must all be nonzero; validation precedes submission.
Defaults are 256 native rows, 4,096 data/notice frames, 64 KiB per retained frame,
2 MiB cumulative queued payload, and 8,192 protobuf fields/packed elements.
Reading an item never replenishes a budget. Native rows are counted before
consumer filtering; symbol descriptions count individually despite arriving in
one frame. An oversized symbol frame is rejected as a frame, not partially
decoded into a misleading successful prefix.

The queue enforces frame/count/byte limits at dispatch. Domain decoding first
scans borrowed protobuf bytes without allocating arrays/maps: nested contract
fields, map entries, duplicate message occurrences, option expirations, and
both packed and unpacked strikes spend the decode-entry budget. String sizes
and subsequent string-derived domain allocations are bounded by payload bytes,
not counted as protobuf array elements. The socket's existing global frame
ceiling still applies before request routing; a per-request byte limit cannot
prevent that temporary initial frame allocation. One bounded terminal diagnostic
is retained separately from the data queue.

The native end is tracked outside the queue, so reaching a limit does not hide
later completion. These are local retention/work limits, not limits on Gateway
computation after a query is sent. Keep queries narrow even with local limits.

## Completion and session reuse

`QueryDisposition` describes lifecycle evidence, not collected-row completeness
or the health of unrelated requests. In particular, `ResponseEnded` can coexist
with a reported local limit: the server ended, but collection was partial.

| Outcome | Evidence / disposition | Cleanup and session consequence |
| --- | --- | --- |
| Prepared, not submitted | `NotSubmitted` | Drop only unregisters locally. |
| Native end received, including empty response | `ResponseEnded` | Drop only unregisters; no retirement needed by this query. |
| Native definition error 200 | `DefinitionRejected`, original notice returned by read | Not an empty success; no retry and no retirement solely for that rejection. |
| Read timeout, local row/frame/decode limit, or canceled async read | Usually still `Pending` | Keep confirmed items and ownership; cancel/drain within the remaining budget or retire. |
| Native cancel written | Still `Pending` unless terminal evidence independently arrived | A successful write is not an acknowledgement; keep registration and drain. |
| Failed/dropped submission or cancel write | `RetireRequired` | Shutdown is latched; queued/later writes cannot append to an uncertain frame. |
| EOF, reset, other terminal failure | `RetireRequired` | Preserve confirmed prefix and error. This query does not certify reuse; cleanup/drop retires. |
| Cleanup deadline or canceled async cleanup | `RetireRequired` | Shutdown is requested immediately. No detached cleanup writer remains. |
| Unfinished submitted handle dropped | No remaining handle | Conservatively request shutdown of this client session. |

`request_cancel` writes once only for contract details on server version 215 or
newer. Its `true` result means written, **not acknowledged**. `false` means no
write was needed or available: for example an already completed request, older
server, symbol search, or option parameters (which have no native cancel here).
`drain_until` discards unread queued items and waits for native terminal evidence;
it does not itself write cancellation. On older/no-cancel paths, it can still
observe natural completion within the remaining budget. Missing completion
requires retirement, not optimistic reuse.

Retirement means requesting shutdown of **the client session that owns this
query**. Account subscriptions and other queries sharing that client can be
interrupted. Unrelated clients sharing the same Gateway are not retired. Call
`Client::disconnect` when the application needs joined teardown before releasing
its own admission turn or replacing the session. Dropping a query is not itself
a joined teardown guarantee.

## Time and feature variants

The async type is `contracts::ContractQuery`, also explicitly available as
`contracts::enumeration::async_impl::ContractQuery`. Use one absolute
`tokio::time::Instant` budget around preparation/admission, `start`, collection,
optional `request_cancel`, `drain_until` and application teardown. Reserve
cleanup time up front instead of restarting a fresh timeout after every notice
or retry. No SDK method silently renews the caller's budget.

The blocking type is `client::blocking::ContractQuery` (or the top-level
contracts alias in sync-only builds). It offers `start`, `next_until` with a
`std::time::Instant`, `request_cancel`, and `drain_until`. Its read/drain waits
are deadline-bounded, but **blocking socket writes are not bounded by those read
deadlines**. Choose the async path when an operation needs a cancellable total
submission/collection/cleanup budget; do not wrap a blocking write in an
abandoned background thread and call it canceled.
