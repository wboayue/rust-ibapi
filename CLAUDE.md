# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Quick Start

The rust-ibapi crate is a Rust implementation of the Interactive Brokers TWS API with both synchronous and asynchronous support.

**Important:** The async client is enabled by default. You can opt into the blocking client with `--features sync`, and the two features may be combined:
- `cargo build` (default features) exposes the async client as `client::Client`
- `cargo build --no-default-features --features sync` enables only the blocking client
- `cargo build --no-default-features --features "sync async"` enables both; the blocking API lives under `client::blocking::Client`

```bash
# Build with async support (default)
cargo build

# Build with sync support only
cargo build --no-default-features --features sync

# Build with both clients
cargo build --no-default-features --features "sync async"

# Run tests (cover every configuration)
cargo test
cargo test --no-default-features --features sync
cargo test --all-features
```

## Documentation Index

### Getting Started
- [**Quick Start Guide**](docs/quick-start.md) - Get up and running in minutes
- [**Examples Guide**](docs/examples.md) - Running and writing examples
- [**Troubleshooting**](docs/troubleshooting.md) - Common issues and solutions

### Core Concepts
- [**Architecture Overview**](docs/architecture.md) - System design, components, and module organization
- [**Feature Flags**](docs/feature-flags.md) - Sync vs async modes and feature guards
- [**API Patterns**](docs/api-patterns.md) - Builder patterns, protocol versions, and common patterns

### Development
- [**Code Style Guidelines**](docs/code-style.md) - Coding standards and conventions
- [**Build and Test**](docs/build-and-test.md) - Build commands, testing patterns, and CI
- [**Testing Patterns**](docs/testing-patterns.md) - Table-driven tests and MockGateway
- [**Extending the API**](docs/extending-api.md) - Adding new TWS API functionality

## Key Points to Remember

1. **Be explicit about feature coverage**: Default async, sync-only, and combined builds must compile when touched
2. **Test each configuration**: Run tests for default, sync-only, and `--all-features`
3. **Follow module structure**: Use the common pattern for shared logic between sync/async
4. **Minimal comments**: Keep comments concise, avoid stating the obvious
5. **Run quality checks**: Before committing, run `cargo fmt`, `cargo clippy --features sync`, and `cargo clippy --features async`
6. **Fluent conditional orders**: Use helper functions (`price()`, `time()`, `margin()`, etc.) and method chaining (`.condition()`, `.and_condition()`, `.or_condition()`) for building conditional orders. See [docs/order-types.md](docs/order-types.md#conditional-orders-with-conditions) and [docs/api-patterns.md](docs/api-patterns.md#conditional-order-builder-pattern) for details
7. **Don't repeat code**: Extract repeated logic to `common/`; use shared helpers like `request_helpers`
8. **Single responsibility**: One responsibility per function/module; split orchestration from business logic
9. **Composition**: Single responsibility per struct; use builders for complex construction; max 3 params per function (use builder if 4+)

See [docs/code-style.md](docs/code-style.md#design-principles) for detailed design guidelines.

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

# Run clippy (cover every configuration)
cargo clippy
cargo clippy --no-default-features --features sync
cargo clippy --all-features

# Run all tests
just test

# Generate coverage report (opens HTML report in browser)
just cover
```

For detailed information on any topic, refer to the linked documentation files above.

## Git Commit Guidelines

- DO NOT include "Generated with Claude Code" or similar attribution in commit messages
- Keep commit messages focused on the technical changes and their purpose
