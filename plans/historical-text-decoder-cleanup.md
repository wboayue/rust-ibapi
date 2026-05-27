# Historical-data text-decoder cleanup

> **Status: ✅ SHIPPED — closed.** All 9 historical-data decoders went
> proto-only in [#626](https://github.com/wboayue/rust-ibapi/pull/626).
> Retained for historical reference.

Continues the per-family ratchet sweep started in PRs #529 / #531 / #532 / #534 /
#543 / #623. Floor is at 210 (`PROTOBUF_SCAN_DATA`). This PR deletes the
now-unreachable text branches in `market_data/historical/common/decoders/mod.rs`.

## Per-decoder gate analysis

Sourced from `/Users/wboayue/projects/tws-api/source/csharpclient/client/Constants.cs`
(`PROTOBUF_MSG_IDS`). All historical-data response IDs are gated by
`PROTOBUF_HISTORICAL_DATA` (208); the floor is 210, so all 9 text branches are
unreachable.

| Decoder                             | Incoming msg id              | Gate              |
|-------------------------------------|------------------------------|------------------:|
| `decode_head_timestamp`             | `HeadTimestamp` (88)         | 208 (historical)  |
| `decode_historical_data`            | `HistoricalData` (17)        | 208 (historical)  |
| `decode_historical_data_end`        | `HistoricalDataEnd` (108)    | 208 (historical)  |
| `decode_historical_data_update`     | `HistoricalDataUpdate` (90)  | 208 (historical)  |
| `decode_historical_schedule`        | `HistoricalSchedule` (106)   | 208 (historical)  |
| `decode_historical_ticks_bid_ask`   | `HistoricalTickBidAsk` (97)  | 208 (historical)  |
| `decode_historical_ticks_mid_point` | `HistoricalTick` (96)        | 208 (historical)  |
| `decode_historical_ticks_last`      | `HistoricalTickLast` (98)    | 208 (historical)  |
| `decode_histogram_data`             | `HistogramData` (89)         | 208 (historical)  |

## C# verification

`EDecoder.cs` dispatches purely on the 4-byte msg-id framing for all 9 cases —
no `if serverVersion >=` guards inside any historical case. Safe to remove text
branches without per-field version checks.

## Plan

### 1. Decoder edits in `src/market_data/historical/common/decoders/mod.rs`

Convert every decoder to the canonical proto-only shape used by `scanner` /
`news`:

```rust
// before
pub(crate) fn decode_historical_data(server_version: i32, time_zone: &Tz, message: &mut ResponseMessage) -> Result<HistoricalData, Error> {
    message.decode_proto_or_text(
        |bytes| { ... proto ... },
        |msg| { ... text ... },
    )
}

// after
pub(crate) fn decode_historical_data(message: &ResponseMessage) -> Result<HistoricalData, Error> {
    let bars = decode_historical_data_proto(message.require_proto()?)?;
    Ok(HistoricalData {
        start: OffsetDateTime::now_utc(),
        end: OffsetDateTime::now_utc(),
        bars,
    })
}
```

Receiver flips from `&mut ResponseMessage` to `&ResponseMessage` (matches
scanner/news; only `require_proto()` is needed on the message).

Signature simplifications to push through:

- `decode_head_timestamp` — drop `time_zone: Option<&Tz>` (proto carries unix
  seconds and resolves directly via `OffsetDateTime::from_unix_timestamp`)
- `decode_historical_data` — drop `server_version: i32` + `time_zone: &Tz` (the
  proto path doesn't use either; start/end come on a separate `HistoricalDataEnd`)
- `decode_historical_data_end` — drop `server_version: i32` + `time_zone: &Tz`
  (proto path uses `parse_date_with_tz` on the embedded TZ string)
- `decode_historical_data_update` — drop `time_zone: &Tz` (proto path resolves
  unix seconds to UTC; this matches the existing `decode_historical_data_bar`
  helper which ignores the time_zone arg)

`decode_historical_schedule`, the three `decode_historical_ticks_*`, and
`decode_histogram_data` already take just `message`, so only their bodies
collapse.

### 2. Caller signature updates

Three caller files:

- `src/market_data/historical/sync.rs` (5 callsites at lines 48 / 240 / 287 / 291 / 394)
- `src/market_data/historical/async.rs` (5 callsites at lines 60 / 262 / 308 / 312 / 414)
- `src/market_data/historical/mod.rs` (the `Update::decode` dispatcher at lines 394–656)

Drop the dropped args at the call sites:

```rust
// before
let mut data = decoders::decode_historical_data(client.server_version(), time_zone(client), &mut message)?;
let (start, end) = decoders::decode_historical_data_end(client.server_version(), time_zone(client), &mut end_msg)?;
let head = decoders::decode_head_timestamp(&mut message, self.time_zone())?;

// after
let mut data = decoders::decode_historical_data(&message)?;
let (start, end) = decoders::decode_historical_data_end(&end_msg)?;
let head = decoders::decode_head_timestamp(&message)?;
```

The `Update::decode` dispatcher in `historical/mod.rs:394` threads
`context.server_version` and `tz` into all three; after the conversion both
become dead in this match arm — audit other arms before deleting `tz` / `context`
plumbing entirely.

### 3. Dead helpers to remove from the decoder module

Once the text branches are gone, audit and delete:

- `parse_unix_seconds_str` — kept (used by `decode_head_timestamp` proto path)
- `parse_date` — likely dead (only used by text branches of `decode_historical_data` + `_end`)
- `parse_bar_date` — likely dead (only used by the text body of `decode_historical_data`/`_update`)
- `parse_schedule_date_time`, `parse_schedule_date`, `parse_time_zone` — kept
  (used by `decode_historical_schedule_proto` and `decode_historical_data_end_proto`)

Grep each helper after the body deletions; remove the ones with zero callers.
This also lets the time-zone plumbing through the sync/async clients drop where
it's only fed into deleted args (likely partial — `Subscription` listeners may
still need `time_zone` for `Update::decode` of other message types).

### 4. New testdata response builders

`src/testdata/builders/market_data.rs` currently has *request* builders for
historical (`HistoricalDataRequestBuilder` etc.) but **no response builders**.
Add 9 response shapes following the `AccountUpdateTimeResponse` / `ScannerData`
precedent — `ResponseProtoEncoder` impl over the corresponding `proto::*`
message, with field-minimal `Default` and a free-function entry point.

| Response struct                  | Wraps `proto::*`              | Msg id |
|----------------------------------|-------------------------------|-------:|
| `HeadTimestampResponse`          | `HeadTimestamp`               |     88 |
| `HistoricalDataResponse`         | `HistoricalData`              |     17 |
| `HistoricalDataEndResponse`      | `HistoricalDataEnd`           |    108 |
| `HistoricalDataUpdateResponse`   | `HistoricalDataUpdate`        |     90 |
| `HistoricalScheduleResponse`     | `HistoricalSchedule`          |    106 |
| `HistoricalTicksBidAskResponse`  | `HistoricalTicksBidAsk`       |     97 |
| `HistoricalTicksResponse`        | `HistoricalTicks`             |     96 |
| `HistoricalTicksLastResponse`    | `HistoricalTicksLast`         |     98 |
| `HistogramDataResponse`          | `HistogramData`               |     89 |

Document load-bearing `Default` values inline (e.g. `time_zone:
"US/Eastern".to_string()` because `decode_historical_schedule_proto` looks the
zone up via `parse_time_zone` — empty would error). Work backwards from the
existing test assertions in `historical/{sync,async}_tests.rs` and
`common/decoders/tests.rs` to know which fields each builder needs to expose
via `.setter(...)`.

Per rule 19, builders live in `src/testdata/builders/market_data.rs` (not under
`historical/common/`).

### 5. Test-fixture migration

Three test files, 43 text-fixture call sites total:

| File                                                     | `response_messages: vec![...]` count |
|----------------------------------------------------------|------------------------------------:|
| `src/market_data/historical/sync_tests.rs`               |                                  25 |
| `src/market_data/historical/async_tests.rs`              |                                  18 |
| `src/market_data/historical/common/decoders/tests.rs`    |        0 (uses `ResponseMessage::from`) |

For sync/async tests: convert each `MessageBusStub` text fixture to the
`ordered_responses: vec![proto_response(IncomingMessages::X, builder.encode_proto())]`
shape. Keep `text_response(...)` only for end-markers or cross-domain shared
decoders (none expected — all historical messages are gate 208).

For `common/decoders/tests.rs`: the existing tests build `ResponseMessage::from("17|...|")`
text payloads and call the decoder directly. Two options:

1. **Drive proto bytes through the production decoder** — `let bytes = builder.encode_proto(); let message = ResponseMessage::from_binary_text(IncomingMessages::HistoricalData, &bytes); decode_historical_data(&message)`.
2. **Add a `rejects_text_framing` test per decoder** — mirrors what scanner did
   in PR #532 (`decoders_tests.rs::test_decode_scanner_data_rejects_text_framing`)
   to lock in the proto-only contract.

Prefer (1) for the existing happy-path tests and add (2) per decoder as a thin
regression guard.

### 6. Tracker update

In `plans/legacy-text-protocol-cleanup.md`:

- Move historical row from "Floor 210 deletions unlocked" candidate list to the
  "shipped" list, with PR number
- Update the per-domain table — `market_data/historical/common/decoders/`
  goes from "8 text-decoders, 9 dual-format calls" to "0 / 9 / 0"
- Note that helpers `parse_date` / `parse_bar_date` were also deleted
- Note signature simplification: 4 decoders dropped `server_version` / `time_zone` args

## Sweep before opening the PR

```bash
cargo test                                         # default (async)
cargo test --features sync
cargo test --all-features
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features -- -D warnings
cargo fmt
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo build -p ibapi-integration-sync  --tests
cargo build -p ibapi-integration-async --tests
```

## Out of scope (next ratchet candidates after this PR)

- `decode_tick_news` text-branch deletion — leftover at floor 210 (gate 206
  `PROTOBUF_MARKET_DATA`); folds into a small news follow-up PR
- `decode_option_computation` text-branch deletion — leftover at floor 210
  (gate 206); shared between contracts and realtime, audit both call sites
- Floor ratchet 210 → 211 (`PROTOBUF_REST_MESSAGES_1`) — the next floor bump
  per `plans/legacy-text-protocol-cleanup.md`
