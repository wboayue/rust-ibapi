//! Sync-connection tests. Scaffold landed in PR 2 of eliminate-mock-gateway;
//! handshake / connect / disconnect / reconnect scenarios from §2 of the plan
//! land in PR 4.

use super::*;
use crate::tests::assert_send_and_sync;
use crate::transport::sync::MemoryStream;

/// Connection's `S: Stream` bound is satisfied by the new `MemoryStream` fixture.
/// Compiling this assertion is the wiring smoke-check; PR 4 adds the real
/// connect/handshake scenarios that drive scripted responses through it.
#[test]
fn connection_with_memory_stream_is_send_and_sync() {
    assert_send_and_sync::<Connection<MemoryStream>>();
}
