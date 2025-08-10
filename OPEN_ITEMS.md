# Open Items

This document tracks known issues, technical debt, and improvements needed in the rust-ibapi codebase.

## Architecture & Design Issues

### Shared Channel Routing Logic Discrepancy
**Priority:** High  
**Components:** `transport/sync.rs`, `transport/async.rs`, `transport/routing.rs`  
**Issue:** The sync and async implementations use different approaches for routing messages to shared channels:

- **Sync implementation:** 
  - Uses `CHANNEL_MAPPINGS` from `shared_channel_configuration.rs` to configure channels at startup
  - Routes messages by directly checking if a sender exists: `shared_channels.contains_sender(message.message_type())`
  - Single source of truth for channel mappings

- **Async implementation:**
  - Also uses `CHANNEL_MAPPINGS` for initial channel setup
  - BUT uses a separate `map_incoming_to_outgoing()` function in `routing.rs` for response routing
  - Two sources of truth that must be kept manually in sync

**Impact:** This inconsistency can lead to bugs where a channel is configured but responses aren't routed (as happened with `FamilyCodes` and `MarketRule`). This also affects integration testing where async tests fail while sync tests pass.

**Specific Example - Order Messages:**
The issue is particularly problematic for order-related messages where multiple request types generate the same response types:
- `RequestOpenOrders`, `RequestAllOpenOrders`, and `RequestAutoOpenOrders` all generate `OpenOrder`, `OrderStatus`, and `OpenOrderEnd` responses
- The `map_incoming_to_outgoing()` function can only map each response type to ONE request type
- When an `OpenOrder` message arrives, the async implementation cannot determine which of the three possible shared channels to route it to
- This causes integration tests for `open_orders()` to fail in async mode while passing in sync mode

**Proposed Solutions:**
1. **Option A:** Modify async to use the same approach as sync (check channel existence directly)
2. **Option B:** Generate `map_incoming_to_outgoing()` from `CHANNEL_MAPPINGS` to ensure consistency
3. **Option C:** Consolidate both implementations to use a unified routing strategy

**Related Files:**
- `/src/messages/shared_channel_configuration.rs` - Contains `CHANNEL_MAPPINGS`
- `/src/transport/routing.rs` - Contains `map_incoming_to_outgoing()`
- `/src/transport/sync.rs` - Sync message routing (lines 281-289)
- `/src/transport/async.rs` - Async message routing (lines 310-320)

## Testing Gaps

### Integration Test Coverage
**Priority:** Medium  
**Issue:** Only ~15% of client methods have integration tests using MockGateway

See `CLIENT_INTEGRATION_TEST_TRACKING.md` for detailed coverage information.

## Known Bugs

*No critical bugs currently tracked*

## Performance Improvements

*No performance issues currently tracked*

## Documentation

*No documentation issues currently tracked*

---

*Last updated: 2025-08-09*