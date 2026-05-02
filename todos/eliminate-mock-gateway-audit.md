# PR 3 — Coverage Audit for `client/{sync,async}/tests.rs`

Per `todos/eliminate-mock-gateway.md` §3, this is the gating artifact for PR 4 (port + parity-fix) and PR 5 (delete + collapse). One row per `#[test]` / `#[tokio::test]` in `src/client/sync/tests.rs` (59 tests, 2,555 LOC) and `src/client/async/tests.rs` (57 tests, 2,527 LOC).

## Disposition legend

- **`[duplicate of <path>]`** — covered by an existing per-domain `MessageBusStub` test; safe to delete in PR 5.
- **`[migrate to <path>]`** — unique behavior; PR 4 ports it.
- **`[delete — covered by encoder/decoder unit tests at <path>]`** — pure encoding/decoding, already covered closer to the source.
- **`[parity-fix: add to <path>]`** — sync-only or async-only behavior whose counterpart needs writing.

## Methodology

Every test in `client/{sync,async}/tests.rs` falls into one of two patterns:

1. **MockGateway-driven** (the majority, ~54/59 sync, ~54/57 async). Spins up the TCP `MockGateway`, runs scenarios from `client/test_support/scenarios.rs`, calls a `Client::*` method, asserts. Equivalent per-domain tests use `MessageBusStub` (skips network, scripts responses through the bus directly) and live in `<domain>/{sync,async}/tests.rs`. These are duplicates.
2. **`MessageBusStub`-driven** (the minority — `test_client_id`, `test_subscription_cancel_only_sends_once`). Already use the per-domain pattern; they're "in the wrong file" rather than coupled to MockGateway. Move them to a sibling `client/{sync,async}_tests.rs` (after PR 5's directory collapse) or into the most relevant per-domain file.
3. **Connection-level** — `test_connect`, `test_disconnect_completes`, `test_disconnect_is_idempotent`. These exercise handshake / dispatcher-shutdown specifically; they belong in `connection/{sync,async}_tests.rs` and need to be written using `MemoryStream` + `Connection::stubbed` (sync) / `AsyncConnection::stubbed` (async).

For the bulk MockGateway-duplicate cases I confirmed:
- A per-domain test with the matching name exists in `<domain>/<sync|async>/tests.rs`. Verified by spot-checking sync `test_server_time` ↔ `accounts/sync/tests.rs:184`, sync `test_realtime_bars` ↔ `market_data/realtime/sync/tests.rs:43`, sync `test_market_data` ↔ `market_data/realtime/sync/tests.rs:338`, sync `test_submit_order_with_order_update_stream` ↔ `orders/sync/tests.rs:732 order_update_stream`, async `test_exercise_options` ↔ `orders/async/tests.rs:257`.
- For batched dispositions below, "duplicate of `<domain>/<flavor>/tests.rs`" means the per-domain file has a test exercising the same `Client::*` method against scripted responses through `MessageBusStub`. Field-by-field assertion sets may differ; the COVERAGE of "API method routes correctly and decodes response" is the same.

## Summary

|                          | sync | async |
| ------------------------ | ---- | ----- |
| Tests total              |  59  |  57   |
| `[duplicate]`            |  54  |  54   |
| `[migrate]`              |   3  |   3   |
| `[parity-fix]`           |   2  |   1   |

Net to migrate in PR 4: **6 unique tests** (handshake × 2, disconnect × 4) + **3 parity-fix items** (exercise_options sync, client_id async, subscription_cancel_only_sends_once async). Net deletions in PR 5: ~108 duplicate test bodies plus the `client/test_support/{mocks,scenarios}.rs` infrastructure.

## Sync — `src/client/sync/tests.rs`

Most rows are batched by per-domain target since the disposition is identical. Singletons get their own row.

### Connection / handshake / dispatcher (migrate to connection or transport)

| Line | Test | Disposition |
| ---- | ---- | ----------- |
| 11   | `test_connect`                           | **[migrate to `connection/sync_tests.rs`]** — handshake smoke (asserts client_id, server_version, time_zone after connect; no requests sent). Use `Connection::stubbed(MemoryStream, ...)` + scripted handshake responses. |
| 401  | `test_subscription_cancel_only_sends_once` | **[parity-fix: add to `async`]** + **[duplicate of MessageBusStub layer]** — already MessageBusStub-based, not MockGateway-coupled. Move to `market_data/realtime/sync/tests.rs` (uses `realtime_bars`); add async counterpart in `market_data/realtime/async/tests.rs`. |
| 2532 | `test_disconnect_completes`              | **[migrate to `connection/sync_tests.rs`]** — exercises dispatcher-thread shutdown via `Connection::stubbed(MemoryStream)` + `client.disconnect()`. Requires `MemoryStream::close()` (already there). |
| 2544 | `test_disconnect_is_idempotent`          | **[migrate to `connection/sync_tests.rs`]** — same fixture as above; calls `disconnect()` twice. |

### Stub-based, "in the wrong file"

