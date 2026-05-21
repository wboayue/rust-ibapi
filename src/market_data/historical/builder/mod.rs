//! Fluent builders for the historical-data API surface.
//!
//! Each builder mirrors the `RealtimeBarsBuilder` / `MarketDataBuilder` shape:
//! generic over `'a` + client type, defaults in `new()`, `mut self`-returning
//! setters, per-feature terminal `impl` blocks. See
//! [`HistoricalScheduleBuilder`] for the canonical example.

pub mod data;
pub mod schedule;
pub mod ticks;
pub use data::HistoricalDataBuilder;
pub use schedule::HistoricalScheduleBuilder;
pub use ticks::{HistoricalTicksBuilder, IgnoreSize};
