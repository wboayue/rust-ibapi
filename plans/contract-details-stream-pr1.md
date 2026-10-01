# PR 1: `contract_details_stream` builder + retry docs (#876)

Parts 1–2 of [contract-details-stream](contract-details-stream.md). Additive,
4.x-safe. Parts 3 (`buffer_limit`) and 4 (`cancel_and_drain`) follow as
separate PRs.

## Scope

1. `ContractDetailsBuilder`: the request id is allocated when the builder is
   built, `.request_id()` reads it, and `.subscribe()` sends once.
2. `StreamDecoder<ContractDetails>`, with the native cancel on server 215+.
3. Cancel-after-end guard on both `Subscription` types.
4. `contract_details` (Vec) rewritten as a collect over the stream.
5. A `# Retries` section in the `matching_symbols` rustdoc.
6. Docs, examples, CHANGELOG, integration tests.

## 1. Builder

New file `src/contracts/contract_details_builder.rs`, modelled on
`option_chain_builder.rs` (module doc, `#[must_use]`, per-feature terminal
`impl` blocks). Re-export it from `contracts/mod.rs` next to
`OptionChainBuilder`.

```rust
#[must_use = "ContractDetailsBuilder does nothing until you call .subscribe()"]
pub struct ContractDetailsBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    request_id: i32,
}

impl<'a, C> ContractDetailsBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract, request_id: i32) -> Self;
    /// Allocated when the builder was made; nothing has been sent yet.
    pub fn request_id(&self) -> i32;
}
```

- **The id is passed in, not allocated inside `new`.** `new` is generic over
  `C` with no bounds, so the `Client::contract_details_stream` methods call
  `self.next_request_id()` and pass the result to `new`.
- **The terminal** `subscribe()` (sync: `-> Result<Subscription<ContractDetails>, Error>`;
  async: the same, `async`):
  ```rust
  verify::verify_contract(client.server_version(), contract)?;
  let packet = encoders::encode_request_contract_data(request_id, contract)?;
  RequestBuilder::with_id(client, request_id).send::<ContractDetails>(packet)
  ```
  `RequestBuilder::with_id` already exists on both sides. Its impl carries
  `#[allow(dead_code)]`; once this caller lands, check whether the allow can be
  narrowed.
- **The client entry point** goes in `contracts/{sync,async}.rs`, next to
  `option_chain`:
  ```rust
  pub fn contract_details_stream<'a>(&'a self, contract: &'a Contract) -> ContractDetailsBuilder<'a, Self> {
      ContractDetailsBuilder::new(self, contract, self.next_request_id())
  }
  ```
  It's sync (not `async fn`) on both clients, like `option_chain`.

## 2. Decoder

In `src/contracts/common/stream_decoders.rs`:

```rust
impl StreamDecoder<ContractDetails> for ContractDetails {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] =
        &[IncomingMessages::ContractData, IncomingMessages::ContractDataEnd];

    fn decode(_: &DecoderContext, message: &ResponseMessage) -> Result<ContractDetails, Error> {
        match message.message_type() {
            IncomingMessages::ContractData => decoders::decode_contract_details(message),
            IncomingMessages::ContractDataEnd => Err(Error::EndOfStream),
            _ => Err(Error::unexpected_response(message)),
        }
    }

    fn cancel_message(server_version: i32, request_id: Option<i32>, _: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        check_version(server_version, Features::CANCEL_CONTRACT_DATA)?;
        let request_id = request_id.ok_or_else(|| Error::InvalidArgument("request id required to cancel contract details".into()))?;
        encoders::encode_cancel_contract_data(request_id)
    }
}
```

- **Older servers.** `cancel_message` returning `Err` on a server below 215
  makes drop a no-op, which is the existing path for decoders without a cancel
  message (e.g. `OptionChain`).
- **Routing.** `ContractData` / `ContractDataEnd` are already in
  `routes_by_request_id` (`messages/tests.rs:185,192`).
- **Agreement test.** Add `ContractDetails` to `response_message_ids_tests.rs`
  if that test lists its decoders explicitly.
- **Decoder test.** Add a row to `contracts/common/test_tables.rs` if the
  stream-decoder table there covers the other contract decoders
  (`OptionChain` has an `EndOfStream` row at :803).

