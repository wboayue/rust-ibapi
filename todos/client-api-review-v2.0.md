# Client API Review for v2.0 Release

## Executive Summary

This document provides a comprehensive review of the public API for the client module in rust-ibapi, identifying opportunities for breaking changes that would improve type safety, ergonomics, and adherence to Rust idioms for the v2.0 release.

**Last Updated**: 2025-08-31 - Updated to reflect TradingHours enum implementation and async client visibility fixes

## Key Findings

### 1. Method Naming Inconsistencies

**Issue**: Inconsistent naming patterns across sync and async implementations
- Sync uses `place_order()` while async has both `place_order()` and `submit_order()`
- Some methods use underscores (e.g., `account_updates_multi`) while similar methods use different patterns

**Recommendation**: 
- Standardize on a single naming convention
- Consider renaming to follow Rust conventions more closely (e.g., `get_` prefix for getters that do work)

### 2. Error Handling Improvements

**Current State**: All methods return `Result<T, Error>` with a single `Error` type

**Issues**:
- The single `Error` enum mixes different error categories (connection, parsing, validation, server errors)
- No way to handle specific error types ergonomically
- Error variants like `Parse(i32, String, String)` use positional parameters

**Recommendations**:
```rust
// Split errors into categories
pub enum ClientError {
    Connection(ConnectionError),
    Validation(ValidationError),
    Server(ServerError),
    Parse(ParseError),
}

// Use structured error data
pub struct ParseError {
    pub message_type: i32,
    pub field: String,
    pub details: String,
}
```

### 3. Builder Pattern Issues

**Current State**: Internal builder traits that are not exposed properly

**Issues**:
- `ClientRequestBuilders` and `SubscriptionBuilderExt` are pub(crate) but referenced in mod.rs
- Builder pattern not consistently used across all complex operations
- No fluent API for common operations

**Recommendations**:
```rust
// Expose a proper builder API
pub struct ClientBuilder {
    address: String,
    client_id: i32,
    timeout: Option<Duration>,
    retry_policy: Option<RetryPolicy>,
}

impl ClientBuilder {
    pub fn new(address: impl Into<String>) -> Self { ... }
    pub fn client_id(mut self, id: i32) -> Self { ... }
    pub fn timeout(mut self, timeout: Duration) -> Self { ... }
    pub fn connect(self) -> Result<Client, Error> { ... }
    pub async fn connect_async(self) -> Result<Client, Error> { ... }
}
```

### 4. Type Safety Issues

**✅ PARTIALLY ADDRESSED - TradingHours Enum**: The `use_rth: bool` parameter has been successfully replaced with the `TradingHours` enum across all market data methods in both sync and async implementations.

**String Parameters**: Many methods still use `&str` where enums would be more appropriate
```rust
// Current
pub fn cancel_order(&self, order_id: i32, manual_order_cancel_time: &str)

// Better
pub fn cancel_order(&self, order_id: i32, cancel_time: CancelTime)
```

**Optional Parameters**: Heavy use of `Option<T>` in method signatures
```rust
// Current
pub fn positions_multi(&self, account: Option<&AccountId>, model_code: Option<&ModelCode>)

// Better - use builder pattern
client.positions().with_account(&account).with_model(&model).subscribe()
```

### 5. ID Management Exposure

**Issue**: ID generation is mostly internal but partially exposed

**Current State**:
- `next_order_id()` and `next_request_id()` are public
- `IdGenerator` and `ClientIdManager` are public but shouldn't be

**Recommendation**:
- Make ID generation completely internal
- Provide a way to get current IDs if needed, but don't expose mutation

### 6. Subscription API Consistency

**Issue**: Different return types for similar operations

```rust
// Returns Iterator
pub fn matching_symbols(&self, pattern: &str) -> Result<impl Iterator<Item = ContractDescription>, Error>

// Returns Vec
pub fn contract_details(&self, contract: &Contract) -> Result<Vec<ContractDetails>, Error>

// Returns Subscription
pub fn positions(&self) -> Result<Subscription<PositionUpdate>, Error>
```

**Recommendation**: Standardize on consistent return types for similar operations

### 7. Async/Sync API Divergence

**✅ FIXED - Internal Method Visibility**:
- Changed async client's internal methods to `pub(crate)`:
  - `send_request()`, `send_shared_request()`, `send_order()`, `send_message()`, `create_order_update_subscription()`
  - These are now properly hidden from the public API

**✅ RESOLVED - Method Naming**:
- Both sync and async correctly have both `place_order()` and `submit_order()` methods
- `submit_order()` - sends order without subscription for updates
- `place_order()` - sends order with subscription for updates
- This is intentional API design, not an inconsistency

**Remaining Minor Issues**:
- Different method signatures between sync and async (fully qualified paths vs imported types)
- Could be standardized for consistency but not critical

### 8. Connection Management

**Current State**: Single `connect()` method with string address

**Issues**:
- No connection pooling
- No automatic reconnection
- No connection configuration options

**Recommendations**:
```rust
pub struct ConnectionConfig {
    pub address: SocketAddr,
    pub client_id: i32,
    pub auto_reconnect: bool,
    pub heartbeat_interval: Duration,
    pub request_timeout: Duration,
}

impl Client {
    pub fn connect(config: ConnectionConfig) -> Result<Client, Error> { ... }
}
```

### 9. Resource Management

**Issue**: No explicit cleanup methods

**Current State**: Relies on Drop trait

**Recommendation**: 
- Add explicit `disconnect()` or `close()` method
- Document cleanup behavior
- Consider implementing `AsyncDrop` when stable

### 10. Documentation Gaps

**Issues**:
- Many public methods lack comprehensive documentation
- No examples for complex operations
- Missing error condition documentation

**Recommendation**: Add comprehensive rustdoc with:
- Examples for each public method
- Error conditions
- TWS version requirements
- Rate limiting information

## Breaking Changes Priority List

### High Priority (Must Fix for v2.0)
1. ⚠️ Hide internal implementation details (builder traits, ID generators)
2. ✅ Fix async client's exposed internal methods - **FIXED**
3. ⚠️ Standardize error handling with categorized errors - **Current single Error enum still in use**
4. ✅ Ensure API parity between sync and async - **Both have submit_order and place_order with correct semantics**

### Medium Priority (Should Fix)
1. ⚠️ Implement proper builder pattern for Client construction
2. ✅ Replace string parameters with type-safe enums - **Partially done with TradingHours**
3. ⚠️ Standardize return types for similar operations
4. ✅ Fix method naming inconsistencies - **Major issues resolved**

### Low Priority (Nice to Have)
1. Add connection configuration options
2. Implement automatic reconnection
3. Add request/response interceptors for debugging
4. Support for connection pooling

## Migration Guide Outline

For each breaking change, provide:
1. Clear before/after examples
2. Automated migration tool or script where possible
3. Deprecation warnings in v1.x releases
4. Feature flags for gradual migration

## Conclusion

The current API is functional but has several issues that impact type safety, ergonomics, and maintainability. The v2.0 release presents an opportunity to address these issues with breaking changes that will significantly improve the developer experience.

Key improvements will include:
- Better type safety through enums and structured types
- Consistent API between sync and async implementations  
- Proper encapsulation of internal implementation details
- More ergonomic builder patterns for complex operations
- Comprehensive error handling with categorized errors

These changes will make the library more idiomatic, safer, and easier to use while maintaining the core functionality.