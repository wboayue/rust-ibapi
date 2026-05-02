//! Async-connection tests. Routing-test scaffold; specific handshake /
//! connect / disconnect / reconnect scenarios from §2 of
//! `todos/eliminate-mock-gateway.md` land in PR 4.

use super::*;
use crate::transport::r#async::MemoryStream;

/// `AsyncConnection`'s `S: AsyncStream` bound is satisfied by the in-memory
/// fixture. Compiling `stubbed` over `MemoryStream` is the wiring smoke check;
/// PR 4 adds the real connect/handshake scenarios.
#[test]
fn connection_with_memory_stream_compiles() {
    let _conn = AsyncConnection::stubbed(MemoryStream::default(), 28);
}
