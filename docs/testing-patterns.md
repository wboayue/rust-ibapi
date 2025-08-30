# Testing Patterns

This document describes the testing patterns and infrastructure used in the rust-ibapi crate, with a focus on the MockGateway pattern for integration testing.

## MockGateway Integration Testing Pattern

The MockGateway pattern provides a robust framework for testing Client methods without requiring a real IB Gateway/TWS connection. This pattern is implemented in `src/client/common.rs` and ensures consistent, reliable testing across both sync and async implementations.

### Architecture Overview

```
┌─────────────┐       TCP Socket       ┌──────────────┐
│   Client    │ ◄──────────────────────► │ MockGateway  │
│  (under     │                         │  (simulated  │
│   test)     │                         │   IB server) │
└─────────────┘                         └──────────────┘
```

### Key Components

1. **MockGateway** (`src/client/common.rs::mocks::MockGateway`)
   - Simulates IB Gateway/TWS server behavior
   - Binds to a random TCP port for real network testing
   - Handles the complete handshake protocol including magic token exchange
   - Records all incoming requests for verification
   - Sends pre-configured responses based on defined interactions

2. **ConnectionHandler** (internal to MockGateway)
   - Manages the TCP connection lifecycle
   - Performs protocol handshake (version exchange, client ID validation)
   - Routes requests to appropriate response handlers
   - Maintains request/response interaction mappings

3. **Setup Functions** (`src/client/common.rs::tests`)
   - Provide pre-configured MockGateway instances for specific test scenarios
   - Define expected request/response interactions
   - Examples: `setup_connect()`, `setup_server_time()`, `setup_contract_details()`

### Test Pattern Structure

#### 1. Create Setup Function
```rust
pub fn setup_contract_details() -> MockGateway {
    let mut gateway = MockGateway::new(server_versions::IPO_PRICES);
    
    gateway.add_interaction(
        OutgoingMessages::RequestContractData,
        vec![
            // Response messages in TWS protocol format
            "10\09000\0AAPL\0STK\0...", // ContractData message
            "52\01\09000\0",             // ContractDataEnd message
        ],
    );
    
    gateway.start().expect("Failed to start mock gateway");
    gateway
}
```

#### 2. Write Test (Sync)
```rust
#[test]
fn test_contract_details() {
    let gateway = setup_contract_details();
    let client = Client::connect(&gateway.address(), CLIENT_ID).expect("Failed to connect");
    
    // Execute the method under test
    let details = client.contract_details(&contract).expect("Failed to get details");
    
    // Verify response parsing
    assert_eq!(details[0].contract.symbol, "AAPL");
    
    // Verify request format
    let requests = gateway.requests();
    assert_eq!(requests[0], "9\08\09000\0...");
}
```

#### 3. Write Test (Async)
```rust
#[tokio::test]
async fn test_contract_details() {
    let gateway = setup_contract_details();
    let client = Client::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");
    
    // Execute the method under test
    let details = client.contract_details(&contract).await.expect("Failed to get details");
    
    // Verify response parsing (identical assertions as sync)
    assert_eq!(details[0].contract.symbol, "AAPL");
    
    // Verify request format
    let requests = gateway.requests();
    assert_eq!(requests[0], "9\08\09000\0...");
}
```

### Message Format

Messages follow the IB TWS protocol format using null-terminated strings:
- Format: `field1\0field2\0field3\0...`
- First field is typically the message type ID
- Subsequent fields depend on the specific message type
- Example: `"10\09000\0AAPL\0STK\0"` represents ContractData with request_id=9000, symbol=AAPL, security_type=STK

### Benefits of This Pattern

1. **Real Network Testing**: Uses actual TCP connections, testing the full network stack
2. **Protocol Verification**: Tests the complete handshake and message exchange
3. **Request Recording**: All requests are captured for detailed verification
4. **Deterministic Responses**: Pre-configured responses ensure consistent test results
5. **Shared Test Logic**: Common setup functions ensure sync/async tests are identical
6. **No External Dependencies**: Tests run without requiring IB Gateway/TWS installation

### Best Practices

1. **Reuse Setup Functions**: Create shared setup functions for common scenarios
2. **Test Both Directions**: Verify both request format (what client sends) and response parsing (what client receives)
3. **Use Meaningful Request IDs**: Use consistent IDs like 9000 for easier debugging
4. **Document Message Formats**: Add comments explaining the structure of request/response messages
5. **Keep Tests Identical**: Sync and async tests should have identical assertions
6. **Record Real Messages**: When implementing new tests, you can run against a real IB Gateway/TWS server with `IBAPI_RECORDING_DIR=/tmp/tws-messages` to capture actual protocol messages for use in MockGateway setup functions

## Unit Testing

The crate uses standard Rust unit testing patterns with `#[test]` for sync code and `#[tokio::test]` for async code.

### Running Tests

```bash
# Run all sync tests
cargo test --features sync

# Run all async tests  
cargo test --features async

# Run specific test module
cargo test --features sync client::sync::tests

# Run with logging
RUST_LOG=debug cargo test --features sync
```

### Test Organization

- Unit tests are placed in `#[cfg(test)]` modules within the same file as the code being tested
- Integration tests using MockGateway are in `src/client/sync.rs` and `src/client/async.rs`
- Common test utilities are in `src/client/common.rs`

## Coverage

The crate achieves 100% test coverage for all public Client methods using the MockGateway pattern. Every method has both sync and async tests to ensure feature parity.