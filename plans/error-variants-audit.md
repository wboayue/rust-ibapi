# `Error::Simple` / `Error::Message` audit

**Status:** complete · **Last audited:** 2026-05-18 (PR-6 shipped) · **Parent:** [`plans/v3-api-ergonomics.md`](v3-api-ergonomics.md) §5 item 1

## Context

The parent v3.0 ergonomics tracker estimated ~42 untyped `Error::Simple` / `Error::Message` constructor sites concentrated in `src/messages.rs` and `src/errors.rs`. A fresh audit finds **~80 non-test sites across 27 files** — the parent estimate undercounted by ~2×, mostly because helpers in `src/common/error_helpers.rs` and decoder paths in the realtime / accounts / contracts / news / historical-data domains weren't in the original tally.

This doc breaks the sweep into 6 small PRs so each lands a single error category and stays reviewable. It is the working tracker for the audit and **lives** until the sweep ships.

## What's in scope

**In scope** (~80 sites): `Error::Simple(...)` and `Error::Message(...)` constructed in production code where downstream code would plausibly want to pattern-match (validation failure, EOF, protobuf decode, unexpected response, etc.).

**Out of scope / by design** — do not touch:

- `From<ResponseMessage> for Error` and `From<DecodedError> for Error` in `src/errors.rs:117,128` — canonical TWS error projection.
- `Clone` impl's `ParseTime → Simple` fallback at `src/errors.rs:158` — documented lossy collapse.
- Pattern-match arms in `src/client/error_handler/mod.rs:70,96` and `src/subscriptions/sync.rs:176` — consumers, not constructors.
- Test fixtures: `src/common/retry.rs` (4×), `src/common/test_utils.rs` (3×), `src/transport/async_memory.rs:109` (test reconnect stub).
- Doc-comments referencing `Error::Message` (e.g. `src/contracts/common/stream_decoders.rs`, `src/scanner/common/stream_decoders.rs`, `src/wsh/common/stream_decoders.rs`, `src/display_groups/common/stream_decoders.rs`).

## Site inventory

Production constructors only. Targets reference variants already on `Error` in `src/errors.rs`; no new variants proposed in this audit (the connection-rejected case is the one exception — see PR-6 below).

| Category | Sites | Files | Target variant |
|---|---:|---|---|
| Validation | 14 direct (+19 transitive via helpers) | `common/error_helpers.rs` ×7 helpers; `contracts/common/contract_builder/mod.rs` ×6; `contracts/builders.rs:516` ×1 | `Error::InvalidArgument` |
| EOF / no response | 8 | `accounts/{sync,async}/mod.rs` ×4; `orders/{sync,async}/mod.rs` ×2; `orders/builder/{sync,async}_impl.rs` ×2 | `Error::UnexpectedEndOfStream` |
| Server-version check | 1 | `client/async.rs:308` | `Error::ServerVersion` |
| Protobuf decode | 4 | `connection/common.rs:227,240`; `accounts/common/decoders/mod.rs:255,271` | `Error::ProtobufDecode` via `?` |
| Datetime / timestamp parse | 15 | `messages.rs:1342,1362,1365,1371,1375`; `accounts/common/decoders/mod.rs:257,263,273,278`; `news/common/decoders.rs:75,80`; `market_data/historical/common/decoders/mod.rs:27,28,352,368` | `Error::Parse` (or `Error::ParseTime` where source is `time::error::Parse`) |
| Cursor EOF (`peek_*`) | 13 | `messages.rs:1023,1036,1049,1057,1073,1092,1104,1120,1144,1151,1163,1174,1194` | `Error::Parse` (auto-promoted today via the `Parse(_,_,_) \| Simple(_)` wrapper at `messages.rs:1155`) |
| Message-type parse | 3 | `messages.rs:403,753,754` | `Error::Parse` or `Error::InvalidArgument` |
| Unexpected response / message | 12 | `contracts/sync/mod.rs:87,123,127,165,199`; `contracts/async/mod.rs:109,113,155,176,202`; `display_groups/common/decoders.rs:15`; `accounts/common/decoders/mod.rs:429`; `market_data/realtime/common/decoders/mod.rs:307,321` | `Error::UnexpectedResponse` (or the `Error::unexpected_response(message)` helper) |
| Decoder protocol mismatch | 6 | `market_data/realtime/common/decoders/mod.rs:134,137,158,161,181,184` (unexpected tick_type / missing `HistoricalTick*` in `TickByTickData`) | `Error::Parse` |
| Connection rejected | 2 | `connection/{sync,async}.rs:224,229` ("The server may be rejecting connections from this host") | `Error::ConnectionRejected(String)` (new variant, additive — see PR-6) |
| Internal invariant | 2 | `contracts/common/encoders.rs:98` (encoder dispatch fall-through); `messages.rs:921` (`is_protobuf == true` ⇒ `raw_bytes` should be `Some`) | `Error::InvalidArgument` or leave; low priority |
| Shared-channel dispatch | 1 | `transport/async.rs:717` ("No shared channel configured for message type") | `Error::InvalidArgument` (programmer error — message type not wired up) |

