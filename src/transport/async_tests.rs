//! Async transport routing tests. Scaffold landed in PR 2 of
//! eliminate-mock-gateway; the §2 routing scenarios (framing, partial reads,
//! multi-message coalescing, request_id correlation, etc.) land in PR 2c
//! once `AsyncTcpMessageBus` can be constructed over a `MemoryStream`-backed
//! `AsyncConnection`.