## 3. Cancel-after-end guard

Today `Subscription::cancel`/`Drop` (sync `subscriptions/sync.rs:79`, async
`subscriptions/async.rs:343,384`) skips only on `snapshot_ended`. After
`EndOfStream` it still writes the cancel. Without a guard, every Vec
`contract_details` call would write a stray `cancelContractData`.

- **New flag.** Add `ended_natively` (sync: `AtomicBool`; async:
  `Arc<AtomicBool>` shared across clones, like `snapshot_ended`, since once any
  clone has seen the end marker the request is complete at TWS). Set it at the
  two `EndOfStream` sites per side: decode returning `Err(EndOfStream)`, and
  `RoutedItem::Error(EndOfStream)`. Don't set it on other errors: after a TWS
  error or a reset, the cancel is still sent as today.
- **Effect.** When `ended_natively && request_id.is_some()`, behave exactly as
  if `cancel_message` returned `Err`: no wire write, with the same local cleanup
  as the `OptionChain` path. The `request_id.is_some()` condition keeps
  shared-channel subscriptions unchanged; their count release must still run.
- **Audit (done while planning).** The `EndOfStream` decoders are
  `OptionChain` (no cancel), `NewsArticle` (`HistoricalNewsEnd`; its cancel is
  only for `RequestMarketData`, which never gets that frame), orders
  `OpenOrderEnd`/`CompletedOrdersEnd` (shared), and `Executions`. Re-grep
  `Err(Error::EndOfStream)` when implementing, and confirm the `Executions`
  cancel path. No current stream relies on a cancel after its end marker.
- **Historical ticks** use `TickAction::EndOfStream` in their own subscription
  type, so this change doesn't touch them.

## 4. Vec rewrite

Both sides:

```rust
pub fn contract_details(&self, contract: &Contract) -> Result<Vec<ContractDetails>, Error> {
    let subscription = self.contract_details_stream(contract).subscribe()?;
    let mut details = Vec::new();
    while let Some(item) = subscription.next() {
        match item? {
            SubscriptionItem::Data(d) => details.push(d),
            SubscriptionItem::Notice(n) => log::warn!("contract details notice: {n}"),
        }
    }
    if !subscription.ended_natively() {
        return Err(Error::UnexpectedEndOfStream);
    }
    Ok(details)
}
```

Behaviour to keep, with the existing tests in `contracts/{sync,async}_tests.rs`
as the judge:

- **Code 200 and other errors** → `Err`. Unchanged: both paths surface
  `RoutedItem::Error`.
- **Channel closed without an end marker** → `UnexpectedEndOfStream`.
  `Subscription::next` returns a bare `None` on close, so the wrapper needs the
  `pub(crate) fn ended_natively()` accessor (the step 3 flag) to tell the two
  apart.
- **Notices.** The old `send_raw` path dropped them silently (`into_legacy`);
  the wrapper logs them instead. That's strictly more visible and doesn't change
  the result.
- **Undeclared message types.** These used to be an `unexpected_response`
  error; the subscription now skips them at `trace!`. This only matters if TWS
  sends a foreign type on this request id, which routing already prevents.
  Accept it and mention it in the PR body.
- **No extra cancel.** The existing `request_message_count(..) == 1`
  assertions (e.g. `request_bond_contract_details`, `async_tests.rs:392`)
  enforce the step 3 guard on the Vec path.

Use the explicit match rather than `iter_data().flatten()` or `filter_data`
(see [[feedback_result_flatten_drops_errors]]).

## 5. `matching_symbols` retry docs

Add to the rustdoc on both sides (`contracts/sync.rs:154`, `contracts/async.rs`):

```text
/// # Retries
///
/// If the connection resets mid-request, this waits for the reconnect and
/// re-sends with a fresh request id, up to 3 times (4 attempts total). Other
/// errors are not retried. A caller pacing requests against TWS limits should
/// count each reconnect (see the notice stream) as a possible extra request.
```

Check `DEFAULT_MAX_RETRIES` (`common/retry.rs:15`) when writing it.

## 6. Docs, examples, changelog

- **Rustdoc `# Examples`** on `contract_details_stream` and both `subscribe`s,
  sync and async paired ([[feedback_per_method_sync_async_doc_pairing]]).
  Show `request_id()` before `subscribe()`, an explicit `next()` loop, and an
  early `break` (which cancels on drop). Don't use `Ok(_) => {}` catch-alls
  ([[feedback_doc_example_user_trace]]).
