# Plan: Test Input/Output Builders

## Context

Tests across the crate build inputs (scripted responses) and verify outputs (captured requests) with raw, untyped primitives that scale poorly:

- **Response messages are 40–95 inline fields** of pipe-delimited literals like `"61|3|DU1234567|76792991|TSLA|STK||0.0|||NASDAQ|USD|TSLA|NMS|500|196.77|"`. No field names, easy to mis-position, painful to read or change.
- **Request assertions only check message type ID** via `assert_proto_msg_id` — they do not verify request content (account, quantity, contract id, etc.).
- **`.replace('|', "\0")` is duplicated 6 times** in `src/stubs.rs` (lines 112, 158, 181, 194, 207, 240) — a sign the conversion lives at the wrong layer.
- **Sync and async tests duplicate response literals** verbatim in parallel files (`src/{accounts,orders,…}/{sync,async}/tests.rs`).
- **A separate plan** (`todos/eliminate-mock-gateway.md`) replaces `MockGateway` with a `MemoryStream` fixture. That work and the existing 100+ `MessageBusStub` tests both benefit from the same typed builders, so the foundation lands first.

The intended outcome is: tests construct responses by name with sensible defaults, assert request content by field, and share data tables across sync/async — without changing any production code.

## Approach

### 1. Response builders (`src/testdata/builders/`)

Per-message typed builder structs with named-field setters and `Default` impls populated from `src/common/test_utils.rs::constants`. Output methods produce all three formats consumers need:

```rust
// src/testdata/builders/positions.rs
#[derive(Clone)]
pub struct PositionResponse {
    pub account: String,
    pub contract_id: i32,
    pub symbol: String,
    pub position: f64,
    pub avg_cost: f64,
    // … all wire fields
}

impl Default for PositionResponse { /* TEST_ACCOUNT, TEST_CONTRACT_ID, TSLA, … */ }

impl PositionResponse {
    pub fn account(mut self, v: impl Into<String>) -> Self { self.account = v.into(); self }
    pub fn contract_id(mut self, v: i32) -> Self { self.contract_id = v; self }
    // … one fluent setter per field

    pub fn encode_pipe(&self) -> String { /* "61|3|account|…|" */ }
    pub fn encode_null(&self) -> String { /* "61\03\0account\0…\0" */ }
    pub fn encode_length_prefixed(&self) -> Vec<u8> { encode_raw_length(self.encode_null().as_bytes()) }
}

pub fn position() -> PositionResponse { PositionResponse::default() }
pub fn position_end() -> PositionEndResponse { PositionEndResponse::default() }
```

Call site:
```rust
let responses = vec![
    positions::position().symbol("AAPL").contract_id(265598).position(100.0).encode_pipe(),
    positions::position_end().encode_pipe(),
];
let (client, bus) = create_test_client_with_responses(responses);
```

`encode_pipe()` for current `MessageBusStub` consumers; `encode_null()` and `encode_length_prefixed()` for the upcoming `MemoryStream` tests in the eliminate-mock-gateway work — same builders serve both.

### 2. Request assertions

Add a content-aware assertion to `src/common/test_utils.rs::helpers` that decodes the protobuf and compares full structure:

```rust
pub fn assert_request_proto<T: prost::Message + Default + PartialEq + std::fmt::Debug>(
    bus: &MessageBusStub,
    index: usize,
    expected_msg_id: OutgoingMessages,
    expected: &T,
) {
    assert_request_msg_id(bus, index, expected_msg_id);
    let actual: T = decode_request_proto(bus, index);
    assert_eq!(&actual, expected, "request {index} body mismatch");
}
```

Reuses existing `decode_request_proto::<T>` and `assert_request_msg_id`. Tests express expectations as protobuf structs (already generated under `src/proto/protobuf.rs`), avoiding manual byte fiddling.

### 3. Consolidate `.replace('|', "\0")` in `MessageBusStub`

Convert at intake, eliminating the 6 duplicate call sites. Two options, pick one:

- **Option A**: pre-convert at `with_responses` time, keep `Vec<String>` storage in NUL form.
- **Option B**: store as `Vec<ResponseMessage>` directly; trait methods clone instead of re-parsing.

Option A is the smaller change; Option B is cleaner but moves more code. Default to Option A unless it surfaces issues.

### 4. Shared sync/async test tables (per domain, opportunistic)

`src/accounts/common/test_tables.rs` already establishes the pattern (`ManagedAccountsTestCase`, `PnLTestCase`, etc., with shared `responses: Vec<String>` and per-call assertions). For each domain migrated, ensure shared response data lives in `src/<domain>/common/test_tables.rs` and both sync and async test files reference it. Don't expand the pattern in this work to domains that aren't being migrated.

## Files to modify / create