## Per-PR slicing

Ordering rationale: validation first (most repeated pattern, mechanical, lowest review burden); then EOF and decode and datetime (one variant per PR, each clean); cursor and unexpected-response last (more semantic judgment per site). The cross-cutting `Error::Parse` shape change (parent §5.3) is resolved with no-index constructors that land in PR-4 — see **Parse-shape decision (resolved)** below.

### PR-1: Validation → `Error::InvalidArgument` (~14 direct + 19 transitive) — shipped (#584)

- Flip the 9 helpers in `src/common/error_helpers.rs` (`require`, `require_with`, `require_request_id`, `require_request_id_for`, `require_range`, `require_not_empty`, `require_not_empty_vec`, `map_error`, `map_error_with`) to emit `Error::InvalidArgument` instead of `Error::Simple`. Cascades to every caller automatically.
- Convert 6 sites in `src/contracts/common/contract_builder/mod.rs:444-475` (builder validation: missing symbol / strike / expiration / contract month / negative strike).
- Convert 1 site in `src/contracts/builders.rs:516` (spread "must have at least one leg").
- Update `src/common/error_helpers.rs` unit tests' `matches!(... Error::Simple(_))` patterns to `Error::InvalidArgument(_)`.
- Also flipped 2 downstream consumer tests that pattern-matched `Error::Simple(_)` on the `require_request_id` path (`src/accounts/common/stream_decoders/tests.rs:63,413`) and the 7 `to_string()` assertions in `contracts/common/contract_builder/tests.rs` + 1 in `contracts/builders/tests.rs` (Display string prefix changed from `"error occurred:"` to `"InvalidArgument:"`).

**Verify:** `cargo clippy --all-targets -- -D warnings` (default + sync + all-features); `cargo test`.

### PR-2: EOF / no response → `Error::UnexpectedEndOfStream` (~8) — shipped (#585)

- `accounts/sync/mod.rs:35,48` and `accounts/async/mod.rs:368,382` ×4 "No response from server".
- `orders/sync/mod.rs:204` and `orders/async/mod.rs:91` ×2 "no response from server".
- `orders/builder/sync_impl.rs:46` and `orders/builder/async_impl.rs:50` ×2 "What-if analysis did not return order state" (same no-response shape).

Each is `or_else(|| Err(Error::Simple("...".into())))`-style; mechanical swap. Downstream test updates: `accounts/sync/tests.rs:223,766`, `accounts/{sync,async}/tests.rs` table-driven server-time arms (with `accounts/common/test_tables.rs` sentinel renamed `"No response from server"` → `"unexpected end of stream"`), and `orders/builder/{sync_impl,async_impl}/tests.rs` what-if assertions + parallel mock implementations.

### PR-3: Server version + protobuf decode (~5) — shipped (#586)

