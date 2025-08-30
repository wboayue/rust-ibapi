# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Quick Start

The rust-ibapi crate is a Rust implementation of the Interactive Brokers TWS API with both synchronous and asynchronous support.

```bash
# Build with sync support
cargo build --features sync

# Build with async support  
cargo build --features async

# Run tests
cargo test --features sync
cargo test --features async
```

## Documentation Index

### Core Concepts
- [**Architecture Overview**](docs/claude/architecture.md) - System design, components, and module organization
- [**Feature Flags**](docs/claude/feature-flags.md) - Sync vs async modes and feature guards
- [**API Patterns**](docs/claude/api-patterns.md) - Builder patterns, protocol versions, and common patterns

### Development
- [**Code Style Guidelines**](docs/claude/code-style.md) - Coding standards and conventions
- [**Build and Test**](docs/claude/build-and-test.md) - Build commands, testing patterns, and CI
- [**Examples Guide**](docs/claude/examples.md) - Running and writing examples

## Key Points to Remember

1. **Always specify a feature**: The crate requires either `sync` or `async` feature flag
2. **Test both modes**: Changes should work for both sync and async implementations
3. **Follow module structure**: Use the common pattern for shared logic between sync/async
4. **Minimal comments**: Keep comments concise, avoid stating the obvious
5. **Run quality checks**: Before committing, run `cargo fmt`, `cargo clippy --features sync`, and `cargo clippy --features async`

## Connection Settings

When running examples or tests:
- **IB Gateway Paper Trading**: 127.0.0.1:4002 (recommended)
- **IB Gateway Live Trading**: 127.0.0.1:4001
- **TWS Paper Trading**: 127.0.0.1:7497
- **TWS Live Trading**: 127.0.0.1:7496

## Environment Variables

```bash
# Set log level
RUST_LOG=debug cargo run --example <example_name>

# Record TWS messages for debugging
IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --example <example_name>
```

## Quick Commands

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --features sync
cargo clippy --features async

# Run all tests
just test

# Generate coverage report
just cover
```

For detailed information on any topic, refer to the linked documentation files above.