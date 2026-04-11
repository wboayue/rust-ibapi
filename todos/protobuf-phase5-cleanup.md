# Phase 5: Cleanup — Remove Text Protocol Dead Code

## Context

After Phases 1-4, all encoding/decoding uses protobuf. The text protocol code is dead weight. Remove it to simplify the codebase.

**Depends on:** Phases 3 and 4 (all encoders/decoders migrated to protobuf)

## Removals

### `src/messages.rs`

**Delete `ToField` trait and implementations:**
- `ToField` trait (defined in `src/lib.rs` line 156-158)
- All `impl ToField for T` blocks throughout the codebase
- `push_field()` method on `RequestMessage`

**Delete `RequestMessage` text encoding:**
- `encode()` method (lines 779-783) that joins fields with NUL delimiters
- The `fields: Vec<String>` struct and its builder methods
- Keep only the protobuf-aware request encoding from Phase 2

**Delete `ResponseMessage` text accessors (if not needed for handshake):**
- `next_int()`, `next_string()`, `next_double()`, `next_bool()`, `next_long()`, `next_date_time()`, etc. (lines 919-1053)
- `next_optional_int()`, `next_optional_long()`, `next_optional_double()`
- `peek_int()`, `peek_string()` helpers
- `skip()` method
- **Exception:** Keep minimal text parsing if handshake response still uses text format. The handshake (`parse_handshake_response`) reads server_version and server_time as text — verify if this changes at v201+.

**Delete field index lookup tables:**
- `request_id_index()` (lines 408-463)
- `order_id_index()` (lines 399-405)
- These are replaced by protobuf-aware routing from Phase 2

**Delete `FromStr` for message enums:**
- `OutgoingMessages::from_str()` / `IncomingMessages::from_str()` — text parsing of message IDs no longer needed

### `src/messages/shared_channel_configuration.rs`

- Review if still needed. The channel mappings (request→response type pairings) may still be useful for the channel-based dispatch system even with protobuf. Only delete if routing is fully reworked.

### `src/lib.rs`

- Delete `ToField` trait definition (line 156-158)
- Delete all `impl ToField for ...` blocks

### Server Version Checks

**Remove dead version guards throughout codebase:**
- Any `if server_version >= X` where X < 201 is always true (since min_version = 201)
- Any `if server_version < X` where X <= 201 is always false (dead branch)
- Search pattern: `server_versions::` references to constants < 201
- **Be careful:** Some version checks guard fields that are still version-dependent above 201 (e.g., v202 `ZERO_STRIKE`, v214 `ADD_Z_SUFFIX_TO_UTC_DATE_TIME`). Only remove checks for versions < 201.

### `encode_length()` (`src/messages.rs` lines 750-758)

- Check if still used for outbound framing. If the outer 4-byte length prefix is now handled differently, delete it. If still used for the outer frame, keep it.

## Files Modified

| File | Change |
|------|--------|
| `src/messages.rs` | Delete text encoding/decoding, field accessors, index lookups |
| `src/lib.rs` | Delete `ToField` trait and impls |
| `src/messages/shared_channel_configuration.rs` | Delete if no longer needed |
| Various `*.rs` files | Remove dead `server_version < 201` checks |

## Approach

1. Start by deleting `ToField` and `push_field()` — compiler errors reveal any remaining text encoding call sites that were missed in Phase 4
2. Delete `ResponseMessage` text accessors — compiler errors reveal any remaining text decoding call sites missed in Phase 3
3. Delete index lookup functions — compiler errors reveal remaining text-based routing
4. Run `cargo clippy` to find unused code flagged by warnings
5. Search for `server_versions::` and audit each reference

## Verification

```bash
cargo build
cargo build --features sync
cargo build --all-features
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features
just test
```

Final integration test: full end-to-end against IB Gateway to confirm nothing was over-deleted.
