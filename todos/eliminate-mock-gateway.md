# Plan: Replace MockGateway with In-Memory Stream Tests

## Context

The crate already has two parallel test backings:

1. **`MessageBusStub`** (`src/stubs.rs`) — implements the existing `MessageBus` / `AsyncMessageBus` traits. Used by ~20 domain modules (`accounts/`, `orders/`, `contracts/`, `market_data/`, `news/`, `scanner/`, `wsh/`, etc.) via `Client::stubbed(...)`. Fast, in-process, trait-based.
2. **`MockGateway`** (`src/client/test_support/mocks.rs`, 260 LOC + `scenarios.rs` 1,026 LOC) — a real TCP server that performs the IB wire handshake and replays scripted byte sequences. Used **only** by `src/client/sync/tests.rs` (2,555 LOC) and `src/client/async/tests.rs` (2,527 LOC).

Most MockGateway-backed tests duplicate per-domain `MessageBusStub` tests (e.g. `client::tests::test_server_time` ↔ accounts module's server-time tests). The genuinely unique coverage is **connect/handshake/disconnect/reconnect**, which exercises `Connection<S: Stream>` and the message-dispatcher thread, not the public domain API.

The trait seam needed to eliminate MockGateway already exists. `Connection<S: Stream>` (sync) and the async equivalent are generic over `Stream: Io + Reconnect + Send + Sync`. Today only `TcpSocket` implements it. Adding a `MemoryStream` test fixture lets handshake/transport tests run in-process without sockets, threads, sleeps, or port binding.

**Goal:** delete MockGateway and `test_support/` entirely. Keep coverage by (a) trusting per-domain `MessageBusStub` tests for domain logic and (b) adding a small `MemoryStream` fixture that exercises the connection/transport layer at its native module.

## Approach

### 1. Add in-memory `Stream` implementations

**Sync** — new file `src/transport/sync/memory.rs` (`#[cfg(test)] pub(crate)`):

```rust
pub(crate) struct MemoryStream {
    inbound:  Mutex<VecDeque<u8>>,   // bytes the client will read (scripted)
    outbound: Mutex<Vec<u8>>,        // bytes the client wrote (captured)
    reconnect_count: AtomicU32,
}

impl MemoryStream {
    pub fn new() -> Self { ... }
    pub fn push_response_frame(&self, fields: &[&str]);    // \0-joined + length-prefixed
    pub fn push_response_protobuf(&self, msg_id: i32, payload: &[u8]);
    pub fn captured_writes(&self) -> Vec<Vec<u8>>;         // split by length-prefix
}

impl Io for MemoryStream { /* read/write against the queues */ }
impl Reconnect for MemoryStream { /* increment counter, swap queues */ }
impl Stream for MemoryStream {}
```

**Async** — new file `src/transport/async_memory.rs` (flat sibling of `src/transport/async.rs`, which is already flat — don't introduce a new directory module). Spike this first before committing to the rest of the plan; the design choice here gates the async test rewrite.

Two viable options, neither obvious:

- `tokio::io::duplex()` returns a connected pair, but it's bidirectional and doesn't let test code pre-script responses; you'd need a side task that writes scripted bytes when the production code reads. Workable but adds a task per test.
- A custom `AsyncRead`/`AsyncWrite` over `tokio::sync::Mutex<VecDeque<u8>>` with explicit `Waker` registration: when `poll_read` finds an empty queue, store the waker; `push_response_*` wakes it after extending the queue. This is non-trivial — getting waker semantics wrong yields tests that hang or spin.

Pick one in a small standalone PR (commit/branch) before any deletion work.

**EOF / close signal.** Both sync and async variants must support a `close()` method that causes subsequent reads to return EOF (`Ok(0)`). This is required for the disconnect tests to exercise the dispatcher's shutdown path.

The `MemoryStream` types replace **two** things at once: the TCP socket *and* the scripted-response logic that lived in `ConnectionHandler` inside MockGateway. Tests script responses by pushing pre-encoded frames; assertions read the captured writes.

### 2. Move handshake/transport tests to their natural home

Today, `src/client/{sync,async}/tests.rs` mixes three concerns: (a) handshake, (b) transport-level routing, (c) per-domain API calls. Split:

- **Handshake / connect / disconnect / reconnect** → `src/connection/sync_tests.rs` and `src/connection/async_tests.rs` (flat sibling form, per Section 4). Construct `Connection::connect(MemoryStream::new(), client_id)` directly. Required scenarios:
  - Magic token + API version exchange.
  - Server version negotiation, including unsupported-version error path.
  - Time-zone parsing, including unknown-timezone error path (regression coverage for #459/#467).
  - Next-valid-order-id read.
  - Managed-accounts read.
  - Truncated handshake (EOF mid-frame).
  - `disconnect()` is idempotent and completes the dispatcher thread.
  - Reconnect path increments the reconnect counter and re-runs the handshake.

- **Transport routing / message dispatch** → `src/transport/sync/tests.rs` (already exists; stays inside the existing directory module) and new `src/transport/async_tests.rs` (flat sibling of the flat `transport/async.rs`). Construct `TcpMessageBus::new(Connection<MemoryStream>, ...)`. Required scenarios — these cover what `MessageBusStub` *cannot* (it bypasses framing and dispatch entirely by injecting `ResponseMessage::from(&str)` directly):
  - **Framing**: length-prefix decoding round-trip (`encode_raw_length` / `parse_raw_message`).
  - **Partial reads**: one logical message arriving in two read chunks.
  - **Multi-message coalescing**: two logical messages in a single read buffer.
  - **EOF mid-frame**: dispatcher surfaces a clean error rather than hanging.
  - **Correlation by request_id**: two in-flight requests with interleaved responses route to the correct subscriptions.
  - **Correlation by order_id**: same, for order-id-routed subscriptions.
  - **Shared-message routing**: `next_valid_id`, `managed_accounts`, error broadcasts.
  - **Cancel coalescing**: calling `cancel_subscription` twice sends only one cancel message (today covered only by `client/sync/tests.rs:401 test_subscription_cancel_only_sends_once`).
  - **Protobuf vs text dispatch**: `decode_proto_or_text()` selects the right path under both feature combinations.

- **Per-domain API smoke (server_time, positions, managed_accounts, …)** → already covered in `src/{accounts,orders,contracts,…}/{sync,async}/tests.rs` via `MessageBusStub`. **Delete from client/tests.rs only after the audit (Section 3) confirms duplication.** See gaps already identified there.

### 3. Pre-deletion coverage audit (gating step)

Before any test deletion lands, produce an explicit per-test disposition table. For each `#[test]` / `#[tokio::test]` in `src/client/sync/tests.rs` and `src/client/async/tests.rs`, mark one of:

- **`[duplicate of <path>]`** — covered by an existing per-domain `MessageBusStub` test. Safe to delete.
- **`[migrate to <path>]`** — unique behavior; must be ported (handshake to `connection/`, dispatcher to `transport/`, or a new per-domain stub test).
- **`[delete — covered by encoder/decoder unit tests at <path>]`** — pure encoding/decoding, already covered closer to the source.

Known gaps to resolve in this audit (found in plan review):

- `test_exercise_options` (`client/sync/tests.rs:1346`) is sync-unique; `orders/sync/tests.rs` has no `exercise_options` test (async side has `test_exercise_options`). Either add an `exercise_options` test in `orders/sync/tests.rs` using `MessageBusStub`, or migrate this one. Do not just delete it.
- `test_subscription_cancel_only_sends_once` (`client/sync/tests.rs:401`) tests dispatcher cancel-coalescing. `MessageBusStub` returns subscriptions directly without going through the dispatcher, so per-domain tests cannot cover this. Migrate to transport tests.
- `test_disconnect_completes` and `test_disconnect_is_idempotent` exercise dispatcher-thread shutdown; migrate to `connection/{sync,async}_tests.rs`. Requires `MemoryStream::close()`.
- The async side may have its own asymmetric-coverage tests (e.g. `test_order_update_stream_drop_releases_subscription`) that don't exist on the sync side. Audit both files independently.

**Close any sync/async parity gaps the audit surfaces.** A test that exists on one side but not the other is a coverage hole regardless of MockGateway. Whenever the audit shows a behavior covered by an async per-domain test but not its sync counterpart (or vice versa), add the missing test in this work — do not defer it. Examples already known: `exercise_options` (missing sync per-domain test), naming-convention drift (`orders/sync/tests.rs` uses `place_order` while `orders/async/tests.rs` uses `test_place_order` — fine, but the audit may surface async-only or sync-only tests in this file). Track each parity fix as a row in the audit table with disposition `[parity-fix: add to <path>]`.

Commit the audit table as part of the first PR; it's the artifact that makes "no coverage loss" reviewable.

### 4. Opportunistic code/test separation cleanup

CLAUDE.md item 13 says: tests live in their own files, and a directory module that exists *only* to host a `tests.rs` should collapse to flat (`foo.rs` + `foo_tests.rs`). This applies to test layout — implementation files inside a justified directory module (e.g. `transport/sync/{mod.rs, memory.rs}`) are fine as-is.

Two distinct cases this work creates the opportunity to fix:

**(a) Collapse directory modules whose only reason to exist is `tests.rs`.** After PR 5 deletes `src/client/sync/tests.rs` and `src/client/async/tests.rs`, the only remaining file in each directory is `mod.rs` — the exact disfavored form. Collapse them in the same PR:

| Today | After PR 5 |
| --- | --- |
| `src/client/sync/{mod.rs, tests.rs}` | `src/client/sync.rs` (flat) |
| `src/client/async/{mod.rs, tests.rs}` | `src/client/async.rs` (flat) |

Update `src/client/mod.rs` to declare `mod sync;` / `mod async;` against the flat files. Re-export paths don't change.

**(b) Use flat-sibling test files for new tests in already-flat modules.** `src/connection/{sync,async}.rs` and `src/transport/async.rs` are flat impl files with no tests today. New tests for them go to *parent-sibling* `_tests.rs` files, not new `connection/sync/tests.rs` directory modules:

| New impl edits | New test file (sibling form) |
| --- | --- |
| `src/connection/sync.rs` | `src/connection/sync_tests.rs` |
| `src/connection/async.rs` | `src/connection/async_tests.rs` |
| `src/transport/async.rs` | `src/transport/async_tests.rs` |

Wired in from each impl file with:

```rust
#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
```

**Where the existing directory module is justified, leave it alone.** `src/transport/sync/` already contains `mod.rs` and gains `memory.rs` (legit sibling impl file) — keep the directory. The existing test file path `src/transport/sync/tests.rs` is acceptable for a directory module; do not migrate it.

This is opportunistic, not blanket. Do not touch unrelated directory modules (`src/accounts/sync/`, `src/orders/async/`, etc.) — they have their own legitimate impl content. The trigger is "we're either deleting `tests.rs` from this directory, or adding the first tests to a flat impl file."

Land the layout collapse from (a) in **PR 5** (the deletion PR), since it's mechanical once the test moves are settled. The new sibling test files from (b) land in **PR 2** when the new tests are first added.

### 5. Delete the old infrastructure

Remove:
- `src/client/test_support/mocks.rs` (260 LOC)
- `src/client/test_support/scenarios.rs` (1,026 LOC)
- `src/client/test_support/` directory
- `src/client/sync/tests.rs` (2,555 LOC)
- `src/client/async/tests.rs` (2,527 LOC)
- The two `#[cfg(test)] mod tests` blocks in `src/client/builders/sync.rs:306` and `src/client/builders/async.rs:347` that import `MockGateway` — port their `setup_connect`-based tests to use `Connection<MemoryStream>` if not already redundant; otherwise delete.
- The corresponding `mod tests;` declarations and any `mod test_support` re-exports in `src/client/mod.rs` / `src/client/sync/mod.rs` / `src/client/async/mod.rs`.

Net delta: drop ~6,400 LOC of test infrastructure; add ~300–500 LOC of `MemoryStream` + targeted handshake/transport tests.

## PR sequencing

Do **not** ship as a single PR. The diff (≈ −6,400 / +500 LOC) hides regressions inside the noise. Stage it:

1. ✅ **PR 1 — async `MemoryStream` spike** (#473, merged). Validated `AsyncRead`/`AsyncWrite` + waker design via `src/transport/async_memory.rs`. Note: the byte-level design from this PR was later replaced with a frame-level `AsyncIo` impl in PR 2c-prep when the trait abstraction landed; the spike still served its purpose by gating the design choice.
2. PR 2 was split into four parts because the original scope conflated scaffolding, refactor work, and routing tests:
   - ✅ **PR 2a — scaffold MemoryStream test homes** (#474, merged). Added `src/transport/sync/memory.rs` plus three scaffold test files (`connection/{sync,async}_tests.rs`, `transport/async_tests.rs`).
   - ✅ **PR 2b — sync routing tests** (#475, merged). Five tests in `src/transport/sync/tests.rs`: request_id correlation under interleaving, order_id correlation, shared-channel fan-out for OpenOrder, shared-channel routing for CurrentTime, EOF surfacing.
   - ✅ **PR 2c-prep — generic `AsyncConnection` / `AsyncTcpMessageBus`** (#476, merged). Refactored async transport to be generic over a new `AsyncStream` trait (`src/transport/async_io.rs`), unblocking `AsyncConnection<MemoryStream>` and `AsyncTcpMessageBus<MemoryStream>` for tests. Rewrote `MemoryStream` to frame-level (`std::sync::Mutex` + `tokio::sync::Notify`).
   - ✅ **PR 2c — async routing tests** (#477, merged). Mirror of PR 2b's five tests in `src/transport/async_tests.rs`. One pub(crate) widening: `AsyncTcpMessageBus::read_and_route_message` (analog of sync's `dispatch`).
3. ✅ **PR 3 — coverage audit** (#478, in review). `todos/eliminate-mock-gateway-audit.md` lists every test in `client/{sync,async}/tests.rs` with disposition. Findings: 109/117 are duplicates of per-domain tests (delete in PR 5); 6 unique tests need migration (handshake × 2, disconnect × 4); 3 parity gaps need new tests.
4. **PR 4 — port unique tests and close parity gaps.** Concrete scope from the audit (`todos/eliminate-mock-gateway-audit.md`):

   **Migrate to `connection/sync_tests.rs`** (using `Connection::stubbed(MemoryStream, ...)` + scripted handshake responses — script bytes via `messages::encode_length` for the version exchange and account-info phases):
   - `test_connect` — handshake smoke (sync, line 11)
   - `test_disconnect_completes` — dispatcher-thread shutdown (sync, line 2532)
   - `test_disconnect_is_idempotent` — repeated `disconnect()` calls (sync, line 2544)

   **Migrate to `connection/async_tests.rs`** (using `AsyncConnection::stubbed(MemoryStream, ...)`; the async `MemoryStream` already supports the same scripted-frame API):
   - `test_connect` — handshake smoke (async, line 9)
   - `test_disconnect_completes` — async dispatcher shutdown via `process_messages` task (async, line 2505)
   - `test_disconnect_is_idempotent` — same fixture as above (async, line 2517)

   **Parity-fix new tests** (no migration; write fresh per-domain test):
   - `exercise_options` test in `orders/sync/tests.rs` — async has `test_exercise_options` at `orders/async/tests.rs:257`; sync per-domain file lacks it.
   - `subscription_cancel_only_sends_once` in `market_data/realtime/async/tests.rs` — sync test at `client/sync/tests.rs:401` is already `MessageBusStub`-based; port verbatim to async.
   - `client_id` field-accessor in `client/async_tests.rs` (or drop both — trivial; defer to reviewer).

   Every parity gap surfaced by the audit must be closed in this PR, not deferred.

5. **PR 5 — delete `MockGateway` infrastructure and collapse empty directory modules.** All 6,400 LOC of removals plus the `client/{sync,async}/` directory → flat-file collapse from Section 4(a). Specific deletions:
   - `src/client/sync/tests.rs` (2,555 LOC, then the empty `client/sync/` directory)
   - `src/client/async/tests.rs` (2,527 LOC, then the empty `client/async/` directory)
   - `src/client/test_support/{mocks,scenarios,mod}.rs` (~1,300 LOC)
   - The two `#[cfg(test)] mod tests` blocks in `src/client/builders/{sync,async}.rs:306,347` that import `MockGateway`. Per the audit, port their `setup_connect`-based tests to use `Connection<MemoryStream>` if not already redundant; otherwise delete.
   - `client/{sync,async}/mod.rs` → flat `client/{sync,async}.rs`, update `src/client/mod.rs` declarations.
   Reviewer verifies "everything deleted here is either duplicated elsewhere or migrated by PR 4" against the audit table.

## Files to modify

**New:**
- `src/transport/sync/memory.rs` — sync `MemoryStream` + `Stream` impl (inside the existing directory module; `mod.rs` declares it)
- `src/transport/async_memory.rs` — async `MemoryStream` (flat sibling of `transport/async.rs`)
- `src/connection/sync_tests.rs` — handshake/connect/disconnect/reconnect tests (flat sibling)
- `src/connection/async_tests.rs` — same for async (flat sibling)
- `src/transport/async_tests.rs` — async transport routing tests (flat sibling)

**Modify:**
- `src/transport/sync/mod.rs` — declare `#[cfg(test)] mod memory;` and `pub(crate) use memory::MemoryStream;`
- `src/transport/async.rs` — declare `#[cfg(test)] #[path = "async_memory.rs"] mod memory;` and re-export, plus `#[cfg(test)] #[path = "async_tests.rs"] mod tests;`
- `src/connection/sync.rs` — declare `#[cfg(test)] #[path = "sync_tests.rs"] mod tests;`
- `src/connection/async.rs` — declare `#[cfg(test)] #[path = "async_tests.rs"] mod tests;`
- `src/transport/sync/tests.rs` — extend with routing tests previously implicit in MockGateway scenarios (use `MemoryStream` instead of `MessageBusStub` where the test is genuinely about the bus, not the stub)
- `src/client/mod.rs` — drop `mod test_support;` and (in PR 5) update `mod sync;` / `mod async;` to point at the flat files after the directory collapse
- `src/client/builders/sync.rs` and `src/client/builders/async.rs` — rewrite the two `#[cfg(test)]` blocks at lines 306 and 347 to drop MockGateway

**Move (PR 5, layout collapse from Section 4(a)):**
- `src/client/sync/mod.rs` → `src/client/sync.rs`
- `src/client/async/mod.rs` → `src/client/async.rs`

**Delete:**
- `src/client/test_support/mocks.rs`
- `src/client/test_support/scenarios.rs`
- `src/client/test_support/mod.rs`
- `src/client/sync/tests.rs` (then the empty `client/sync/` directory)
- `src/client/async/tests.rs` (then the empty `client/async/` directory)

## What to reuse

- `MessageBusStub` (`src/stubs.rs:31`) — unchanged. Already the right abstraction for ~20 domain modules.
- `Stream`, `Io`, `Reconnect` traits (`src/transport/sync/mod.rs:769,800,764`) — `MemoryStream` implements these.
- `Connection<S: Stream>` (`src/connection/sync.rs:23`) — already generic; `Connection::connect(MemoryStream::new(), id)` works without changes.
- `encode_raw_length` / `parse_raw_message` from `src/messages.rs` — `MemoryStream` helpers use these so scripted bytes stay framed correctly.
- `ResponseMessage::from(&str)` for text-format scripting; `encode_request_binary_from_text` (used today by MockGateway in `mocks.rs:129`) for protobuf-format scripting.
- `src/testdata/responses.rs` — shared response constants stay; just used from the new test homes.

## Key design constraints

- **Async needed a new trait surface; sync did not.** The original plan called for "no new trait surface" but async previously had no analog of sync's `Stream: Io + Reconnect`. PR 2c-prep added `AsyncIo` / `AsyncReconnect` / `AsyncStream` in `src/transport/async_io.rs`, mirroring sync. `pub(crate)` only — production `Client` API untouched.
- **Sync and async parity.** Both `MemoryStream` variants must compile under default-features (async), `--features sync`, and `--all-features` (per CLAUDE.md item 5). Both are `#[cfg(test)] pub(crate)` so they don't leak.
- **No real I/O in tests.** No `TcpListener::bind`, no `thread::sleep`, no port allocation. `MemoryStream` is fully deterministic.
- **Coverage parity, not test-count parity.** Before deletion, verify each unique behavior in `client/{sync,async}/tests.rs` is either (a) covered by a per-domain test today, or (b) reproduced in the new connection/transport tests. Items only in `client/tests.rs` and not duplicated elsewhere migrate; items duplicated elsewhere just get deleted.

## Verification

1. `cargo fmt` — clean.
2. `cargo clippy --all-targets -- -D warnings` — clean (default = async).
3. `cargo clippy --all-targets --features sync -- -D warnings` — clean.
4. `cargo clippy --all-features` — clean.
5. `cargo test` (default), `cargo test --no-default-features --features sync`, `cargo test --all-features` — all pass.
6. `just test` — clean.
7. **Per-file coverage diff** — capture `coverage/lcov.info` before PR 2 and after PR 5 (or at any boundary that adds/removes tests). Diff per-file line and branch counts. The global ratio is too coarse: with ~6,400 LOC deleted, "≥ today's coverage" trivially passes even if real coverage was lost. Any drop in `src/connection/`, `src/transport/`, `src/messages.rs`, or `src/{orders,accounts,contracts,…}/common/decoders.rs` is a red flag and blocks merge. Specifically verify the `establish_connection` path in `src/connection/sync.rs` and `src/connection/async.rs` retains full coverage via the new `MemoryStream` tests.
8. Manual: run one example against a paper IB Gateway (e.g. `cargo run --example server_time`) to confirm the production wire path still works end-to-end. The deleted tests never caught this anyway — the integration tests under `tests/` are what cover live-gateway behavior.

## Out of scope

- Changing the public `MessageBus` / `AsyncMessageBus` trait surface.
- Refactoring `MessageBusStub` itself.
- Refactoring per-domain test files beyond the parity fixes Section 3 surfaces (e.g., adding `exercise_options` to `orders/sync/tests.rs`). Renaming, restructuring, or migrating test layout in `accounts/`, `orders/`, `contracts/`, etc. is out of scope.
- Migrating other directory modules' layout (`src/accounts/sync/`, `src/orders/async/`, etc.). Section 4 is scoped narrowly to the modules this work already touches heavily.
- Adding new traits or higher-level transport abstractions (per the user's choice).
