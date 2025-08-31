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
pub use crate::market_data::{MarketDataType, TradingHours};

// Order types
#[cfg(feature = "sync")]
pub use crate::orders::{order_builder, Action, ExecutionFilter, OrderUpdate, Orders, PlaceOrder};

// Account types
pub use crate::accounts::{
    AccountSummaryResult, AccountSummaryTags, AccountUpdate, AccountUpdateMulti, FamilyCode, PnL, PnLSingle, PositionUpdate, PositionUpdateMulti,
};

// Client subscription type
#[cfg(feature = "sync")]
pub use crate::client::Subscription;
#[cfg(feature = "async")]
pub use crate::subscriptions::Subscription;

// Async-specific imports
#[cfg(feature = "async")]
pub use futures::StreamExt;
