# Floor ratchet: 210 → 213 (final)

End-state of `plans/legacy-text-protocol-cleanup.md`. Raises the minimum
accepted server version from `PROTOBUF_SCAN_DATA` (210) to
`PROTOBUF_REST_MESSAGES_3` (213) in one bump, skipping 211 + 212.

After this lands, every gate in `Constants.cs::PROTOBUF_MSG_IDS` is at or below
the floor. The `decode_proto_or_text` machinery, `ResponseMessage::is_protobuf`
field, the inherent `ResponseMessage::from(&str)` constructor, and every
remaining text-decoder branch become structurally dead and can be deleted in
follow-ups.

## Prerequisites (verified 2026-05-25)

- **Live IB Gateway version**: paper-trading Gateway at `127.0.0.1:4002` reports
  `server_version = 220` (confirmed via `examples/sync/connect.rs`). The
  planned floor of 213 is well below this — no user lockout risk.
- **Examples**: `grep -rn "server_version" examples/` shows 14 hits, all
  `println!` formatting (no `< X` comparisons). No example needs updating.
- **Runtime routing for ⚪ "not implemented" message types**: defensive on
  both sides. `determine_routing` (routing.rs:124) catches unknown variants
  via `RoutingDecision::ByMessageType(_)`. Async dispatcher
  (`transport/async.rs:649`) does `channels.get(&message_type)` → no
  subscriber → silent drop. Sync dispatcher (`transport/sync/mod.rs:354-363`)
  falls through `requests` → `orders` → `shared_channels` → logs
  `info!("no recipient found")` and drops. A spontaneous `ReceiveFA` /
  `SoftDollarTiers` / etc. arrival would no-op safely — TWS doesn't push
  these unsolicited (all are request-response).

## Why one bump (not three)

Per rule 20 (CLAUDE.md): "Multi-gate ratchets are safe IFF every family in the
skipped range already has a proto decoder + `decode_proto_or_text` wrapper in
place." Direct audit confirms every family at gates 211/212/213 satisfies this
**after** the display_groups wiring in PR-A below. The 211 gate is itself a
no-op (only request-side changes — no incoming-decoder work), so 211 → 212 →
213 as separate bumps would just be churn.

Precedent: PR #530 did 203 → 210 in one move (skipping 6 gates) under the same
rule.

## Per-family audit (cross-checked from source)

| Family / Msg                              | Gate | Status in crate                                            |
|-------------------------------------------|-----:|------------------------------------------------------------|
| **211 (REST_MESSAGES_1)**                 |      |                                                            |
| ReceiveFA (16)                            |  211 | ⚪ not implemented (enum only in `messages.rs`)             |
| ReplaceFAEnd (103)                        |  211 | ⚪ not implemented (enum only)                              |
| ExerciseOptions response                  |  211 | OrderStatus / OpenOrder — proto-only since #539            |
| TickOptionComputation (21) — calc paths   |  206 | proto-only since #630                                      |
| **212 (REST_MESSAGES_2)**                 |      |                                                            |
| SecurityDefinitionOptionParameter (75)    |  212 | ✅ proto-only (`contracts/common/decoders/`)                |
| SecurityDefinitionOptionParameterEnd (76) |  212 | ✅ proto-only (option-chain end-of-stream)                  |
| SoftDollarTiers (77)                      |  212 | ⚪ not implemented                                          |
| FamilyCodes (78)                          |  212 | ⚠️ dual-format (`accounts/common/decoders/mod.rs:23`)       |
| SymbolSamples (79)                        |  212 | ✅ proto-only (`contracts/common/decoders/`)                |
| SmartComponents (82)                      |  212 | ⚪ not implemented                                          |
| MarketRule (93)                           |  212 | ✅ proto-only (`contracts/common/decoders/`)                |
| UserInfo (107)                            |  212 | ⚪ not implemented                                          |
| **213 (REST_MESSAGES_3)**                 |      |                                                            |
| NextValidId (9)                           |  213 | ⚠️ dual-format (`connection/common.rs:223` `is_protobuf`)    |
| ManagedAccounts (15)                      |  213 | ⚠️ dual-format (`connection/common.rs:235` + accounts:104)  |
| CurrentTime (49)                          |  213 | ⚠️ dual-format (`accounts/common/decoders/mod.rs:71`)       |
| CurrentTimeInMillis (109)                 |  213 | ⚠️ dual-format (`accounts/common/decoders/mod.rs:87`)       |
| MktDepthExchanges (80)                    |  213 | ⚠️ dual-format (`market_data/realtime/common/decoders/mod.rs:40`) |
| DisplayGroupList (67)                     |  213 | ⚪ not implemented (no decoder; query path absent)          |
| **DisplayGroupUpdated (68)**              |  213 | **❌ text-only with orphan proto decoder** — blocker, see PR-A |
| VerifyMessageApi (65) / VerifyCompleted (66) | 213 | ⚪ not implemented                                       |

