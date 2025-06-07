//! A prelude module for convenient importing of commonly used types and traits.
//!
//! This module re-exports the most frequently used types from the ibapi crate
//! to simplify imports in user code. Instead of importing each type individually:
//!
//! ```rust
//! use ibapi::Client;
//! use ibapi::contracts::Contract;
//! use ibapi::orders::{Action, PlaceOrder};
//! use ibapi::market_data::historical::{BarSize, WhatToShow, ToDuration};
//! ```
//!
//! You can simply use:
//!
//! ```rust
//! use ibapi::prelude::*;
//! ```

// Core client
pub use crate::Client;
pub use crate::Error;

// Contract types
pub use crate::contracts::{Contract, SecurityType};

// Market data types - historical
pub use crate::market_data::historical::{BarSize as HistoricalBarSize, ToDuration, WhatToShow as HistoricalWhatToShow};

// Market data types - realtime
pub use crate::market_data::realtime::{BarSize as RealtimeBarSize, TickTypes, WhatToShow as RealtimeWhatToShow};
pub use crate::market_data::MarketDataType;

// Order types
pub use crate::orders::{order_builder, Action, ExecutionFilter, Orders, PlaceOrder};

// Account types
pub use crate::accounts::{AccountSummaries, AccountSummaryTags, AccountUpdate, AccountUpdateMulti, PositionUpdate};

// Client subscription type
pub use crate::client::Subscription;
