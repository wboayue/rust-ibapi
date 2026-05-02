//! Async-connection tests. Scaffold landed in PR 2 of eliminate-mock-gateway;
//! handshake / connect / disconnect / reconnect scenarios from §2 of the plan
//! land in PR 4.
//!
//! `AsyncConnection` is concrete on tokio's `OwnedReadHalf`/`OwnedWriteHalf`,
//! so plugging the async `MemoryStream` in requires generalizing
//! `AsyncConnection`'s reader/writer types first. That refactor and the
//! tests it unblocks both belong to PR 4.
