# Accounts text-decoder cleanup

Continues the per-family ratchet sweep started in PRs #529 / #531 / #532 / #534 /
#543. Floor is at 210 (`PROTOBUF_SCAN_DATA`). This PR deletes the now-unreachable
text branches in the accounts decoders **and** fixes a pre-existing proto-framing
bug in `decode_account_update_time`.

## Per-decoder gate analysis

Sourced from `/Users/wboayue/projects/tws-api/source/csharpclient/client/Constants.cs`
(`PROTOBUF_MSG_IDS` dictionary). The gate is on the **originating outgoing
request** — the response format mirrors the request format on the wire.

### In scope: gate ≤ 210 → delete text branch

| Decoder                          | Originating outgoing      | Gate            |
|----------------------------------|---------------------------|----------------:|
| `decode_position`                | `RequestPositions`        |  207 (accounts) |
| `decode_position_multi`          | `RequestPositionsMulti`   |  207 (accounts) |
| `decode_account_summary`         | `RequestAccountSummary`   |  207 (accounts) |
| `decode_account_value`           | `RequestAccountData`      |  207 (accounts) |
| `decode_account_portfolio_value` | `RequestAccountData`      |  207 (accounts) |
| `decode_account_update_time`     | `RequestAccountData`      |  207 (accounts) |
| `decode_account_multi_value`     | `RequestAccountUpdatesMulti` | 207 (accounts) |
| `decode_pnl`                     | `ReqPnL`                  |  210 (scan)     |
| `decode_pnl_single`              | `ReqPnLSingle`            |  210 (scan)     |

### Out of scope: gate > 210 → stays dual-format

| Decoder                          | Originating outgoing       | Gate            |
|----------------------------------|----------------------------|----------------:|
| `decode_managed_accounts`        | `RequestManagedAccounts` (207) + `StartApi` handshake | 213 |
| `decode_family_codes`            | `RequestFamilyCodes`       |  212 (REST 2)   |
| `decode_server_time`             | `RequestCurrentTime`       |  213 (REST 3)   |
| `decode_server_time_millis`      | `RequestCurrentTimeInMillis` | 213 (REST 3) |

`decode_managed_accounts` is dual-use: per-request gate is 207 but the
connection-layer handshake path travels via `StartApi` (gate 213). Below 213 the
client sends text-framed `StartApi`, so the server's `ManagedAccounts` push at
handshake is text-framed too. Stays dual-format until floor 213.

## C# verification

`EDecoder.cs` (lines 81-322) dispatches purely on the 4-byte msg-id framing
(`useProtoBuf` flag set by `Constants.PROTOBUF_MSG_ID` sentinel). No
`if serverVersion >=` guards within any in-scope case. Safe to remove text
branches without per-field version checks.

## Pre-existing bug surfaced: `decode_account_update_time`

The current implementation reads text fields directly without a
`decode_proto_or_text` wrapper:

```rust
pub(crate) fn decode_account_update_time(message: &mut ResponseMessage) -> Result<AccountUpdateTime, Error> {
    message.skip();           // message type
    message.skip();           // version
    Ok(AccountUpdateTime {
        timestamp: message.next_string()?,
    })
}
```

At floor 210, the gate for `RequestAccountData` is 207, so every
`AccountUpdateTime` push arrives proto-framed. The current decoder EOFs (rule 15
bug class). `proto::AccountUpdateTime { time_stamp: Option<String> }` already
exists at `src/proto/protobuf.rs:1591` — just needs a proto decoder and the
dispatch.

This PR bundles the fix (text branch deleted in the same change that adds the
proto decoder; no transient bug-fix-only commit needed since the deletion is
synchronous with the proto-only switch).

## Plan

### 1. Decoder edits in `src/accounts/common/decoders/mod.rs`

Drop the text closure from each in-scope `decode_proto_or_text` call:

```rust
// before
pub(crate) fn decode_position(message: &mut ResponseMessage) -> Result<Position, Error> {
    message.decode_proto_or_text(decode_position_proto, |msg| { ... text branch ... })
}

// after
pub(crate) fn decode_position(message: &mut ResponseMessage) -> Result<Position, Error> {
    message.require_proto()?;
    decode_position_proto(message.raw_bytes())
}
```

