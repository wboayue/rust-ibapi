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
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "sync")]
pub use sync::{
    positions, positions_multi, family_codes, pnl, pnl_single,
    account_summary, account_updates, account_updates_multi,
    managed_accounts, server_time
};