⚪ "not implemented" = `IncomingMessages::*` variant exists in `src/messages.rs`
but no decoder function. Safe under bump because there's no decoder to crash —
unsolicited arrivals fall through `dispatch_unsolicited_message` which already
handles unknown message types defensively. If/when we add the client method,
the decoder must be proto-only from day one.

## C# verification

`EDecoder.cs` dispatches every 211/212/213 case purely on 4-byte msg-id framing
— no `if serverVersion >=` guards inside case bodies. Confirmed by spot-check
of `processReceiveFA`, `processSecurityDefinitionOptionalParameter`,
`processFamilyCodes`, `processNextValidId`, `processDisplayGroupUpdated`,
`processMarketDepthExchanges`. Same precondition that justified the 203 → 210
skip in #530.

## PR sequence

### PR-A — display_groups: wire the orphan proto decoder

**Status: ✅ Shipped in [#631](https://github.com/wboayue/rust-ibapi/pull/631)** (merged 2026-05-25).

The single blocker. `decode_display_group_updated` at
`src/display_groups/common/decoders.rs:12` reads `peek_string(3)` directly
with no proto branch. The proto decoder `decode_display_group_updated_proto`
already exists at line 28 of the same file, marked `#[allow(dead_code)]`. Wire
it through `decode_proto_or_text`.

**Deviations from the original plan**: (a) receiver stayed `&mut ResponseMessage`
(not `&ResponseMessage` as planned) — `decode_proto_or_text` takes `&mut self`,
so no flip. The receiver flip will happen in PR-C5 when we collapse to
`require_proto()`. (b) /simplify swapped the manual `from_protobuf` test
fixture for the existing `proto_response()` helper at `src/common/test_utils.rs:127`
and dropped the unused `req_id` field.

Post-bump failure mode if skipped: every `DisplayGroupUpdate` returns an empty
`contract_info` string (the `peek_string(3).unwrap_or_else(|_| ...)` path
returns `""` for proto-framed messages because `peek_string` is not
proto-aware on the legacy index path — see rule 17 for the bug-class
backstop). No panic; just silently-wrong data.

#### Changes

```rust
// src/display_groups/common/decoders.rs
pub(crate) fn decode_display_group_updated(message: &ResponseMessage) -> Result<DisplayGroupUpdate, Error> {
    if message.message_type() != IncomingMessages::DisplayGroupUpdated {
        return Err(Error::unexpected_response(message));
    }
    message.decode_proto_or_text(
        decode_display_group_updated_proto,
        |msg| {
            let contract_info = msg.peek_string(3).unwrap_or_else(|_| {
                warn!("DisplayGroupUpdated message has fewer fields than expected (len={})", msg.len());
                String::new()
            });
            Ok(DisplayGroupUpdate::new(contract_info))
        },
    )
}
```

Receiver flips `&mut ResponseMessage` → `&ResponseMessage` (the dual-format
helper takes `&self`). Drop `#[allow(dead_code)]` on
`decode_display_group_updated_proto` (now wired).

Caller: `src/display_groups/common/stream_decoders.rs` — the dispatcher
already calls `decode_display_group_updated(...)`; update for the receiver
flip if needed.

#### Tests

Add `test_decode_display_group_updated_dispatches_proto` to the existing test
block:

```rust
#[test]
fn test_decode_display_group_updated_dispatches_proto() {
    use prost::Message;
    let proto_msg = crate::proto::DisplayGroupUpdated {
        req_id: Some(1),
        contract_info: Some("265598@SMART".into()),
    };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();
    let message = ResponseMessage::from_binary_text(IncomingMessages::DisplayGroupUpdated, &bytes);

    let result = decode_display_group_updated(&message).expect("decoding failed");
    assert_eq!(result.contract_info, "265598@SMART");
}
```

Keep the existing text-path tests — they exercise the legacy branch which
stays alive until floor 213 (will be deleted in PR-C5 below).

### PR-B — the ratchet

**Status: 🚧 Open in [#632](https://github.com/wboayue/rust-ibapi/pull/632)** (pushed 2026-05-25, awaiting merge).

Mechanical. Three edits + a doc sweep.

**Actual diff scope** (12 + 11 files, two commits):
- 4 sites in `connection/common.rs` (the load-bearing bump)
- ~30 test-fixture sites (`SERVER_VERSION` consts in connection/client tests;
  `PROTOBUF_HISTORICAL_DATA` → `PROTOBUF_REST_MESSAGES_3` in historical +
  realtime tests; 17 hardcoded `"v210..221"` mock handshake strings in
  `transport/sync/tests.rs`)
- /simplify follow-up: 6 scanner fixtures + 4 `_rejects_text_framing` tests
  (contracts/scanner/orders) + the `proto_response()` helper default +
  decoder doc-comments (drop the inline `(PROTOBUF_SCAN_DATA = 210)`
  parentheticals so comments don't rot at every bump)
- Renamed `test_require_protobuf_support_rejects_previous_place_order_floor`
  → `_rejects_previous_scan_data_floor` per the per-bump-rename convention
- `docs/migration-3.0.md` floor reference updated; SLONG note reverted to
  "well below our floor" (no version pin)

#### 1. Bump the constant

`src/connection/common.rs:340`:

```rust
// before
pub(crate) fn require_protobuf_support(server_version: i32) -> Result<(), Error> {
    if server_version < server_versions::PROTOBUF_SCAN_DATA {
        ...

// after
pub(crate) fn require_protobuf_support(server_version: i32) -> Result<(), Error> {
    if server_version < server_versions::PROTOBUF_REST_MESSAGES_3 {
        ...
```

Same constant swap on line 342 + 346 (the error message and connection-info
formatting).

#### 2. Bump the handshake offer range

`src/connection/common.rs:171` — `ConnectionHandler::default()` sets
`min_version: server_versions::PROTOBUF_SCAN_DATA` (this is the lower bound
advertised in the handshake `v<min>..<max>` string, NOT a `Features::*`
entry — there is no Features table at this site). Flip to
`PROTOBUF_REST_MESSAGES_3`. `max_version` is `server_versions::UPDATE_CONFIG`
and unrelated to the floor — leave it.

#### 3. Sync test-fixture server versions

```bash
# Bulk find
grep -rn "server_versions::PROTOBUF_SCAN_DATA\|server_versions::PROTOBUF_HISTORICAL_DATA" src/
```

Identified callsites (per audit grep):

- `src/connection/{sync,async,common}_tests.rs` — `SERVER_VERSION` const +
  `too_old = PROTOBUF_SCAN_DATA - 1` boundary checks. Per rule 21, derive
  expectations from the constant under test — these tests use `PROTOBUF_SCAN_DATA`
  to mean "current floor"; flip to `PROTOBUF_REST_MESSAGES_3` so `too_old` and
  `actual` boundary cases still pin to floor-minus-one. The test
  `accepts_at_or_above_floor`-style cases that assert specific protobuf gate
  values need rewording (the assertion `assert_eq!(required, ...)` should
  reflect the new floor).
- `src/connection/common_tests.rs:375` — `PROTOBUF_PLACE_ORDER` (203) is below
  the new floor; the test name (`rejects_below_floor`) stays correct but the
  values it asserts shift. Re-read the test body, derive every assertion from
  `server_versions::PROTOBUF_REST_MESSAGES_3` per rule 21.
- `src/market_data/historical/{sync,async}_tests.rs` (15 fixtures) and
  `src/market_data/realtime/common/decoders/tests.rs:43, 124` — use
  `PROTOBUF_HISTORICAL_DATA` (208). These pre-date the 210 bump. Flip to
  `PROTOBUF_REST_MESSAGES_3` (213) to align with the new floor. The decoder
  paths exercised are all proto-only at floor 210 already, so the server
  version is decorative in those tests — bumping is a one-token replace_all.

The full sweep (single `replace_all` per file):

```bash
# Conceptually — read each file first per Edit tool rules
sed -i '' 's/PROTOBUF_HISTORICAL_DATA/PROTOBUF_REST_MESSAGES_3/g' \
  src/market_data/historical/sync_tests.rs \
  src/market_data/historical/async_tests.rs \
  src/market_data/realtime/common/decoders/tests.rs
```

(Don't actually use `sed`; use `Edit` with `replace_all` per project tooling.)

For `connection/*_tests.rs`, the change is asymmetric: `SERVER_VERSION` const
flips, boundary literals shift, but `PROTOBUF_PLACE_ORDER` references stay (they
test "well below the floor" semantics). Read each test and decide per-site.

#### 4. Update the tracker

`plans/legacy-text-protocol-cleanup.md`:

- §"Status today" → "Connection gate: floor ratcheted 210 → 213 in [#XXX]
  (skipping 211 + 212 in one move — every family in that range had proto
  decoder + dual-format wrapper or no decoder at all)."
- §"Per-family protobuf-incoming gates" — all rows now at or below floor;
  mark the final state.
- §"Decoders that stay dual-format at floor 210" — drop the entire list (all
  unlocked under 213).
- §"Helper APIs that go away" — these are now structurally dead; flag for
  PR-D below.
- Add §"Floor 213 deletions (unlocked, follow-up PRs)" listing the per-decoder
  cleanups in PR-C below.

#### 5. Docs

Grep for "210" / "PROTOBUF_SCAN_DATA" in `README.md`, `docs/*.md`, and rustdoc
comments. Any "minimum server version 210" claims flip to 213. Per the
`.md code blocks rot silently` memory, also mentally compile-pass any nearby
code blocks.

#### Sweep

```bash
cargo test                                         # default (async)
cargo test --features sync
cargo test --all-features
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features -- -D warnings
cargo fmt
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo build -p ibapi-integration-sync  --tests
cargo build -p ibapi-integration-async --tests
```

### PR-C — per-decoder text-branch deletions

**Status: 🚧 In progress** — PR-B merged 2026-05-25; C1 shipped in [#634](https://github.com/wboayue/rust-ibapi/pull/634); C6 shipped in [#633](https://github.com/wboayue/rust-ibapi/pull/633).

Six follow-up PRs after PR-B. Each is a thin proto-only conversion + fixture
migration following the gate-206 / historical precedent (PRs #626, #629, #630).

**Dependency order:** PR-A → PR-B → {C1, C2, C3, C4, C6 parallel} → C5.
C5 specifically depends on PR-B because it collapses the dual-format wrapper
PR-A introduced — that wrapper must stay alive until the floor bump lands.
C1/C2/C3/C4/C6 are already-dual-format → proto-only, all independent of each
other.

Per rule 19: response builder lives in `src/testdata/builders/<domain>.rs`;
field-minimal `ResponseProtoEncoder` impl; migrate fixtures from
`text_response(builder.encode_pipe())` → `proto_response(IncomingMessages::X, builder.encode_proto())`;
add `_rejects_text_framing` regression test per decoder.

| PR    | Decoder(s)                                              | Domain                          | New builder(s) needed                  |
|-------|---------------------------------------------------------|---------------------------------|----------------------------------------|
| ~~C1~~ | ~~`decode_family_codes`~~ — shipped in [#634](https://github.com/wboayue/rust-ibapi/pull/634) | `accounts/common/decoders/`     | n/a (builder existed)                  |
| ~~C2~~ | ~~`decode_server_time`, `decode_server_time_millis`~~ — shipped in this PR | `accounts/common/decoders/`     | n/a (builders existed)                 |
| C3    | `decode_managed_accounts`                               | `accounts/common/decoders/`     | `ManagedAccountsResponse`              |
| C4    | `decode_market_depth_exchanges`                         | `market_data/realtime/common/decoders/` | `MktDepthExchangesResponse`    |
| C5    | `decode_display_group_updated` (collapse PR-A wrapper)  | `display_groups/common/`        | `DisplayGroupUpdatedResponse`          |
| ~~C6~~ | ~~`NextValidId` / `ManagedAccounts` `is_protobuf` branches in `connection/common.rs`~~ — shipped in [#633](https://github.com/wboayue/rust-ibapi/pull/633) | `connection/` | n/a (delete-only)             |

Each PR also:

1. Flips the decoder receiver `&mut ResponseMessage` → `&ResponseMessage`
   (matches scanner/news shape post-cleanup).
2. Drops any text-only helper functions the deleted branch was using
   (grep for callers after the delete).
3. Updates the tracker — moves the row from "Floor 213 deletions unlocked"
   → "Floor 213 deletions shipped" with PR number.

**Touchpoint: `transport/routing.rs`** — C2 (`CurrentTime`), C3
(`ManagedAccounts`), C6 (`NextValidId` + `ManagedAccounts`) all hit message
types that route through the `is_shared_message` special-case at
`src/transport/routing.rs:89-94`. Grep `routing.rs` for the message-type name
before opening each PR to confirm the special-case dispatch still composes
with the proto-only decoder.

C6 is the smallest (~10 lines). C1-C5 each follow the same shape and take
roughly one PR-A's worth of work.

### PR-D — final cleanup (after all C-series PRs ship)

**Status: 📋 Pending** — unblocks once all PR-C series merge.

Delete the dual-format machinery and text-only `ResponseMessage` surface.
Sequenced because some deletions block others.

**D1 — collapse caller branches (independent of D2/D3).** Each site reads
`is_protobuf` and forks; after C-series, the text arm is unreachable.
- `From<&ResponseMessage> for Notice` proto branch → collapse
- `From<ResponseMessage> for Error` proto branch (`src/errors.rs:116`) → collapse
- `src/transport/routing.rs:104` proto/text error dispatch → collapse to
  `decode_error_envelope(message.raw_bytes()?)` only
- `decode_proto_or_text{,_owned}` callsites: by C-series end, every call has
  the text closure as `|_| unreachable!()`. Inline the proto closure at each
  callsite.
- `connection::common::parse_raw_message` text-payload `else` branch
  (lines 384-389): per the tracker, already unreachable at floor 203; delete
  while we're in there. The binary-text-payload branch (lines 377-383) goes
  in D3.

**D2 — collapse proto-aware accessors (depends on D1).** Per rule 17 the
`peek_*` / `request_id` / `order_id` / `execution_id` accessors have
`is_protobuf` branches. Once every caller is proto-only:
- `peek_int` / `peek_long` / `peek_string` simplify to direct
  `self.fields[i]` (and `fields` shrinks to `[msg_id]` only — or goes away
  entirely)
- `request_id` / `order_id` / `execution_id` collapse to proto-envelope
  decode only (the minimal `ProtoIdEnvelope` / `ExecutionDetailsMinimal`
  patterns from rule 17 stay; their text-index fallback goes)
- All `message.skip()` calls in remaining decoders disappear with the field

**D3 — delete the dual-format helpers + collapse `ResponseMessage` (depends
on D1+D2).** With no callers left:
- `messages::ResponseMessage::is_protobuf` field
- `messages::ResponseMessage::from(fields: &str)` inherent constructor
  (audit `stubs.rs:99` and any remaining test-fixture callers; replace with
  `from_binary_text` equivalent or proto-builder fixtures)
- `messages::ResponseMessage::from_binary_text` (after `stubs.rs` migration)
- `messages::ResponseMessage::with_server_version`
- `messages::ResponseMessage::decode_proto_or_text{,_owned}`
- `connection::common::parse_raw_message` text branch (full)

`ResponseMessage` post-D3 is a thin `(IncomingMessages, Bytes)` carrier — at
that point consider whether to flatten it onto `RoutedItem` directly (out of
scope, see end of file).

All three are crate-internal (`ResponseMessage` is `pub(crate)` since PR #581
per memory `feedback_narrowing_transparency_audit`). No major-version bump
needed unless an audit finds a leaked public reference. The grep before D3:
`grep -rn "pub.*ResponseMessage\|pub.*is_protobuf\|pub.*from_binary_text" src/`
must return zero non-impl hits.

D1 and D2 ship as separate PRs (cleanly diff-able, independent test surfaces).
D3 ships last as a single PR to land the field/helper deletions atomically —
splitting D3 would leave the crate in a half-collapsed intermediate state.

## Open questions / risks

1. **`from_binary_text` is used in the PR-A test fixture.** Intentional —
   constructs a `ResponseMessage` from proto bytes for testing. After D3
   this helper goes away; the PR-A test (and any other testdata-using
   fixtures) need rewriting to use a `MessageBusStub` + `proto_response(...)`
   shape. Tracked in D3's `stubs.rs` migration note.

## /simplify follow-ups (deferred from per-PR review)

- **`create_test_client_with_ordered_proto_responses` helper.** Each C-series
  PR is adding `Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(...)])) + Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES)`
  to replace the deleted `create_*_test_client_with_responses(vec![..encode_pipe()])`
  shape. PR-C1 added 4 such blocks (sync + async × scenarios 1 & 3). PR-C2/C3
  will add more. By rule-of-three, land a shared
  `create_test_client_with_ordered_proto_responses(Vec<ResponseMessage>) → (Client, Arc<MessageBusStub>)`
  helper in `src/common/test_utils.rs` alongside (or just before) PR-C3 and
  fold the new C-series sites onto it. Don't sweep pre-existing manual
  setups (`test_positions`, `test_account_updates`, etc.) in the same PR —
  that's a separate consistency pass.
- **`one_shot_request` processor signature `Fn(&mut ResponseMessage)` → `Fn(&ResponseMessage)`.**
  C-series proto-only flips wrap the decoder in `|msg| decoders::decode_X(msg)`
  at the callsite because the helper's processor sig didn't change. After
  PR-C3 lands (3rd occurrence: family_codes, server_time, managed_accounts),
  flip the helper signature in a follow-up PR and drop the closure wrappers.
  Wsh's `one_shot_request_with_retry` decoders still use `&mut` (`message.clone()`
  pattern), so leave that helper untouched.

## Out of scope (after PR-D)

- Replace `ResponseMessage` with `prost::Message` directly on the routing
  channels. Currently `RoutedItem` carries `ResponseMessage`; post-PR-D this
  is just a `(IncomingMessages, Bytes)` carrier and could be flattened.
  Separate refactor PR; not blocking.

- Delete `IncomingMessages` variants for never-implemented responses
  (ReceiveFA, ReplaceFAEnd, SoftDollarTiers, SmartComponents, UserInfo,
  VerifyMessageApi, VerifyCompleted, DisplayGroupList). These exist as enum
  variants but the crate doesn't decode any of them. Removal is a public-API
  breaking change (the enum is `pub`), so defer to a separate v3.x cleanup PR
  with explicit migration note.

## Source of truth

`/Users/wboayue/projects/tws-api/source/csharpclient/client/Constants.cs`
(`PROTOBUF_MSG_IDS`) and `EDecoder.cs` for per-case version-gate verification.