(Confirm the canonical proto-only call shape from prior cleanup PRs —
`message.require_proto()` + direct proto-decoder call, or the existing
`decode_proto_or_text` keeping an `unreachable!()` text branch. Match what
`decode_execution_data` / `decode_scanner_data` did in #529 / #532.)

Decoders to convert (9):
- `decode_position`
- `decode_position_multi`
- `decode_account_summary` — drop the unused `_server_version: i32` arg if its
  only purpose was text-path branching (the proto decoder doesn't take it)
- `decode_account_value`
- `decode_account_portfolio_value` — drop `_server_version: i32` if only used
  for the `version == 6 && server_version == 39` text branch
- `decode_account_multi_value`
- `decode_pnl` — drop `_server_version: i32`
- `decode_pnl_single` — drop `_server_version: i32`
- `decode_account_update_time` — **add** `decode_account_update_time_proto`,
  wire it via `require_proto()` (no `decode_proto_or_text` wrapper since text
  path is unreachable at floor 210)

Then update `decode_account_update_message` (the dispatch fn) to match the new
signatures.

### 2. New proto decoder: `decode_account_update_time_proto`

```rust
pub(crate) fn decode_account_update_time_proto(bytes: &[u8]) -> Result<AccountUpdateTime, Error> {
    let p = proto::AccountUpdateTime::decode(bytes)?;
    Ok(AccountUpdateTime {
        timestamp: p.time_stamp.unwrap_or_default(),
    })
}
```

### 3. Testdata builders

`src/testdata/builders/accounts.rs` and `src/testdata/builders/positions.rs`
already cover all 9 response shapes with `ResponseProtoEncoder` impls (verified
in audit). One gap: **no `AccountUpdateTimeResponse` builder** — add one
alongside `AccountDownloadEndResponse`. Shape:

```rust
pub struct AccountUpdateTimeResponse {
    pub timestamp: String,
}

impl Default for AccountUpdateTimeResponse {
    fn default() -> Self {
        Self { timestamp: "2026-05-22 14:20:00".to_string() }
    }
}
// + ResponseEncoder (msg "8") + ResponseProtoEncoder (proto::AccountUpdateTime)
// + entry-point fn `account_update_time()`
```

### 4. Test-fixture migration

3,113 lines of test code touch this surface across:
- `src/accounts/sync/tests.rs` (770 lines) — currently uses
  `MessageBusStub.response_messages: Vec<String>` + `encode_pipe()` text
- `src/accounts/async/tests.rs` (523 lines) — same shape
- `src/accounts/common/decoders/tests.rs` (1,301 lines) — direct decoder tests
- `src/accounts/common/stream_decoders/tests.rs` (519 lines) — StreamDecoder tests

Per rule 19, migrate:
- `text_response(builder.encode_pipe())` → `proto_response(IncomingMessages::X, builder.encode_proto())`
  for the 9 in-scope message types
- Keep `text_response(...)` for end-markers + cross-domain shared decoders if
  any (none expected here at floor 210)
- Verify post-`next_data()` assertions still execute (skip-classification per
  rule 15 means a missed conversion silently swallows the test)

Some tests use `create_test_client_with_responses(Vec<String>)` which only
supports text. Those callsites need to switch to the `ordered_responses`
constructor + `proto_response(...)` helper.

### 5. Tracker update

In `plans/legacy-text-protocol-cleanup.md`:
- Move accounts row from the "Floor 210 deletions unlocked" candidate list to
  the "shipped" list
- Add the 4 stays-dual-format accounts decoders to the explicit stays list
  (`decode_managed_accounts`, `decode_family_codes`, `decode_server_time`,
  `decode_server_time_millis`) — these were previously omitted from the table,
  which incorrectly suggested all of accounts was gate 207

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

## Out of scope (next ratchet candidates)

- `decode_family_codes` text-branch deletion — unlocks at floor 212 (REST_MESSAGES_2)
- `decode_server_time` / `decode_server_time_millis` — unlock at floor 213
- `decode_managed_accounts` handshake-text path — unlocks at floor 213
- Floor ratchet 210 → 211 (`PROTOBUF_REST_MESSAGES_1`) — unrelated to accounts
  but the next planned ratchet per `plans/legacy-text-protocol-cleanup.md`
