//! Fluent builders for the real-time market-data API surface.
//!
//! Each builder mirrors the canonical shape: generic over `'a` + client
//! type, defaults in `new()`, `mut self`-returning setters, per-feature
//! terminal `impl` blocks. See [`RealtimeBarsBuilder`] for the precedent.

pub mod bars;
pub mod market_depth;
pub mod tick_by_tick;
pub use bars::RealtimeBarsBuilder;
pub use market_depth::MarketDepthBuilder;
pub use tick_by_tick::TickByTickBuilder;