- `src/client/async.rs:308` — `Error::Simple(format!("Server version ... too old. ..."))` → `Error::ServerVersion(required, server, feature)`. The sync counterpart at `src/client/sync.rs:397` was already on `ServerVersion`; no production change needed there.
- `src/connection/common.rs:227,240` — `prost::Message::decode(...).map_err(|e| Error::Simple(...))` → `?` via the existing `From<prost::DecodeError>` impl on `Error`.
- `src/accounts/common/decoders/mod.rs:255,271` — same prost-decode shape.
- Internal-invariant additives: `src/contracts/common/encoders.rs:98` (encoder dispatch fall-through) and `src/messages.rs:921` ("missing protobuf bytes") both went to `Error::InvalidArgument` (programmer-error shape; preferred over `debug_assert!` so failures are diagnosable instead of panicking).
- Downstream test updates: `client/async_tests::check_server_version_branches` swapped `matches!(err, Error::Simple(_));` → `assert!(matches!(err, Error::ServerVersion(_,_,_)))`; `connection/common_tests` two `parse_account_info_*_protobuf_decode_error` cases swapped to `Error::ProtobufDecode(_)` (lost the contextual `"NextValidId"` / `"ManagedAccounts"` Display fragment — acceptable trade since the variant + file:line is enough for triage).
- /simplify follow-up cleanup (rule 9): upgraded the no-op `matches!(err, ...);` statements in `client/{sync,async}_tests::check_server_version_branches` and `create_order_update_subscription_is_unique` to `assert!(matches!(..))` — the previous form discarded the bool result.

Follow-ups surfaced by /simplify (deferred — see [feedback_simplify_deferral_rule_of_three](../../.claude/projects/-Users-wboayue-projects-rust-ibapi/memory/feedback_simplify_deferral_rule_of_three.md) once a 3rd occurrence appears):

- `Error::server_version(req, got, feature)` helper alongside the existing `Error::unexpected_response` factory at `src/errors.rs:144`. Would converge 4 call sites: `client/sync.rs:397`, `client/async.rs:308`, `protocol.rs:167`, `connection/common.rs:346`. Rule-of-three already met; land when the next consumer shows up or as a standalone cleanup.
- `test_parse_account_info_{next_valid_id,managed_accounts}_protobuf_decode_error` in `connection/common_tests.rs` now both assert only `Error::ProtobufDecode(_)`. Could be parameterized or one dropped — low priority.

### PR-4: Datetime / message-type parse → `Error::Parse` (~18) — shipped (#587)

Landed alongside the new `Error::parse_field` / `Error::parse_proto` constructors (resolution of parent §5.3 Option 4) so the fake-`0` index stays encapsulated.

