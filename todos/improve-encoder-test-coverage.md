# Improve Encoder Test Coverage Post-Protobuf Migration

## Problem

After Phase 5 (PR #452), many encoder tests only validate the 4-byte message ID header via `assert_proto_msg_id()`. Field-level protobuf decode assertions are missing for ~100+ integration tests and several encoder unit tests. If an encoder produces correct message ID but wrong field values, these tests won't catch it.

## Current State

### Helper functions (src/common/test_utils.rs)

- `assert_proto_msg_id(bytes, expected)` — checks only the 4-byte header
- `assert_request_msg_id(bus, index, expected)` — wrapper for integration tests
- `decode_request_proto<T>(bus, index) -> T` — exists but rarely used

### Encoder unit tests with field-level validation (good)

| Module | File | Field-validated tests | ID-only tests |
|--------|------|-----------------------|---------------|
| accounts | src/accounts/common/encoders.rs | 7 | 10 |
| contracts | src/contracts/common/encoders.rs | 8 | 0 |
| market_data/realtime | src/market_data/realtime/common/encoders.rs | 4 | 5 |
| market_data/historical | src/market_data/historical/common/encoders.rs | 3 | 5 |
| orders | src/orders/common/encoders.rs | 1 | 10 |
| news | src/news/common/encoders.rs | 1 | 3 |
| wsh | src/wsh/common/encoders.rs | 1 | 3 |
| scanner | src/scanner/common/encoders.rs | 0 | 3 |
| display_groups | src/display_groups/common/encoders.rs | 0 | 4 |

### Integration tests (async.rs / sync.rs) — all ID-only

Every domain's async.rs and sync.rs test module uses only `assert_request_msg_id()` without decoding request fields.

## Plan

### Phase 1: High-priority encoder unit tests

Add field-level protobuf decode assertions to encoders where incorrect fields could cause real trading impact.

#### 1a. Orders (highest priority)
- `test_encode_cancel_order` — validate `order_id`
- `test_encode_executions` — validate `req_id`, `filter` fields (account, symbol, time, side, exchange)
- `test_encode_exercise_options` — validate `req_id`, `con_id`, `exercise_action`, `exercise_quantity`, `account`, `override_flag`
- `test_encode_open_orders` — no fields to validate (empty message), skip
- `test_encode_global_cancel` — no fields, skip
- `test_encode_auto_open_orders` — validate `auto_bind`

#### 1b. Scanner
- `test_encode_scanner_subscription` — validate `req_id`, `number_of_rows`, `instrument`, `location_code`, `scan_code`, and filter fields
- `test_encode_scanner_parameters` — empty message, skip
- `test_encode_cancel_scanner_subscription` — validate `req_id`

#### 1c. Display Groups
- `test_encode_query_display_groups` — validate `req_id`
- `test_encode_subscribe_to_group_events` — validate `req_id`, `group_id`
- `test_encode_update_display_group` — validate `req_id`, `contract_info`
- `test_encode_unsubscribe_from_group_events` — validate `req_id`

### Phase 2: Fill gaps in partially-covered encoders

#### 2a. Accounts (10 ID-only tests)
- Cancel messages (`cancel_positions`, `cancel_account_summary`, `cancel_pnl`, `cancel_pnl_single`, `cancel_positions_multi`) — validate `req_id` where applicable
- `test_encode_request_positions` — empty message, skip
- `test_encode_request_managed_accounts` — empty message, skip
- `test_encode_request_family_codes` — empty message, skip
- `test_encode_request_server_time` / `_millis` — empty messages, skip

#### 2b. Market Data — realtime (5 ID-only tests)
- `test_encode_cancel_market_data` — validate `req_id`
- `test_encode_cancel_tick_by_tick` — validate `req_id`
- `test_encode_cancel_realtime_bars` — validate `req_id`
- `test_encode_request_realtime_bars` — validate `req_id`, `bar_size`, `what_to_show`, `use_rth`
- `test_encode_request_market_depth_exchanges` — empty message, skip

#### 2c. Market Data — historical (5 ID-only tests)
- `test_encode_cancel_historical_data` — validate `req_id`
- `test_encode_cancel_historical_ticks` — validate `req_id`
- `test_encode_cancel_head_timestamp` — validate `req_id`
- `test_encode_cancel_histogram_data` — validate `req_id`
- `test_encode_request_histogram_data` — validate `req_id`, `contract`, `use_rth`, `period`

#### 2d. News (3 ID-only tests)
- `test_encode_request_news_providers` — empty message, skip
- `test_encode_request_news_bulletins` — validate `all_messages` flag
- `test_encode_cancel_news_bulletin` — empty message, skip

#### 2e. WSH (3 ID-only tests)
- `test_encode_request_wsh_metadata` — validate `req_id`
- `test_encode_cancel_wsh_metadata` — validate `req_id`
- `test_encode_cancel_wsh_event_data` — validate `req_id`

### Phase 3: Integration test field validation (optional)

Upgrade select integration tests in async.rs/sync.rs to use `decode_request_proto<T>()` for critical paths:
- `test_submit_order` — decode and validate order fields
- `test_contract_details` — decode and validate contract fields
- `test_exercise_options` — decode and validate exercise fields

This phase is lower priority since encoder unit tests (Phases 1-2) provide the primary coverage.

## Implementation Pattern

Existing pattern to follow (from accounts/common/encoders.rs):

```rust
#[test]
fn test_encode_request_pnl() {
    let bytes = encode_request_pnl(9000, "DU1234567", "").unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestPnl);
    let request = crate::proto::PnlRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(request.req_id, Some(9000));
    assert_eq!(request.account.as_deref(), Some("DU1234567"));
    assert_eq!(request.model_code.as_deref(), Some(""));
}
```

For cancel messages using `encode_cancel_by_id!`:

```rust
#[test]
fn test_encode_cancel_pnl() {
    let bytes = encode_cancel_pnl(9000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::CancelPnl);
    let request = crate::proto::CancelPnl::decode(&bytes[4..]).unwrap();
    assert_eq!(request.req_id, Some(9000));
}
```

## Estimated Scope

- ~30 encoder tests to upgrade across 7 files
- ~3 optional integration test upgrades
- No new test infrastructure needed — `decode_request_proto` helper already exists
