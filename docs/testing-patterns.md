# Testing Patterns

This document describes the test-fixture strategy for the rust-ibapi crate. Tests are stratified by which seam they exercise — pick the lightest fixture that does the job.

## Three fixtures, three scopes

| Fixture | Scope | Where it lives | Use when |
| --- | --- | --- | --- |
| `MessageBusStub` | Domain logic | `src/stubs.rs` | Testing methods on `Client` (e.g. `realtime_bars`, `place_order`) — verify request encoding and response decoding through the `MessageBus` / `AsyncMessageBus` trait. Skips the dispatcher and framing entirely. |
| `MemoryStream` | Transport / connection | `src/transport/sync/memory.rs`, `src/transport/async_memory.rs` | Testing the dispatcher (routing, cancel coalescing, EOF handling) or the handshake (`establish_connection`, disconnect, reconnect). Implements the `Stream` / `AsyncStream` trait so it slots into `Connection<S>` / `AsyncConnection<S>` directly. |
| `spawn_handshake_listener` | Production TCP entry points | `src/transport/sync/test_listener.rs`, `src/transport/async_test_listener.rs` | Testing `Client::connect*` and `AsyncTcpSocket::*` — the production-only seam that does `TcpStream::connect(addr)`. One-shot listener bound to `127.0.0.1:0`. |

## Pattern 1: `MessageBusStub` for domain tests

Most per-domain tests use this. The stub records outbound `request_messages` and replays scripted `response_messages` through whatever channel kind the request expects (request/order/shared).

```rust
// src/orders/sync/tests.rs
#[test]
fn place_order() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "5|2|637533641|ES|FUT|...".to_owned(),  // OpenOrder
            "3|1|Submitted|0|1|0|...".to_owned(),    // OrderStatus
        ],
    });
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let mut subscription = client.place_order(1, &contract, &order).expect("...");
    // assert subscription yields decoded responses
    // assert message_bus.request_messages records the encoded request
}
```

Counterpart `MessageBusStub::default()` exists for tests that just need a `Client` (accessor tests, builder smoke tests).

## Pattern 2: `MemoryStream` for transport / connection tests

`MemoryStream` is a frame-level in-memory implementation of the `Stream` / `AsyncStream` trait. Tests `push_inbound(body)` to script response frames, and call `captured()` to read back what the consumer wrote. `close()` signals EOF.

```rust
// src/connection/sync_tests.rs
#[test]
fn establish_connection_populates_metadata() {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), CLIENT_ID);
    push_handshake(&stream);  // pre-pushes 3 frames

    connection.establish_connection(None).expect("...");
    assert_eq!(connection.server_version(), server_versions::PROTOBUF);
}
```

Use it for:
- Handshake (handshake response, NextValidId, ManagedAccounts, version-error, unknown-timezone)
- Dispatcher routing (request_id correlation, order_id correlation, shared-channel fan-out, cancel coalescing)
- Disconnect / reconnect lifecycle (push the `-2` shutdown sentinel for clean disconnect; close the stream for EOF/reconnect-fail paths)

## Pattern 3: `spawn_handshake_listener` for `Client::connect*`

Real TCP listener that binds `127.0.0.1:0`, accepts once, replays scripted handshake frames, and drains further writes until the client closes. Used only at the production-TCP entry-point seam.

```rust
// src/client/sync_tests.rs
#[test]
fn connect_handshakes_against_real_socket() {
    let (addr, _h) = spawn_handshake_listener(handshake_frames());
    let client = Client::connect(&addr.to_string(), 100).expect("Client::connect");
    assert_eq!(client.client_id(), 100);
}
```

This is *not* a re-implementation of MockGateway — it has no per-API-call interaction surface. It exists only because `Client::connect`, `connect_with_callback`, `connect_with_options`, and the underlying `TcpSocket::connect` / `AsyncTcpSocket::connect` cannot otherwise be exercised without a real socket.

## Picking the right fixture

When in doubt, default to the lightest:

1. **Are you testing a `Client` method that talks to TWS via the bus?** → `MessageBusStub`.
2. **Are you testing the dispatcher, handshake, or disconnect?** → `MemoryStream`.
3. **Are you testing `Client::connect*` itself or `AsyncTcpSocket`?** → `spawn_handshake_listener`.

Going heavier than necessary adds threads, ports, or framing that doesn't earn its keep.

## Test file layout

- Tests live in their own files — never inline `#[cfg(test)] mod tests { ... }` blocks alongside implementation.
- Prefer flat sibling files (`foo.rs` + `foo_tests.rs`) over a nested module directory (`foo/mod.rs` + `foo/tests.rs`). Wire from the implementation file:
  ```rust
  #[cfg(test)]
  #[path = "foo_tests.rs"]
  mod tests;
  ```
- For domain submodules, the `#[path = "..."] mod tests;` declaration can live in the parent `mod.rs`.

## Table-driven tests

Shared test tables in `<domain>/common/test_tables.rs` are exercised from both `<domain>/sync/tests.rs` and `<domain>/async/tests.rs` to enforce sync/async parity:

```rust
// common/test_tables.rs
pub const TEST_CASES: &[TestCase] = &[
    TestCase { name: "...", input: ..., expected: ... },
    // ...
];

// sync/tests.rs and async/tests.rs both iterate TEST_CASES with the same assertions.
```

## Running tests

```bash
# Default (async)
cargo test

# Sync only
cargo test --no-default-features --features sync

# Both
cargo test --all-features

# Coverage
cargo llvm-cov --all-features --summary-only      # text summary
just cover                                         # HTML report, opens browser

# Specific module
cargo test --features sync client::sync::tests
```

Per CLAUDE.md item 5, every PR should pass `cargo clippy` in all three configurations: default, `--features sync`, `--all-features`.

## Recording real messages

When implementing tests for a new feature, capture real protocol bytes against a paper IB Gateway with `IBAPI_RECORDING_DIR=/tmp/tws-messages`, then use those captured frames in `MessageBusStub::response_messages` or `MemoryStream::push_inbound` calls.
