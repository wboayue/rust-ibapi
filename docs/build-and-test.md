# Build and Test Guide

## Build Commands

### Basic Build
```bash
# Build with sync support
cargo build --features sync

# Build with async support
cargo build --features async

# Release build with optimizations
cargo build --release --features sync
cargo build --release --features async

# Build all targets including examples
cargo build --all-targets --features sync
cargo build --all-targets --features async
```

### Running Tests

```bash
# Run sync tests
cargo test --features sync

# Run async tests
cargo test --features async

# Run specific test
cargo test test_name --features sync

# Test specific module
cargo test --package ibapi module_name:: --features sync

# Run with output
cargo test --features sync -- --nocapture

# Run doctests only
cargo test --doc --features sync
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --features sync -- -D warnings
cargo clippy --features async -- -D warnings

# Generate coverage report
cargo tarpaulin -o html
# or using just
just cover
```

## Testing Patterns

See [docs/testing-patterns.md](testing-patterns.md) for the full fixture stratification (`MessageBusStub` for domain logic, `MemoryStream` for transport/connection, `spawn_handshake_listener` for `Client::connect*`). The short version: pick the lightest fixture that exercises the seam.

### Domain test pattern (`MessageBusStub`)

```rust
let message_bus = Arc::new(MessageBusStub {
    request_messages: RwLock::new(vec![]),
    response_messages: vec!["<scripted-response>".to_owned()],
});
let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
let result = client.some_method()?;
// assert request_messages records the encoded request
// assert result decoded the scripted response
```

### Table-Driven Tests

Use shared test tables for comprehensive coverage:

```rust
// common/test_tables.rs
pub const TEST_CASES: &[TestCase] = &[
    TestCase {
        name: "valid_request",
        input: Input { ... },
        expected: Expected { ... },
    },
    // more cases...
];

// In sync.rs and async.rs
#[test]
fn test_api() {
    for case in TEST_CASES {
        let result = run_test(case);
        assert_eq!(result, case.expected);
    }
}
```

### Testing RequestMessage Fields

Use direct indexing for precise field testing:

```rust
#[test]
fn test_message_format() {
    let request = create_request();
    
    assert_eq!(request[0], "MessageType");
    assert_eq!(request[1], "123");  // request_id
    assert_eq!(request[2], "value");
}
```

## Running Tests for Both Modes

Always test both implementations:

```bash
# Using just command
just test

# Or manually
cargo test --features sync
cargo test --features async

# Test everything (tests + clippy + fmt)
cargo fmt --check && \
cargo clippy --features sync -- -D warnings && \
cargo clippy --features async -- -D warnings && \
cargo test --features sync && \
cargo test --features async
```

## Continuous Integration

The project should pass these checks before merging:

1. **Formatting**: `cargo fmt --check`
2. **Linting**: `cargo clippy` for both features
3. **Tests**: All tests passing for both features
4. **Documentation**: `cargo doc` builds without warnings
5. **Examples**: All examples compile

## Performance Testing

For performance-critical code:

```bash
# Run benchmarks
cargo bench --features sync

# Profile with flamegraph
cargo flamegraph --features sync --example market_data
```

## Debugging

### Enable Debug Logging
```bash
RUST_LOG=debug cargo test --features sync -- --nocapture
RUST_LOG=ibapi=trace cargo run --example connect
```

### Record TWS Messages
```bash
IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --example market_data
```

This creates timestamped files with all TWS communication for debugging protocol issues.