- Added the constructors to `src/errors.rs` (impl block, sibling to `unexpected_response`). Shipped as `pub(crate)` rather than `pub` to keep the public surface minimal; promotion later is a one-line change. The bodies are identical (`Error::Parse(0, ...)`); the name distinguishes intent at the call site (`parse_field` = wire value, `parse_proto` = proto field name).
- 3× message-type parse in `src/messages.rs:403,753,754` ("Unknown incoming / Unknown outgoing / Invalid outgoing message type: …") → `Error::parse_field(s, "<reason>")`.
- 5× datetime parse in `src/messages.rs:1344,1364,1367,1373,1377` (line numbers shifted +2 post-#586) → `Error::parse_field(field, "<reason>")`. The `OffsetDateTime::from_unix_timestamp` arm and both `OffsetResult::{Ambiguous,None}` arms now carry the `field` value explicitly.
- 4× timestamp parse in `src/accounts/common/decoders/mod.rs:257,263,273,278` — proto path uses `Error::parse_proto("current_time" / "current_time_in_millis", e.to_string())`; text path uses `Error::parse_field(timestamp.to_string(), e.to_string())` / `Error::parse_field(millis.to_string(), ...)`.
- 2× news date parse in `src/news/common/decoders.rs::parse_unix_timestamp` — both `ParseInt` and `from_unix_timestamp` failures → `Error::parse_field(time, e.to_string())`. Also collapsed the trailing `match` into a single `.map_err` chain for clarity.
- 4× historical-data parse in `src/market_data/historical/common/decoders/mod.rs` (`parse_unix_seconds_str` ×2, `parse_date_with_tz`, `parse_bar_date`) → `Error::parse_field(text, "<reason>")`.
- No test fixture / pattern-match updates needed: cursor wrapper at `messages.rs:1155` already re-wraps `Simple|Parse → Parse(self.i, ..)`, and existing `Error::Parse | ParseInt | Simple` test-table arms still match.
- /simplify follow-up cleanup: collapsed the two identical `Error::parse_field(s, format!("invalid unix-second timestamp: {e}"))` closures in `parse_unix_seconds_str` into a shared `mk_err: &dyn Display -> Error` local (−1 line; Quality #4 + Efficiency #3 lens).

Follow-ups surfaced by /simplify (deferred):

- **Collapse `parse_field` + `parse_proto`** into a single `Error::parse(...)` factory — only 2 `parse_proto` callsites; the split is real (proto field name vs wire value) but thin. Reassess on the 3rd `parse_proto` consumer.
- **`impl Into<String>` → `impl Display`** on the constructors — would let callers drop `.to_string()` at `messages.rs:753`, `accounts/.../mod.rs:263,278`. Minor ergonomics; defer.
- **Shared `parse_unix_seconds(s)` / `parse_unix_millis(s)` helpers** in `src/proto/decoders.rs` — 3 callers exist (accounts, historical, news). Rule-of-three on the edge; land standalone or wait for 4th caller.

### PR-5: Cursor EOF + unexpected response + decoder mismatch (~31) — shipped (#589)

- 13× cursor `peek_*` / `next_*` EOF in `src/messages.rs:1023..1194` → `Error::Parse(i, "", "expected X and found end of message")` (honest typing; keeps the field index that earlier `Simple` lost). Removes the only `Simple` sources inside the cursor; the existing wrapper at `messages.rs:1155` still catches the `Parse | Simple` shape from `parse_ib_date_time_with_timezone`, so no wrapper change needed. The empty-string sibling at `messages.rs:1153` switched to `Error::parse_field("", "expected timestamp and found empty string")` for consistency.
- 7× unexpected-response sites that have a `ResponseMessage` in scope → `Error::unexpected_response(message)` helper: `contracts/sync/mod.rs:123,127`, `contracts/async/mod.rs:107,111` (matching_symbols Error / catch-all arms), `display_groups/common/decoders.rs:15`, `accounts/common/decoders/mod.rs:429` (catch-all on account-update dispatch — `other` pattern collapsed to `_` since the variant payload is now unused).
- 2× unexpected-response in `market_data/realtime/common/decoders/mod.rs:307,321` → `Error::UnexpectedResponse("missing market_depth_data".into())` (no `ResponseMessage` in scope; helper not applicable).
- 6× contract / option-computation None-channel sites → `Error::UnexpectedEndOfStream`. Reclassified from "unexpected response" because the structural cause is end-of-stream (subscription closed without any data), not a wrong response shape. Sites: `contracts/sync/mod.rs:87,165,199`, `contracts/async/mod.rs:155,176,202`.
- 6× decoder protocol-mismatch in `market_data/realtime/common/decoders/mod.rs:134,137,158,161,181,184` → `Error::parse_field(tick_type.to_string(), "Unexpected tick_type")` for the tick-type guard and `Error::parse_proto("tick", "missing HistoricalTick* in TickByTickData")` for the missing-variant guard. Test-asserted substrings ("Unexpected tick_type", "missing HistoricalTick*") preserved verbatim in the reason field.
- 1× shared-channel dispatch in `transport/async.rs:717` → `Error::InvalidArgument` (programmer-error wiring miss).

Downstream test updates: `contracts/sync/tests.rs` and `contracts/async/tests.rs` swapped 8 `Error::Simple(msg) ... assert!(msg.contains(...))` blocks to `assert!(matches!(err, Error::UnexpectedResponse(_) | Error::UnexpectedEndOfStream))` patterns; `transport/async_tests.rs::test_send_shared_request_unsupported_returns_error` swapped `Error::Simple(_)` → `Error::InvalidArgument(_)`; `display_groups/common/decoders.rs::test_decode_display_group_updated_wrong_message_type` swapped string-contains assertion for typed match. Tests in `market_data/realtime/common/decoders/tests.rs` continue to pass unchanged — their `err.to_string().contains(...)` assertions match the preserved substrings under both new variants' `Display` impls.

### PR-6: New `Error::ConnectionRejected(String)` variant (2 sites) — shipped (#590)

- Added `Error::ConnectionRejected(String)` to `src/errors.rs` with `#[error("connection rejected: {0}")]`; extended the manual `Clone` impl. Sits alongside the existing unit `ConnectionFailed` so callers can distinguish handshake-time rejection (allow-list mismatch) from generic connection failure without string-matching.
- Converted `src/connection/sync.rs:224` and `src/connection/async.rs:229` to `Error::ConnectionRejected(format!("server may be rejecting connections from this host: {err}"))`.
- Test updates: `connection/async_tests.rs::handshake_unexpected_eof_returns_rejection_simple_error` → renamed to `handshake_unexpected_eof_returns_connection_rejected`, swapped `Error::Simple` arm to `Error::ConnectionRejected`. Added a sync counterpart in `connection/sync_tests.rs` (no parallel test existed before; the sync handshake was previously untested at this layer).
- `docs/migration-3.0.md` got a paragraph documenting the new variant under the existing "Error" discussion (additive, non-breaking, but a v3.0 ergonomics win worth signaling).

## Parse-shape decision (resolved)

Decided 2026-05-17 — **Option 4**: keep `Error::Parse(usize, String, String)` shape unchanged; add `Error::parse_field(value, reason)` / `Error::parse_proto(field, reason)` constructors that internally pass `0`. Encapsulates the placeholder without changing the variant shape.

Why this over options 1–3 from parent §5.3:
- Non-breaking (no `Option<usize>` swap, no struct-variant churn, no match-arm rewrites).
- Future promotion to `Option<usize>` or struct variant remains possible — the helpers absorb that future change.
- The `0` lives in one place (`errors.rs`), not 25+ callsites; readers don't need to remember the convention because the constructor name documents it.

Tradeoff accepted: `Error::Parse(_, _, _)` pattern matches still work everywhere, but they don't distinguish text-protocol vs. proto-domain callers. That distinction wasn't load-bearing for any current consumer.

PR-4 lands the constructors. PR-5's cursor migrations also use them.

## Cross-cutting notes

- **`Error::Parse` wrapper at `messages.rs:1155`** already promotes `Simple(_)` → `Parse(self.i, field, msg)` for any cursor-call site. Cursor migrations (PR-5) and message-type-parse migrations (PR-4) should be aware of this — a `Simple` constructor at a cursor boundary is *not* observable to consumers as `Simple`.
- **`From<ValidationError>` mapping** at `src/errors.rs:187-205` already projects every `orders::builder::ValidationError` variant to `Error::InvalidArgument`. PR-1's validation push is consistent with this pre-existing convention.
- **Helper centralization**: `src/common/error_helpers.rs` has `#![allow(dead_code)]` because the helpers aren't fully adopted across the crate. PR-1 doesn't fix that, but adopting the helpers more widely would amplify the value of PR-1's switch (every additional caller automatically gets the right variant).

## Verification (per PR)

- `cargo clippy --all-targets -- -D warnings`
- `cargo clippy --all-targets --features sync -- -D warnings`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` after PR-3 (server-version variant carries a custom `#[error(...)]` rustdoc message).

No live-gateway test required — all sites are constructor-side; routing/decode paths are exercised by existing unit tests.

## Out of scope

- Adopting `error_helpers` helpers at additional callsites that today hand-roll `Error::Simple(format!(...))`.
- Promoting `Error::ConnectionFailed` from unit to `ConnectionFailed(String)`. PR-6 adds a sibling `ConnectionRejected(String)` variant instead.
- The `Error::Parse` variant-shape change (Option 1 / 2 from parent §5.3) — Option 4 (no-index constructors) is the resolution here; the variant tuple stays.
