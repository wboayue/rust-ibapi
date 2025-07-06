//! # Account Management
//!
//! This module provides functionality for managing positions and profit and loss (PnL)
//! information in a trading system. It includes structures and implementations for:
//!
//! - Position tracking
//! - Daily, unrealized, and realized PnL calculations
//! - Family code management
//! - Real-time PnL updates for individual positions
//!

// Common modules used by both sync and async
mod decoders;
mod encoders;
mod types;

// Re-export common types that are available regardless of sync/async
pub use types::*;

// Feature-specific implementations
#[cfg(all(feature = "sync", not(feature = "async")))]
mod sync;

#[cfg(feature = "async")]
mod r#async;

#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{
    account_summary, account_updates, account_updates_multi, family_codes, managed_accounts, pnl, pnl_single, positions, positions_multi, server_time,
};

#[cfg(feature = "async")]
pub use r#async::{
    account_summary, account_updates, account_updates_multi, family_codes, managed_accounts, pnl, pnl_single, positions, positions_multi, server_time,
};
