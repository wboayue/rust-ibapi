# Build and Test Guide

> **`--features sync` is not the sync-only build.** `default = ["async"]` and cargo features
> are additive, so `--features sync` gives you sync **and** async. Every sync command below
> therefore uses `--no-default-features --features sync`. See
> [docs/rules/parity/feature-matrix.md](rules/parity/feature-matrix.md).

## Build Commands

### Basic Build
```bash
# Build with sync support
cargo build --no-default-features --features sync

# Build with async support
cargo build --features async

# Release build with optimizations
cargo build --release --no-default-features --features sync
cargo build --release --features async

# Build all targets including examples
cargo build --all-targets --no-default-features --features sync
cargo build --all-targets --features async
```

### Running Tests

```bash
# Run sync tests
cargo test --no-default-features --features sync

# Run async tests
cargo test --features async

# Run specific test
cargo test test_name --no-default-features --features sync

# Test specific module
cargo test --package ibapi module_name:: --no-default-features --features sync

# Run with output
cargo test --no-default-features --features sync -- --nocapture

# Run doctests only
cargo test --doc --no-default-features --features sync
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --no-default-features --features sync -- -D warnings
cargo clippy --features async -- -D warnings

# Generate coverage report (nightly is required for --doctests)
cargo +nightly llvm-cov --all-features --doctests --html --open
# or using just
just cover
```

## Testing Patterns

See [docs/testing-patterns.md](testing-patterns.md) for the full fixture stratification (`MessageBusStub` for domain logic, `MemoryStream` for transport/connection, `spawn_handshake_listener` for `Client::connect*`). The short version: pick the lightest fixture that exercises the seam.

### Domain test pattern (`MessageBusStub`)

```rust
let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
    IncomingMessages::OrderStatus,
    order_status().order_id(1).status(OrderStatusKind::Submitted).encode_proto(),
)]));
let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
let result = client.some_method()?;
// assert request_messages records the encoded request
// assert result decoded the scripted response
```

Responses are proto-framed: `proto_response(...)` takes bytes from a field-minimal builder in `src/testdata/builders/<domain>.rs`. A text-framed fixture reaching a proto-only decoder is skip-classified, so the test passes with its assertions unrun.

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

## Running Tests for Every Configuration

Three configurations, not two — async-only, sync-only, and both-plus-`utoipa`. A type or impl
can compile in two of them and fail the third:

```bash
# Using just command (runs all three)
just test

# Or manually
cargo test                                          # async only (default)
cargo test --no-default-features --features sync     # sync only
cargo test --all-features                            # sync + async + utoipa

# Test everything (tests + clippy + fmt)
cargo fmt --check && \
cargo clippy --all-targets -- -D warnings && \
cargo clippy --all-targets --no-default-features --features sync -- -D warnings && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test && \
cargo test --no-default-features --features sync && \
cargo test --all-features
```

This chain does not cover intra-doc links or the integration crates. For the full pre-PR gate,
see [pre-PR checks](rules/workflow/pre-pr-checks.md).

## Continuous Integration

`ci.yml` runs one matrix leg per configuration — `async`, `sync`, and `all-features` — and each
leg runs the full sequence:

1. **Formatting**: `cargo fmt --check`
2. **Linting**: `cargo clippy --all-targets … -- -D warnings`
3. **Tests**: `cargo test`
4. **Documentation**: `cargo doc --no-deps` — note this leg runs **without** `RUSTDOCFLAGS`, so
   a broken intra-doc link warns and still passes. Catching those is local-only; see
   [pre-PR checks](rules/workflow/pre-pr-checks.md)
5. **Examples**: `cargo build --examples`
6. **Benches**: `cargo check --benches` (non-blocking)

The legs spell their flags out rather than interpolating a feature name, because
`--features sync` would silently leave the async client enabled — the bug that let a sync-only
break sit unnoticed through 11 merges (#658 → #671).

## Performance Testing

For performance-critical code:

```bash
# Run benchmarks
cargo bench --no-default-features --features sync

# Profile with flamegraph
cargo flamegraph --no-default-features --features sync --example market_data
```

## Debugging

### Enable Debug Logging
```bash
RUST_LOG=debug cargo test --no-default-features --features sync -- --nocapture
RUST_LOG=ibapi=trace cargo run --example connect
```

### Record TWS Messages
```bash
IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --example market_data
```

This creates timestamped files with all TWS communication for debugging protocol issues.