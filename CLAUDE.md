# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build the library
cargo build

# Build with optimizations
cargo build --release

# Build all targets including examples
cargo build --all-targets

# Run tests
cargo test

# Run a specific test
cargo test <test_name>

# Run tests in a specific module
cargo test --package ibapi <module>::

# Run tests with verbose output
cargo test -- --nocapture

# Check code with clippy
cargo clippy

# Format code
cargo fmt

# Generate code coverage report
cargo tarpaulin -o html
# or
just cover
```

## Environment Variables for Debugging

```bash
# Set log level (trace, debug, info, warn, error)
RUST_LOG=debug cargo run --example <example_name>

# Log messages between library and TWS to a directory
IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --example <example_name>
```

## Architecture Overview

The rust-ibapi crate is a Rust implementation of the Interactive Brokers TWS API. The architecture is built around threads and channels for communication.

### Core Components

1. **Client** - Main interface for user interactions, running on the main thread.
   - Encodes user requests
   - Sends requests to the MessageBus
   - Receives responses via channels
   - Decodes responses for the user

2. **MessageBus** - Runs on a dedicated thread with responsibilities:
   - Establishes and maintains connection to TWS
   - Sends messages from client to TWS
   - Listens for messages from TWS
   - Routes incoming messages to appropriate client channels

3. **Request/Response Flow**:
   - For requests with IDs: MessageBus creates dedicated channels for responses
   - For requests without IDs: MessageBus uses shared channels for responses

4. **Key Modules**:
   - `accounts`: Account-related functionality
   - `contracts`: Contract definitions and operations
   - `market_data`: Real-time and historical market data
   - `orders`: Order management functionality
   - `news`: News-related functionality
   - `transport`: Connection and message handling
   - `messages`: Message definitions and routing

## Multi-Threading Model

The `Client` can be shared between threads for concurrent operations:

1. **Thread-Safe Design**: Use `Arc<Client>` when sharing a client across threads
2. **One Client Per Thread**: Recommended for operations with shared channels
3. **Subscription Model**: Subscriptions can be converted to iterators for convenient data handling

## Important Design Considerations

1. **Message Handling**: Some TWS API calls don't have unique request IDs and are mapped by message type. These shared channels can cause confusion if multiple concurrent requests of the same type are made.

2. **Fault Tolerance**: The API automatically attempts to reconnect on disconnection, using a Fibonacci backoff strategy.

3. **Testing**: For important changes, ensure tests are passing and coverage is maintained. Use `cargo tarpaulin` to generate coverage reports.

4. **Examples**: Numerous examples in the `/examples` directory demonstrate API usage.