- **Examples:** `examples/{sync,async}/contract_details_stream.rs`, registered
  in `Cargo.toml` like the others. Run both against a gateway
  ([[feedback_examples_expose_test_gaps]]).
- **`contract_details` rustdoc:** one line pointing to `contract_details_stream`
  for cancel, row caps, or the request id.
- **CHANGELOG** `[Unreleased]`:
  - Added: `contract_details_stream` and `ContractDetailsBuilder` (#876).
  - Changed: dropping a request-id subscription after its end marker no
    longer writes a cancel.
  - Docs: `matching_symbols` retry semantics.
- **`docs/*.md`:** grep for `contract_details(`; no change is needed unless a
  snippet should showcase the stream.
- **Link check:** `RUSTDOCFLAGS="-D warnings" cargo doc` for intra-doc links
  ([[feedback_rustdoc_link_check]]).

## Tests

Unit tests use `MessageBusStub` via `create_test_client` /
`create_blocking_test_client`; fixtures come from
`testdata/builders/contracts.rs` (`contract_data()`, `contract_data_request()`;
add a `contract_data_end()` builder if the text literal `"52|1|9001|"` keeps
recurring). Sync and async each get:

| Test | Asserts |
| --- | --- |
| `request_id` before subscribe | `request_id()` equals the id in the encoded request; request count 0 before `subscribe()` |
| unsent builder | dropped → 0 requests; next builder's id is higher |
| invalid contract | `subscribe()` returns the `verify_contract` error; 0 requests |
| full stream | 2 rows + end → 2 `Data`, then `None`; drop → still 1 request (guard) |
| early drop | 3 rows, read 1, drop → 2 requests, second is `cancelContractData(id)` (server ≥ 215) |
| early drop, old server | the same at server < 215 → 1 request |
| code 200 | `Err(Error::Notice)` with code 200, then `None` |
| async clones (async only) | clone A reads to end, clone B dropped first → no cancel (shared flag) |
| shared subscription unaffected | an `EndOfStream` shared stream (open orders) still releases its count on drop |
| Vec regression | existing `contract_details` tests pass unchanged |

Integration (`integration/{sync,async}/tests/contracts.rs`):

- AAPL stock: the stream yields the same rows as `contract_details`.
- Broad query (e.g. SPY options with no expiry): `.take(5)`-style early
  `break`, then drop. Afterwards there is no error notice for the id within a
  few seconds.
- **Temporary diagnostic** (for part 4, [[feedback_live_diagnostic_tests]]):
  after the early cancel, log everything that arrives for the id for ~5s
  (`ContractDataEnd`? error? nothing?). Record the answer in the main plan,
  then delete the test before merge.

## Commit sequence

1. Cancel-after-end guard + its tests (independent, and it makes the Vec
   rewrite's request-count assertions meaningful).
2. Decoder + builder + `contract_details_stream` + unit tests.
3. Vec rewrite.
4. `matching_symbols` docs, examples, CHANGELOG, integration tests.

## Pre-PR gate

`cargo fmt`; clippy with CI flags on all three feature sets (sync, async, all
features); `cargo test` on each feature set; the rustdoc `-D warnings` build;
the integration crates (`-p` each separately,
[[feedback_integration_crate_feature_unification]]); then self-review
([[feedback_self_review]]).

## Out of scope (found while planning)

**Bond contract details are silently dropped.** C# `EDecoder.cs:111` handles
`BondContractData` (msg 18) under protobuf: the same `protobuf.ContractData`
payload, decoded with `isBond = true`. We don't route 18 by request id (only
`messages/tests.rs:109` mentions it). A bond `contract_details` therefore
likely returns `Ok(vec![])` once `ContractDataEnd` arrives. Our
`request_bond_contract_details` test feeds a `ContractData` (10) frame, so it
doesn't catch this. Open a separate issue and fix it with a live bond
capture; don't fold it into this PR. Once fixed, `BondContractData` joins this
decoder's `RESPONSE_MESSAGE_IDS`.

## Size

Roughly 200 lines of source plus 250 of tests, and 2 examples.
