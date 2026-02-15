# Implementation Plan: New TWS API Features

## 1. `reqCurrentTimeInMillis` / `currentTimeInMillis`

Server version: `CURRENT_TIME_IN_MILLIS` (197)

### Encoder (`src/accounts/common/encoders.rs`)
- Add `encode_request_server_time_millis()` — sends `OutgoingMessages::RequestCurrentTimeInMillis` with no additional fields (no version field, unlike `reqCurrentTime`)

### Shared channel (`src/messages/shared_channel_configuration.rs`)
- Add mapping: `RequestCurrentTimeInMillis` → `[CurrentTimeInMillis]`

### Response parsing (`src/accounts/async.rs`, `src/accounts/sync.rs`)
- Add `server_time_millis()` using `one_shot_with_retry` pattern (mirror `server_time()`)
- Decode: skip message type, read one `i64` field (milliseconds since epoch)
- Return `OffsetDateTime` via `from_unix_timestamp_nanos(millis * 1_000_000)`

### Client API (`src/client/async.rs`, `src/client/sync.rs`)
- Expose `server_time_millis() -> Result<OffsetDateTime, Error>`
- Gate on `server_versions::CURRENT_TIME_IN_MILLIS`

### `request_id_index` (`src/messages.rs`)
- No entry needed — `CurrentTimeInMillis` has no request ID (shared channel, like `CurrentTime`)

### Tests
- Encoder test in `src/accounts/common/encoders.rs`
- Decoder roundtrip test in async/sync modules

---

## 2. `cancelContractData`

Server version: `CANCEL_CONTRACT_DATA` (215)

### Encoder (`src/contracts/common/encoders.rs`)
- Add `encode_cancel_contract_data(request_id: i32)` — sends `OutgoingMessages::CancelContractData` + `request_id`

### Client API (`src/client/async.rs`, `src/client/sync.rs`)
- Expose `cancel_contract_details(request_id: i32) -> Result<(), Error>`
- Gate on `server_versions::CANCEL_CONTRACT_DATA`
- Fire-and-forget (no response expected)

### Tests
- Encoder test verifying message type + request_id fields

---

## 3. `cancelHistoricalTicks`

Server version: `CANCEL_CONTRACT_DATA` (215) — reuses same version gate in C#

### Encoder (`src/market_data/historical/common/encoders.rs`)
- Add `encode_cancel_historical_ticks(request_id: i32)` — sends `OutgoingMessages::CancelHistoricalTicks` + `request_id`

### Client API (`src/client/async.rs`, `src/client/sync.rs`)
- Expose `cancel_historical_ticks(request_id: i32) -> Result<(), Error>`
- Gate on `server_versions::CANCEL_CONTRACT_DATA`
- Fire-and-forget (no response expected)

### Tests
- Encoder test verifying message type + request_id fields

---

## 4. `HistoricalDataEnd` (incoming message 108)

Server version: `HISTORICAL_DATA_END` (196)

### `request_id_index` (`src/messages.rs`)
- Add `IncomingMessages::HistoricalDataEnd => Some(1)` — request ID is at index 1

### Response dispatch (`src/client/common.rs`)
- Register `HistoricalDataEnd` as a response for `RequestHistoricalData` alongside existing `HistoricalData`

### Response handling (`src/market_data/historical/async.rs`, `src/market_data/historical/sync.rs`)
- In `historical_data()`: add match arm for `HistoricalDataEnd` as end-of-stream signal
- Decode fields: `request_id` (i32), `start_date` (string), `end_date` (string)
- In `historical_data_streaming()`: add match arm for `HistoricalDataEnd` to signal stream completion

### Backward compatibility
- Keep existing `HistoricalData` (17) handling — older servers bundle bars + end marker in one message
- When server version >= `HISTORICAL_DATA_END`, the end marker arrives as a separate message (108)

### Tests
- Decoder test for `HistoricalDataEnd` message parsing
- Integration with existing historical data test infrastructure