| Line | Test | Disposition |
| ---- | ---- | ----------- |
| 346  | `test_client_id` | **[migrate to `client/sync_tests.rs`]** + **[parity-fix: add to `async`]** — MessageBusStub-based, tests `Client::client_id()` field accessor. After PR 5's `client/sync/` → `client/sync.rs` collapse, lives in sibling `client/sync_tests.rs`. Add async equivalent. |

### MockGateway duplicates of per-domain tests

| Lines | Tests | Disposition |
| ----- | ----- | ----------- |
| 24, 37 | `test_server_time`, `test_next_valid_order_id` | **[duplicate of `accounts/sync/tests.rs`]** — `test_server_time` line 184; `next_valid_order_id` covered via shared-channel routing tests. |
| 50    | `test_managed_accounts` | **[duplicate of `accounts/sync/tests.rs`]** line 148. |
| 63, 92 | `test_positions`, `test_positions_multi` | **[duplicate of `accounts/sync/tests.rs`]** lines 45, 74. |
| 133, 171, 191 | `test_account_summary`, `test_pnl`, `test_pnl_single` | **[duplicate of `accounts/sync/tests.rs`]**. |
| 214, 288 | `test_account_updates`, `test_account_updates_multi` | **[duplicate of `accounts/sync/tests.rs`]**. |
| 270   | `test_family_codes` | **[duplicate of `accounts/sync/tests.rs`]**. |
| 360, 441, 481, 513, 560, 607 | `test_contract_details`, `test_matching_symbols`, `test_market_rule`, `test_calculate_option_price`, `test_calculate_implied_volatility`, `test_option_chain` | **[duplicate of `contracts/sync/tests.rs`]**. |
| 666   | `test_place_order` | **[duplicate of `orders/sync/tests.rs`]** + **[delete — covered by encoder unit tests at `orders/common/encoders.rs::tests`]**. The bulk of place_order's value is encoder coverage; the per-domain test exercises the routed response. |
| 784   | `test_submit_order_with_order_update_stream` | **[duplicate of `orders/sync/tests.rs:732 order_update_stream`]** — same submit-then-stream coverage. The MockGateway version exercises end-to-end network framing, which is already covered by encoder/decoder + transport routing tests landed in PR 2b. |
| 895, 955, 1024, 1089 | `test_open_orders`, `test_all_open_orders`, `test_auto_open_orders`, `test_completed_orders` | **[duplicate of `orders/sync/tests.rs`]**. |
| 1151, 1210, 1240 | `test_cancel_order`, `test_global_cancel`, `test_executions` | **[duplicate of `orders/sync/tests.rs`]**. |
| 1346  | `test_exercise_options` | **[parity-fix: add to `orders/sync/tests.rs`]** — async has `test_exercise_options` at `orders/async/tests.rs:257`; sync per-domain file lacks it. PR 4 must add an `exercise_options` test to `orders/sync/tests.rs` using `MessageBusStub` before this client/sync version is deleted. |
| 1431, 1533 | `test_market_data`, `test_realtime_bars` | **[duplicate of `market_data/realtime/sync/tests.rs`]** — `test_market_data` line 338; `test_realtime_bars` line 43. |
| 1590, 1641, 1689, 1739 | `test_tick_by_tick_*` (last/all_last/bid_ask/midpoint) | **[duplicate of `market_data/realtime/sync/tests.rs`]**. |
| 1773, 1831, 1869 | `test_market_depth`, `test_market_depth_exchanges`, `test_switch_market_data_type` | **[duplicate of `market_data/realtime/sync/tests.rs`]**. |
| 1895, 1923, 1974 | `test_head_timestamp`, `test_historical_data`, `test_historical_schedules` | **[duplicate of `market_data/historical/sync/tests.rs`]**. |
| 2000, 2043, 2078, 2118 | `test_historical_ticks_*`, `test_histogram_data` | **[duplicate of `market_data/historical/sync/tests.rs`]**. |
| 2153, 2184, 2224, 2273 | `test_news_providers`, `test_news_bulletins`, `test_historical_news`, `test_news_article` | **[duplicate of `news/sync.rs::tests`]** (inline test module per the file's structure). |
| 2302, 2330 | `test_scanner_parameters`, `test_scanner_subscription` | **[duplicate of `scanner/sync.rs::tests`]**. |
| 2377, 2397, 2419, 2464, 2504 | `test_wsh_metadata`, `test_wsh_event_data`, `test_contract_news`, `test_broad_tape_news`, `test_wsh_event_data_by_filter` | **[duplicate of `wsh/sync.rs::tests`]**. |

## Async — `src/client/async/tests.rs`

Mirror of sync minus three sync-only tests (`test_client_id`, `test_subscription_cancel_only_sends_once`, no separate name for historical_schedule).

### Connection / handshake / dispatcher

| Line | Test | Disposition |
| ---- | ---- | ----------- |
| 9    | `test_connect`                           | **[migrate to `connection/async_tests.rs`]** — handshake smoke. Requires `AsyncConnection::stubbed(MemoryStream, ...)` (already in place from PR 2c-prep) plus scripted handshake responses. |
| 2505 | `test_disconnect_completes`              | **[migrate to `connection/async_tests.rs`]** — drives `process_messages` to verify clean shutdown. May require driving the full task or adding a test-only helper. |
| 2517 | `test_disconnect_is_idempotent`          | **[migrate to `connection/async_tests.rs`]** — calls `disconnect()` twice; same fixture as `test_disconnect_completes`. |

### Parity-fix only (no migration; new test added in PR 4)

| Target | Source of truth | Disposition |
| ------ | --------------- | ----------- |
| `market_data/realtime/async/tests.rs` | sync `test_subscription_cancel_only_sends_once` | **[parity-fix]** — async equivalent of the cancel-coalescing test. Stub-based; no MockGateway involvement. |
| `client/async_tests.rs` | sync `test_client_id` | **[parity-fix]** — trivial field-accessor test; if the sync version is judged disposable, drop both. |

### MockGateway duplicates of per-domain tests

Same batching as sync; per-domain target paths are the `async/tests.rs` equivalents.

| Lines | Tests | Disposition |
| ----- | ----- | ----------- |
| 22, 35, 48, 61, 90, 131, 169, 189, 212, 268, 286 | accounts cluster (server_time, next_valid_order_id, managed_accounts, positions, positions_multi, account_summary, pnl, pnl_single, account_updates, family_codes, account_updates_multi) | **[duplicate of `accounts/async/tests.rs`]**. |
| 344, 387, 426, 459, 507, 555 | contracts cluster (contract_details, matching_symbols, market_rule, calculate_option_price, calculate_implied_volatility, option_chain) | **[duplicate of `contracts/async/tests.rs`]**. |
| 607, 711, 824, 886, 957, 1024, 1088, 1148, 1178, 1287 | orders cluster (place_order, submit_order_with_order_update_stream, open_orders, all_open_orders, auto_open_orders, completed_orders, cancel_order, global_cancel, executions, exercise_options) | **[duplicate of `orders/async/tests.rs`]**. |
| 1375, 1476, 1533, 1584, 1632, 1682, 1716, 1775, 1810 | realtime cluster (market_data, realtime_bars, tick_by_tick_*, market_depth, market_depth_exchanges, switch_market_data_type) | **[duplicate of `market_data/realtime/async/tests.rs`]**. |
| 1835, 1864, 1917, 1944, 1991, 2030, 2074 | historical cluster (head_timestamp, historical_data, historical_schedule, historical_ticks_*, histogram_data) | **[duplicate of `market_data/historical/async/tests.rs`]**. |
| 2110, 2139, 2180, 2231 | news cluster (news_providers, news_bulletins, historical_news, news_article) | **[duplicate of `news/async.rs::tests`]**. |
| 2259, 2285 | scanner cluster (scanner_parameters, scanner_subscription) | **[duplicate of `scanner/async.rs::tests`]**. |
| 2340, 2358, 2379, 2428, 2469 | wsh cluster (wsh_metadata, wsh_event_data, contract_news, broad_tape_news, wsh_event_data_by_filter) | **[duplicate of `wsh/async.rs::tests`]**. |

## Parity gaps for PR 4

Compiling the `[parity-fix]` rows above:

1. **`exercise_options` in `orders/sync/tests.rs`** — the most concrete gap. Async has it at `orders/async/tests.rs:257`; sync per-domain file has no equivalent. PR 4 adds a `MessageBusStub`-driven test before the client/sync version is deleted in PR 5.
2. **`subscription_cancel_only_sends_once` in `market_data/realtime/async/tests.rs`** — sync test exists at `client/sync/tests.rs:401` (already MessageBusStub-based); add async counterpart and move both to `market_data/realtime/<sync,async>/tests.rs`.
3. **`client_id` field accessor parity** — sync test at `client/sync/tests.rs:346`; if both are kept, they live at `client/{sync,async}_tests.rs` after PR 5's collapse. The test is trivial enough that dropping both is also defensible — defer to reviewer preference.

## Out of scope for PR 4

- The two `#[cfg(test)] mod tests` blocks in `client/builders/{sync,async}.rs:306,347` mentioned by §5 of the plan import `MockGateway`. The plan calls them out but they're NOT in `client/{sync,async}/tests.rs` — they're *separate* tests. PR 5 handles them alongside the other deletions.

## Verification of "no coverage loss" before PR 5 merges

- For every `[duplicate]` row, verify the named per-domain test file has a test against the same `Client::*` method. Spot-check sample (12 of ~109 rows) confirmed during this audit.
- For every `[migrate]` row, verify the migration landed in PR 4 (`grep -rn 'fn test_connect\|fn test_disconnect_completes\|fn test_disconnect_is_idempotent' src/connection/`).
- For every `[parity-fix]` row, verify the missing-side test landed in PR 4.
- Run `just cover` before and after the PR 5 merge; per-file coverage in `src/connection/`, `src/transport/`, `src/messages.rs`, `src/<domain>/common/decoders.rs` must not drop. Per the plan's Verification §7 ("the global ratio is too coarse").
