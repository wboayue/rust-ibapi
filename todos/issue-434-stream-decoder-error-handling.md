# Issue #434: surface IB errors in all stream decoders

## Context

GitHub issue [#434](https://github.com/wboayue/rust-ibapi/issues/434):

> When IB returns an error for a realtime bars subscription (e.g. error 10089 — missing market data subscription), the error message is routed to the bar decoder which tries to parse it as a `Bar`. This produces a misleading `"invalid digit found in string"` error instead of surfacing the actual IB error.

The routing layer (`transport/routing.rs::determine_routing` → `RoutingDecision::Error`) correctly delivers a non-warning type-4 error message to the matching subscription's request-id channel. But if the subscription's `StreamDecoder<T>::decode` does not match `IncomingMessages::Error`, it falls into the data-decoding arm and tries to read text fields as numbers, yielding `Error::ParseInt(...)` which surfaces as the cryptic `"invalid digit found in string"`.

PR #443 already fixed this for the realtime decoders (`Bar`, `BidAsk`, `MidPoint`, `Trade`, `MarketDepths`, `TickTypes`) on both branches. **The same bug still exists in every other `StreamDecoder` impl that does not match `IncomingMessages::Error`.** Issue #434's title names bar data, but the fix is systemic.

Scope: audit and fix every remaining `StreamDecoder` impl on both `v2-stable` and `main`, using `Err(Error::from(message.clone()))` (terminate-and-surface) — matching the existing `Bar` pattern.

## Approach

For each `StreamDecoder<T>` impl that is missing error handling:

1. Add `IncomingMessages::Error` to its `RESPONSE_MESSAGE_IDS` array.
2. Add a `decode` match arm: `IncomingMessages::Error => Err(Error::from(message.clone()))`.
3. Add a unit test that constructs a type-4 error `ResponseMessage` and asserts `T::decode(...)` returns `Err(Error::Message(code, _))`, not a parse error.

`impl From<ResponseMessage> for Error` already produces `Error::Message(code, message)` (`src/errors.rs:114-120` on `main`, `src/errors.rs:104-108` on `v2-stable`) using the `error_code()` / `error_message()` helpers on `ResponseMessage`. No new error variant needed on either branch.

Routing layer's warning filter (`is_warning_error` at `src/transport/routing.rs:148-150`, applied in `transport/sync/mod.rs:275` and `transport/async.rs::route_error_message`) keeps codes 2100–2199 on the broadcast channel, so anything reaching a `StreamDecoder` is a real error — `Err` is the right shape.

### Reference pattern (already on both branches)

`src/market_data/realtime/mod.rs` `StreamDecoder<Bar>`:

```rust
impl StreamDecoder<Bar> for Bar {
    const RESPONSE_MESSAGE_IDS: &[IncomingMessages] = &[IncomingMessages::RealTimeBars, IncomingMessages::Error];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::RealTimeBars => common::decoders::decode_realtime_bar(context, message),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
    // …
}
```

## Files to modify — `v2-stable`

Tests: inline `#[cfg(test)] mod tests` per v2-stable convention (no sibling `_tests.rs` on this branch).

| File | Decoders to patch |
| --- | --- |
| `src/accounts/common/stream_decoders.rs` | `AccountSummaryResult`, `PnL`, `PnLSingle`, `PositionUpdate`, `PositionUpdateMulti`, `AccountUpdate`, `AccountUpdateMulti` |
| `src/contracts/common/stream_decoders.rs` | `OptionComputation`, `OptionChain` |
| `src/display_groups/common/stream_decoders.rs` | `DisplayGroupUpdate` |
| `src/news/sync.rs` | `NewsBulletin`, `NewsArticle` |
| `src/news/async.rs` | `NewsBulletin`, `NewsArticle` |
| `src/scanner/sync.rs` | `Vec<ScannerData>` |
| `src/scanner/async.rs` | `Vec<ScannerData>` |
| `src/wsh/common/stream_decoders.rs` | `WshMetadata`, `WshEventData` |

Orders decoders (`PlaceOrder`, `OrderUpdate`, `CancelOrder`, `Orders`, `Executions`, `ExerciseOptions`) and realtime decoders are already fixed on `v2-stable` — leave them alone.

`v2-stable` duplicates news/scanner `StreamDecoder` impls between `sync.rs` and `async.rs` (not yet consolidated to `common/`). Patch both copies and keep them in sync.

## Files to modify — `main`

Tests: flat sibling files (memory: `feedback_main_sibling_tests.md`). Either extend an existing sibling tests file or add a new `*_tests.rs` and wire it in with `#[cfg(test)] #[path = "..."] mod tests;` from the impl file.

| File | Decoders to patch |
| --- | --- |
| `src/accounts/common/stream_decoders/mod.rs` | `AccountSummaryResult`, `PnL`, `PnLSingle`, `PositionUpdate`, `PositionUpdateMulti`, `AccountUpdate`, `AccountUpdateMulti` |
| `src/contracts/common/stream_decoders.rs` | `OptionComputation`, `OptionChain` |
| `src/display_groups/common/stream_decoders.rs` | `DisplayGroupUpdate` |
| `src/news/common/stream_decoders.rs` | `NewsBulletin`, `NewsArticle` |
| `src/scanner/common/stream_decoders.rs` | `Vec<ScannerData>` |
| `src/wsh/common/stream_decoders.rs` | `WshMetadata`, `WshEventData` |

Notes:

- `OptionChain::decode` uses `IncomingMessages::SecurityDefinitionOptionParameterEnd → Err(Error::EndOfStream)` to terminate the snapshot. Preserve that arm; insert the `Error` arm before the catch-all.
- `OptionComputation` on v2-stable currently catches with `message => Err(Error::Simple(...))`. Replace with `_ => Err(Error::UnexpectedResponse(message.clone()))` while adding the `Error` arm — match the dominant pattern.
- Orders and realtime decoders on `main` are already fixed (PR #443 / earlier). Don't touch them.

## Test pattern (one per decoder type)

```rust
#[test]
fn decode_surfaces_ib_error() {
    // Type 4 = Error. Format here is server-version >= ERROR_TIME (no version field):
    //   msg_type | request_id | error_code | error_msg | advanced_order_reject_json | error_time
    let mut message = ResponseMessage::from("4\09002\010089\0Requested market data requires additional subscription for API\0\01772160892101\0")
        .with_server_version(crate::server_versions::ERROR_TIME);

    let context = DecoderContext::new(crate::server_versions::ERROR_TIME, None);
    let result = <Bar as StreamDecoder<Bar>>::decode(&context, &mut message);

    match result {
        Err(Error::Message(code, msg)) => {
            assert_eq!(code, 10089);
            assert!(msg.contains("Requested market data"));
        }
        other => panic!("expected Err(Error::Message(...)), got {other:?}"),
    }
}
```

Each affected decoder file gets one such test (named for the decoder, e.g. `decode_account_summary_surfaces_ib_error`). Append alongside existing tests where present.

## Verification

For each branch:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features
just test
cargo test --all-features -- decode_surfaces_ib_error
```

End-to-end sanity check (manual; needs paper-trading gateway): run an example requesting realtime bars on a contract you don't have data for, confirm `subscription.error()` returns `Some(Error::Message(10089, _))` and the log no longer contains `"invalid digit found in string"`.

## PR strategy

Per `feedback_dual_branch_default.md`:

1. PR against `v2-stable` first. Title: `fix: surface IB errors in all stream decoders (#434)`.
2. After merge, PR against `main` referencing the v2-stable PR. Same title; body uses `Closes #434`.

## Out of scope

- The misleading `error!("error decoding message: {err}")` log at `src/subscriptions/sync.rs:189` (and async equivalent). The formatted `err` already includes the IB code/message — leave for a follow-up if the user wants cleaner logs.
- Restructuring v2-stable tests into sibling files (sibling-file convention is `main` only).
- Consolidating v2-stable's news/scanner `StreamDecoder` impls between `sync.rs` and `async.rs` into a shared `common/` module — orthogonal refactor.
