# `Error::Simple` / `Error::Message` audit

**Status:** in progress · **Last audited:** 2026-05-17 (against `main` past PR #582) · **Parent:** [`plans/v3-api-ergonomics.md`](v3-api-ergonomics.md) §5 item 1

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

### PR-1: Validation → `Error::InvalidArgument` (~14 direct + 19 transitive)

- Flip the 7 helpers in `src/common/error_helpers.rs` (`require`, `require_with`, `require_request_id`, `require_request_id_for`, `require_range`, `require_not_empty`, `require_not_empty_vec`, `map_error`, `map_error_with`) to emit `Error::InvalidArgument` instead of `Error::Simple`. Cascades to every caller automatically.
- Convert 6 sites in `src/contracts/common/contract_builder/mod.rs:444-475` (builder validation: missing symbol / strike / expiration / contract month / negative strike).
- Convert 1 site in `src/contracts/builders.rs:516` (spread "must have at least one leg").
- Update `src/common/error_helpers.rs` unit tests' `matches!(... Error::Simple(_))` patterns to `Error::InvalidArgument(_)`.

**Verify:** `cargo clippy --all-targets -- -D warnings` (default + sync + all-features); `cargo test`.

### PR-2: EOF / no response → `Error::UnexpectedEndOfStream` (~8)

- `accounts/sync/mod.rs:35,48` and `accounts/async/mod.rs:368,382` ×4 "No response from server".
- `orders/sync/mod.rs:204` and `orders/async/mod.rs:91` ×2 "no response from server".
- `orders/builder/sync_impl.rs:46` and `orders/builder/async_impl.rs:50` ×2 "What-if analysis did not return order state" (same no-response shape).

Each is `or_else(|| Err(Error::Simple("...".into())))`-style; mechanical swap.

### PR-3: Server version + protobuf decode (~5)

- `src/client/async.rs:308` — `Error::Simple(format!("Server version ... too old. ..."))` → `Error::ServerVersion(required, server, feature)`. Has a sync counterpart in `src/client/sync.rs` to align (the variant payload shape is identical for both).
- `src/connection/common.rs:227,240` — `prost::Message::decode(...).map_err(|e| Error::Simple(...))` → `?` via the existing `From<prost::DecodeError>` impl on `Error`.
- `src/accounts/common/decoders/mod.rs:255,271` — same prost-decode shape.
- One small additive: `src/contracts/common/encoders.rs:98` and `src/messages.rs:921` ("missing protobuf bytes") — both internal-invariant `unreachable`-ish; convert to `Error::InvalidArgument` or escalate with a `debug_assert!`. Decide at PR time.

### PR-4: Datetime / message-type parse → `Error::Parse` (~18)

Lands with the new `Error::parse_field` / `Error::parse_proto` constructors (see **Parse-shape decision** below) so the fake-`0` index stays encapsulated.

- Add the constructors to `src/errors.rs`:
  ```rust
  impl Error {
      pub fn parse_field(value: impl Into<String>, reason: impl Into<String>) -> Self {
          Error::Parse(0, value.into(), reason.into())
      }
      pub fn parse_proto(field: impl Into<String>, reason: impl Into<String>) -> Self {
          Error::Parse(0, field.into(), reason.into())
      }
  }
  ```
  Additive — non-breaking (`Error` is `#[non_exhaustive]`).
- 3× message-type parse in `src/messages.rs:403,753,754` ("Unknown incoming / Unknown outgoing / Invalid outgoing message type: …").
- 5× datetime parse in `src/messages.rs:1342,1362,1365,1371,1375`.
- 4× timestamp parse in `src/accounts/common/decoders/mod.rs:257,263,273,278` (`OffsetDateTime::from_unix_timestamp{,_nanos}().map_err`).
- 2× news date parse in `src/news/common/decoders.rs:75,80`.
- 4× historical-data parse in `src/market_data/historical/common/decoders/mod.rs:27,28,352,368`.

### PR-5: Cursor EOF + unexpected response + decoder mismatch (~31)

- 13× cursor `peek_*` EOF in `src/messages.rs:1023..1194`. These auto-promote through the existing `Parse(_,_,_) | Simple(_)` wrapper at `messages.rs:1155`, so they may already behave as Parse-typed at consumer-facing boundaries. Audit at PR time — either keep `Simple` as the placeholder caught-by-wrapper (and document the contract), or switch to `Error::UnexpectedEndOfStream` for honest typing.
- 12× unexpected-response sites in contracts sync/async (×10), `display_groups/common/decoders.rs:15`, `accounts/common/decoders/mod.rs:429`, `market_data/realtime/common/decoders/mod.rs:307,321` → prefer the `Error::unexpected_response(message)` helper when the source is a `ResponseMessage`; otherwise `Error::UnexpectedResponse(format!(...))`.
- 6× decoder protocol-mismatch in `market_data/realtime/common/decoders/mod.rs:134,137,158,161,181,184` → `Error::Parse(0, field, msg)`.
- 1× shared-channel dispatch in `transport/async.rs:717` → `Error::InvalidArgument` (programmer-error wiring miss).

### PR-6: New `Error::ConnectionRejected(String)` variant (2 sites)

Decided 2026-05-17 — add a new variant alongside the existing unit `ConnectionFailed`. Additive (non-breaking via `#[non_exhaustive]`); preserves the `io::Error` detail; lets downstream code match `ConnectionRejected` separately from generic `ConnectionFailed`.

- Add to `src/errors.rs`:
  ```rust
  #[error("connection rejected: {0}")]
  ConnectionRejected(String),
  ```
  Also extend the `Clone` impl. The `#[error(...)]` format keeps the hint verbatim — no leading "The server may be..." preamble; callers pass the full diagnostic string.
- Convert `src/connection/sync.rs:224` and `src/connection/async.rs:229` to `Error::ConnectionRejected(format!("server may be rejecting connections from this host: {err}"))`.
- Independent of PRs 1–5; can land in any order.

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
