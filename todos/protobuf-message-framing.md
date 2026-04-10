# Plan: Protobuf Message Framing (Protobuf-Only)

## Context

The TWS API uses NUL-delimited text fields for message encoding up to server version 200. Starting at version 201, IB introduced protobuf-based message framing. The Rust client currently caps the negotiated version at 200 (`PARAMETRIZED_DAYS_OF_EXECUTIONS`).

**Design goal:** Drop text protocol support entirely. The crate will require server version 201+ (protobuf) and remove the legacy text-based encoding/decoding paths. This is a breaking change (major version bump) that dramatically simplifies the codebase — no dual-mode dispatch, no feature flags for protocol selection, no conditional encoding.

The crate already has:
- Generated prost types in `src/proto/protobuf.rs` (2556 lines, behind `proto` feature flag)
- Server version constants defined through v221 (`UPDATE_CONFIG`)
- A `tools/proto-gen` workspace member that fetches .proto files from IB's repo and generates Rust bindings

## Wire Format (Protobuf-Only)

**Outbound (client -> server):**
- Outer frame: 4-byte big-endian length prefix (unchanged)
- Message ID: 4-byte big-endian binary, offset by +200
- Payload: raw protobuf bytes

**Inbound (server -> client):**
- Outer frame: text-based length read (unchanged, per C# EReader)
- Message ID: 4-byte big-endian binary
- If message ID > 200: subtract 200, remaining bytes are protobuf
- If message ID <= 200: these are non-protobuf messages the server still sends in text format (e.g., during handshake or for messages not yet migrated to protobuf by IB). Handle as-is for the subset that occurs in practice.

## Design: Phased Approach

---

### Phase 1: Make `prost` a Required Dependency & Restructure Proto Module

**Goal:** Remove the `proto` feature gate. `prost` becomes a mandatory dependency.

**Changes:**
1. **`Cargo.toml`**
   - Move `prost` from optional to required dependency
   - Remove `proto` feature flag

2. **`src/proto/mod.rs`**
   - Remove `#[cfg(feature = "proto")]` gate from `src/lib.rs`
   - This module is now always available

3. **`src/lib.rs`**
   - Remove `#[cfg(feature = "proto")]` on `pub mod proto`

---

### Phase 2: Transport Layer — Binary Message ID Framing

**Goal:** Switch message ID reading/writing to binary format. Remove text-based message ID encoding.

**Files to modify:**

1. **`src/connection/common.rs`**
   - Set `min_version` to 201 (require protobuf-capable servers)
   - Set `max_version` to `server_versions::UPDATE_CONFIG` (221)
   - Rewrite `format_start_api()` — encode message ID as 4-byte big-endian binary + protobuf `StartApiRequest` payload

2. **`src/messages.rs`**
   - Add `PROTOBUF_MSG_ID: i32 = 200` constant
   - Replace `RequestMessage` (NUL-delimited fields) with a protobuf-aware type:
     - `encode()` produces: 4-byte BE message ID (with +200 offset) + serialized protobuf bytes
   - Replace `ResponseMessage` text field parsing with binary-aware parsing:
     - Read message ID as 4-byte big-endian int
     - Detect protobuf (id > 200) vs legacy (id <= 200)
     - Store raw payload bytes for protobuf messages
   - Remove `ToField` trait usage for message construction (no more NUL-delimited fields for outbound)
   - **Keep** `ResponseMessage` text parsing for the small set of messages the server may still send as text (handshake response, possibly some admin messages)

3. **`src/transport/sync.rs`** and **`src/transport/async.rs`**
   - Update message reading to use binary message ID extraction
   - Remove text-based message ID parsing path

4. **`src/transport/routing.rs`**
   - `determine_routing()` currently extracts request_id/order_id by peeking at text field positions. Replace with protobuf-aware extraction — deserialize enough of the protobuf payload to get routing fields (request_id, order_id).
   - Alternatively: define a lightweight "routing envelope" that extracts just the routing fields from raw protobuf bytes without full deserialization (prost partial decode or manual varint reading).

---

### Phase 3: Inbound Protobuf Decoding — Replace Text Decoders

**Goal:** Replace all text-based `decode_*` functions with protobuf decoders. Delete the old text decoders.

**New file: `src/proto/decoders.rs`**
- Shared conversion functions: `proto::Contract -> contracts::Contract`, `proto::Order -> orders::Order`, `proto::OrderState -> orders::OrderState`, etc.
- Pattern: field-by-field mapping using `.unwrap_or_default()` on prost `Option` fields

**Modify each domain module's `decoders.rs`:**
- Replace `decode_open_order(message: ResponseMessage)` with `decode_open_order(bytes: &[u8]) -> Result<OrderData>`
- The new function: `prost::Message::decode(bytes)` -> map proto fields to domain types -> return same domain struct
- **Delete** all text-based `next_int()` / `next_string()` / `next_double()` decoding chains

**Domain modules to update (9 decoder files):**
- `src/orders/common/decoders.rs` — OpenOrder, OrderStatus, ExecutionData, CompletedOrder, CommissionReport
- `src/contracts/common/decoders.rs` — ContractData, BondContractData
- `src/market_data/realtime/common/decoders.rs` — TickPrice, TickSize, TickString, TickGeneric, TickOptionComputation, TickByTick, MarketDepth
- `src/market_data/historical/common/decoders.rs` — HistoricalData, HistoricalTicks, RealTimeBars
- `src/accounts/common/decoders.rs` — AccountValue, PortfolioValue, Position, PnL
- `src/news/common/decoders.rs` — NewsBulletins, NewsArticle, HistoricalNews
- `src/scanner/common/decoders.rs` — ScannerData, ScannerParameters
- `src/display_groups/common/decoders.rs` — DisplayGroupUpdated
- `src/wsh/common/decoders.rs` — WshEventData, WshMetaData

---

### Phase 4: Outbound Protobuf Encoding — Replace Text Encoders

**Goal:** Replace all NUL-delimited text request encoding with protobuf serialization. Delete the old text encoders.

**New file: `src/proto/encoders.rs`**
- Shared conversion: domain types -> proto types (reverse of decoders)
- `contracts::Contract -> proto::Contract`, `orders::Order -> proto::Order`, etc.

**Modify each domain module's `encoders.rs`:**
- Replace text `RequestMessage` construction with proto object creation + `prost::Message::encode()`
- Example: `encode_place_order()` builds `proto::PlaceOrderRequest`, serializes to bytes
- **Delete** all `push_field()` / `ToField` chains

**Domain modules to update:**
- `src/orders/common/encoders.rs` — PlaceOrder, CancelOrder, RequestOpenOrders, RequestCompletedOrders, RequestExecutions
- `src/contracts/common/encoders.rs` — RequestContractData
- `src/market_data/realtime/common/encoders.rs` — RequestMarketData, RequestMarketDepth, RequestTickByTick
- `src/market_data/historical/common/encoders.rs` — RequestHistoricalData, RequestRealTimeBars, RequestHistoricalTicks
- `src/accounts/common/encoders.rs` — RequestAccountData, RequestPositions, RequestPnL
- `src/news/common/encoders.rs` — RequestNewsBulletins, RequestNewsArticle
- `src/scanner/common/encoders.rs` — RequestScannerSubscription, RequestScannerParameters
- `src/display_groups/common/encoders.rs` — QueryDisplayGroups
- `src/wsh/common/encoders.rs` — RequestWshEventData, RequestWshMetaData

---

### Phase 5: Cleanup

**Goal:** Remove dead code from the text protocol era.

- Delete `ToField` trait and implementations (used for NUL-delimited encoding)
- Delete `ResponseMessage` text field accessors (`next_int()`, `next_string()`, `next_double()`, etc.) — unless still needed for handshake
- Delete `request_id_index()`, `order_id_index()` lookup tables (routing now uses protobuf fields)
- Delete `shared_channel_configuration.rs` field-position mappings if routing is fully protobuf-aware
- Remove `OutgoingMessages` / `IncomingMessages` `FromStr` implementations (text parsing)
- Clean up `encode_length()` if no longer used for outbound
- Remove server version checks for versions < 201 throughout the codebase (dead code since min_version = 201)

---

## Key Design Decisions

1. **Protobuf-only, no dual mode:** Eliminates the complexity of maintaining two encoding paths. Breaking change — requires server version 201+ (IB Gateway 10.30+ / TWS 10.30+).

2. **Same routing infrastructure:** Message type IDs remain the same (subtract 200 from wire ID). The channel-based dispatch (`requests`, `orders`, `shared_channels`) is unchanged — only the payload format changes.

3. **Proto-to-domain conversion centralized in `src/proto/`:** Shared conversion functions (`proto::Contract -> Contract`) in `src/proto/decoders.rs` and `src/proto/encoders.rs`. Domain-specific decode/encode functions in each module call these shared converters.

4. **Incremental implementation:** Phases can be merged independently. Phase 1-2 (transport + framing) is the foundation. Phase 3-4 (decoders/encoders) can be done per-domain-module in separate PRs.

5. **`prost` becomes required:** No feature flag for protocol selection. Adds ~200KB to compile but eliminates conditional compilation complexity.

## Critical Files

| File | Role |
|------|------|
| `src/connection/common.rs` | Min/max version, handshake |
| `src/messages.rs` | Message types, framing, request/response types |
| `src/transport/sync.rs` | Sync message dispatch |
| `src/transport/async.rs` | Async message dispatch |
| `src/transport/routing.rs` | Message routing decisions |
| `src/proto/mod.rs` | Proto module entry |
| `src/proto/protobuf.rs` | Generated prost types |
| `src/proto/decoders.rs` | (new) Proto-to-domain conversion |
| `src/proto/encoders.rs` | (new) Domain-to-proto conversion |
| `src/*/common/decoders.rs` | Domain decoders (rewrite to protobuf) |
| `src/*/common/encoders.rs` | Domain encoders (rewrite to protobuf) |
| `Cargo.toml` | Remove proto feature, make prost required |

## C# Reference Files

| File | What to reference |
|------|-------------------|
| `EDecoder.cs` lines 27-325 | Inbound protobuf dispatch + detection |
| `EDecoderUtils.cs` | Proto-to-domain field mapping (primary reference for decoders) |
| `EClient.cs` | Outbound protobuf encoding pattern |
| `EClientUtils.cs` | Proto request creation (72 methods, primary reference for encoders) |
| `Constants.cs` lines 22-103 | Message ID to protobuf version mapping |
| `IBParamsList.cs` lines 21-35 | Binary message ID encoding |

## Verification

1. **Unit tests:** For each proto decoder/encoder, test with sample protobuf bytes and verify round-trip fidelity
2. **Clippy:** `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --features sync -- -D warnings`, `cargo clippy --all-features`
3. **Feature matrix:** `cargo build`, `cargo build --features sync`, `cargo build --all-features` all compile
4. **Integration tests:** Connect to IB Gateway (paper trading, 127.0.0.1:4002). Verify handshake negotiates v201+. Run existing integration test suite — all operations should work with protobuf encoding.
5. **Min version enforcement:** Verify connection fails gracefully with servers < v201
