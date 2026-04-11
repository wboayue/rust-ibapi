# Phase 2: Binary Message ID Framing

## Context

Currently, message IDs are text-encoded (NUL-delimited string fields). Server version 201+ uses 4-byte big-endian binary message IDs. Inbound messages with ID > 200 are protobuf; the real message type is `id - 200`. Outbound protobuf messages add +200 offset to the message ID.

**Depends on:** Phase 1 (prost is a required dependency)

## Wire Format

**Outbound:**
```
[4-byte BE length][4-byte BE message_id + 200][protobuf payload bytes]
```

**Inbound:**
```
[4-byte BE length][4-byte BE message_id][payload]
  if message_id > 200 → protobuf message, real type = message_id - 200
  if message_id <= 200 → legacy text message (handshake, some admin messages)
```

## Changes

### 1. `src/connection/common.rs`

**Bump version range (line 131-134):**
```rust
impl Default for ConnectionHandler {
    fn default() -> Self {
        Self {
            min_version: server_versions::PROTOBUF,     // 201
            max_version: server_versions::UPDATE_CONFIG, // 221
        }
    }
}
```

**Rewrite `format_start_api()` (line 162):**
- Encode message ID as 4-byte big-endian binary with +200 offset
- Serialize `proto::StartApiRequest` as protobuf payload
- C# reference: `EClient.cs` lines 193-213, `EClientUtils.cs:createStartApiRequestProto`

### 2. `src/messages.rs`

**Add constant:**
```rust
pub const PROTOBUF_MSG_ID: i32 = 200;
```

**Add `ResponseMessage` protobuf awareness:**
- Add `raw_bytes: Option<Vec<u8>>` field to store protobuf payload
- Add `is_protobuf: bool` flag
- Add `raw_bytes(&self) -> Option<&[u8]>` accessor
- When `is_protobuf` is true, the raw bytes contain the protobuf payload (everything after the 4-byte message ID)

**Add outbound protobuf encoding helper:**
```rust
/// Encode a protobuf outbound message: 4-byte BE (msg_id + 200) + proto bytes
pub fn encode_protobuf_message(msg_id: i32, proto_bytes: &[u8]) -> Vec<u8>
```

**Keep existing `RequestMessage` and `ResponseMessage` text paths** — still needed for handshake and messages not yet migrated. These will be removed in Phase 5.

### 3. `src/transport/sync.rs` and `src/transport/async.rs`

**Message reading:**
- After reading the 4-byte length and raw bytes, check server_version
- If server_version >= 201: read first 4 bytes as big-endian i32 message ID
- If message_id > 200: set `is_protobuf = true`, `message_type = message_id - 200`, store remaining bytes as `raw_bytes`
- If message_id <= 200: parse as text (existing path)

**Message writing:**
- Add method to write protobuf-framed messages (length prefix + binary message ID + proto bytes)

### 4. `src/transport/routing.rs`

**Update `determine_routing()` (line 23-72):**
- Currently extracts routing fields by peeking at text field positions via `message.message_type()`, `message.request_id()`, `message.order_id()`
- For protobuf messages, `message_type()` already works (set during framing)
- For `request_id()` and `order_id()`: need protobuf-aware extraction
- Option A: Partially decode protobuf bytes — most proto messages have `req_id` at tag 1 or `order_id` at tag 1. Use prost to decode just the first field.
- Option B: Keep `request_id_index()` / `order_id_index()` approach but map to protobuf field tags instead of text positions
- **Recommended:** Define a minimal `RoutingEnvelope` proto message with just `optional int32 req_id = 1` and attempt to decode it. If the first field is the ID, this works universally. For messages where it's not tag 1, handle by message type.

## Files Modified

| File | Change |
|------|--------|
| `src/connection/common.rs` | min_version=201, max_version=221, protobuf start_api |
| `src/messages.rs` | Add PROTOBUF_MSG_ID, raw_bytes field, protobuf encoding helper |
| `src/transport/sync.rs` | Binary message ID reading/writing |
| `src/transport/async.rs` | Binary message ID reading/writing |
| `src/transport/routing.rs` | Protobuf-aware routing field extraction |

## C# Reference

| File | Lines | What |
|------|-------|------|
| `EDecoder.cs` | 48 | `ReadRawInt()` for binary message ID |
| `EDecoder.cs` | 66-96 | Protobuf detection (id > 200, subtract 200) |
| `IBParamsList.cs` | 21-35 | Outbound binary message ID + offset encoding |
| `EClient.cs` | 193-213 | `startApiProtoBuf()` pattern |
| `EReader.cs` | 113-123 | Outer frame still uses text-based length |

## Verification

```bash
cargo build
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features
just test
```

Integration test: connect to IB Gateway paper (127.0.0.1:4002), verify handshake negotiates v201+ and connection succeeds.
