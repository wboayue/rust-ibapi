//! Async transport routing tests. Scaffold; the §2 routing scenarios
//! (framing, partial reads, multi-message coalescing, request_id correlation,
//! shared-channel fan-out, EOF) land in PR 2c using the now-generic
//! `AsyncTcpMessageBus<MemoryStream>` over `AsyncConnection<MemoryStream>`.