**PR 1 — Foundation (new files):**
- `src/testdata/builders/mod.rs` — re-exports
- `src/testdata/builders/positions.rs` — pilot domain (positions, position_end, position_multi, position_multi_end)

**PR 1 — Modifications:**
- `src/testdata/mod.rs` — declare `pub mod builders;`
- `src/stubs.rs` — consolidate `.replace('|', "\0")` per Section 3
- `src/common/test_utils.rs` — add `assert_request_proto<T>` to `helpers`

**Each domain PR (PR 2..N):**
- Add `src/testdata/builders/<domain>.rs` with builder structs for that domain's responses
- Add `src/<domain>/common/test_tables.rs` if not already present
- Migrate tests in `src/<domain>/{sync,async}/tests.rs` to use builders + tables

**No production source changes** — this is test-infrastructure only.

## What to reuse (do not reimplement)

- `RequestMessage::from_simple(&str)` (`src/messages.rs:~791`) — pipe→NUL parser; the consolidation in Section 3 should call this if the chosen storage form is `RequestMessage`/`ResponseMessage`.
- `encode_raw_length(&[u8])` (`src/messages.rs:756`) — used by `encode_length_prefixed`.
- `encode_protobuf_message(msg_id, &[u8])` (`src/messages.rs:~770`) — used by `MessageBusStub` already.
- `encode_request_binary_from_text(&str)` (`src/messages.rs`) — text→length-prefixed binary, useful for the upcoming MemoryStream consumers.
- `decode_request_proto::<T>` (`src/common/test_utils.rs:91`) — `assert_request_proto` wraps it.
- `assert_request_msg_id` / `assert_proto_msg_id` (`src/common/test_utils.rs:74,124`) — keep as the cheap path; `assert_request_proto` is the strict path.
- `create_test_client_with_responses` and friends (`src/common/test_utils.rs:26,36,46–71`) — builders feed these unchanged.
- `constants::{TEST_ACCOUNT, TEST_CONTRACT_ID, TEST_ORDER_ID, TEST_TICKER_ID, …}` (`src/common/test_utils.rs:97–118`) — builder `Default` impls source values from here, so changing a constant updates every builder default.
- `src/accounts/common/test_tables.rs` — established sync/async dedup pattern; replicate into other domains as they're migrated.
- `src/testdata/responses.rs` — keep the existing pipe-delimited constants. New tests prefer builders; old tests stay on constants until their domain is migrated. Don't churn unrelated tests in this work.

## PR sequencing

Per user direction: foundation lands first as a separate PR, then domain-by-domain migration.

1. **PR 1 — Foundation.** Builder ergonomics, pilot positions builder, `MessageBusStub` consolidation, `assert_request_proto`. No existing test migrated. Confirms the API shape is acceptable before scaling.
2. **PR 2 — Accounts.** Builders for managed accounts, account summary, account updates, PnL, positions (already piloted in PR 1), positions multi. Migrate `src/accounts/{sync,async}/tests.rs` to use builders + `accounts/common/test_tables.rs`.
3. **PR 3 — Orders.** Place order, cancel, executions, open orders, completed orders, order update stream.
4. **PR 4 — Contracts.** Contract details, matching symbols, market rule, option chain.
5. **PR 5 — Market data.** Historical, realtime, market depth, tick-by-tick. Split if review burden warrants (e.g., historical separate from realtime).
6. **PR 6 — News, scanner, WSH.**

Each domain PR is small enough to review on its own and surfaces builder API gaps incrementally. If PR 2's adoption reveals an awkward API, fix in PR 2 before propagating.

## Verification

For each PR:

1. `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --features sync -- -D warnings`, `cargo clippy --all-features` — clean.
2. `just test` — all green under default, sync-only, and `--all-features`.
3. **Per-file lcov diff for the migrated domain** — capture `coverage/lcov.info` before and after; line and branch counts in `src/<domain>/` and `src/testdata/builders/` must not regress. (Same gate the eliminate-mock-gateway plan uses.)
4. **Spot-check 2–3 migrated tests** by reading the diff: confirm pre/post assert the same conditions (request format AND response decoding); the migration is supposed to be a refactor, not a behavior change.
5. Smoke-run any examples that touch the migrated domain (e.g. `cargo run --example positions` for PR 2) against a paper IB Gateway — verifies the builders' default field choices match real-server expectations.

## Out of scope

- MockGateway / `MemoryStream` work — separate plan in `todos/eliminate-mock-gateway.md`. That plan will *consume* these builders for its `MemoryStream` tests once PR 1 lands.
- Refactoring production encoders/decoders.
- Changing public `Client` APIs or the `MessageBus` / `AsyncMessageBus` traits.
- Migrating tests in domains beyond what each domain PR explicitly targets.
- Removing `src/testdata/responses.rs` constants — they stay for any tests not yet migrated.
