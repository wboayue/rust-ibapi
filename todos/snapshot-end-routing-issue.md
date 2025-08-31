# SnapshotEnd Message Routing Issue

## Status
**Priority:** Medium  
**Type:** Bug - Test Environment

## Description
During integration testing of real-time market data methods, `SnapshotEnd` messages (IncomingMessages type 17) are not being properly routed to subscriptions in the test environment. Debug logs show "no recipient found for: ResponseMessage { i: 0, fields: ["17", "1", "9000"] }" when the SnapshotEnd message arrives.

## Current Behavior
- The SnapshotEnd message is sent by MockGateway but not received by the subscription
- Other tick messages (Price, Size, Generic, String) are routed correctly
- This affects snapshot market data requests where we expect a SnapshotEnd to signal completion
- Tests have the SnapshotEnd assertion commented out as a workaround

## Files Affected
- `src/client/sync.rs:3563` - test_market_data() test has commented assertion for snapshot_end
- `src/client/async.rs:3431` - Same issue, commented assertion in async version
- `src/transport/sync.rs` - Message routing logic may need investigation
- `src/market_data/realtime/mod.rs` - TickTypes::SnapshotEnd variant handling

## Potential Causes
1. The subscription might be closing/dropping before the SnapshotEnd message arrives
2. Message routing logic might not handle SnapshotEnd messages correctly
3. The subscription channel might be full or closed when SnapshotEnd arrives
4. Timing issue in test environment that doesn't occur in production

## Test Cases
- `src/client/sync.rs:test_market_data()` - SnapshotEnd assertion commented out
- `src/client/async.rs:test_market_data()` - SnapshotEnd assertion commented out

## Notes
- MockGateway correctly sends the SnapshotEnd message
- The message format appears correct: "17\01\09000\0"
- Issue only observed in test environment, production behavior unknown