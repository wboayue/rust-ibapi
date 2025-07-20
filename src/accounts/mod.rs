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

// Common implementation modules
mod common;

// Domain types
pub mod types;

use crate::contracts::Contract;
use serde::{Deserialize, Serialize};

// Public types - always available regardless of feature flags

#[derive(Debug, Default, Serialize, Deserialize)]
/// Account information as it appears in the TWS' Account Summary Window
pub struct AccountSummary {
    /// The account identifier.
    pub account: String,
    /// The account's attribute.
    pub tag: String,
    /// The account's attribute's value.
    pub value: String,
    /// The currency in which the value is expressed.
    pub currency: String,
}

/// Constants for account summary tags used in account summary requests.
/// These tags define which account information fields to retrieve.
pub struct AccountSummaryTags {}

impl AccountSummaryTags {
    pub const ACCOUNT_TYPE: &'static str = "AccountType";
    pub const NET_LIQUIDATION: &'static str = "NetLiquidation";
    pub const TOTAL_CASH_VALUE: &'static str = "TotalCashValue";
    pub const SETTLED_CASH: &'static str = "SettledCash";
    pub const ACCRUED_CASH: &'static str = "AccruedCash";
    pub const BUYING_POWER: &'static str = "BuyingPower";
    pub const EQUITY_WITH_LOAN_VALUE: &'static str = "EquityWithLoanValue";
    pub const PREVIOUS_EQUITY_WITH_LOAN_VALUE: &'static str = "PreviousEquityWithLoanValue";
    pub const GROSS_POSITION_VALUE: &'static str = "GrossPositionValue";
    pub const REQ_T_EQUITY: &'static str = "RegTEquity";
    pub const REQ_T_MARGIN: &'static str = "RegTMargin";
    pub const SMA: &'static str = "SMA";
    pub const INIT_MARGIN_REQ: &'static str = "InitMarginReq";
    pub const MAINT_MARGIN_REQ: &'static str = "MaintMarginReq";
    pub const AVAILABLE_FUNDS: &'static str = "AvailableFunds";
    pub const EXCESS_LIQUIDITY: &'static str = "ExcessLiquidity";
    pub const CUSHION: &'static str = "Cushion";
    pub const FULL_INIT_MARGIN_REQ: &'static str = "FullInitMarginReq";
    pub const FULL_MAINT_MARGIN_REQ: &'static str = "FullMaintMarginReq";
    pub const FULL_AVAILABLE_FUNDS: &'static str = "FullAvailableFunds";
    pub const FULL_EXCESS_LIQUIDITY: &'static str = "FullExcessLiquidity";
    pub const LOOK_AHEAD_NEXT_CHANGE: &'static str = "LookAheadNextChange";
    pub const LOOK_AHEAD_INIT_MARGIN_REQ: &'static str = "LookAheadInitMarginReq";
    pub const LOOK_AHEAD_MAINT_MARGIN_REQ: &'static str = "LookAheadMaintMarginReq";
    pub const LOOK_AHEAD_AVAILABLE_FUNDS: &'static str = "LookAheadAvailableFunds";
    pub const LOOK_AHEAD_EXCESS_LIQUIDITY: &'static str = "LookAheadExcessLiquidity";
    pub const HIGHEST_SEVERITY: &'static str = "HighestSeverity";
    pub const DAY_TRADES_REMAINING: &'static str = "DayTradesRemaining";
    pub const LEVERAGE: &'static str = "Leverage";

    pub const ALL: &'static [&'static str] = &[
        Self::ACCOUNT_TYPE,
        Self::NET_LIQUIDATION,
        Self::TOTAL_CASH_VALUE,
        Self::SETTLED_CASH,
        Self::ACCRUED_CASH,
        Self::BUYING_POWER,
        Self::EQUITY_WITH_LOAN_VALUE,
        Self::PREVIOUS_EQUITY_WITH_LOAN_VALUE,
        Self::GROSS_POSITION_VALUE,
        Self::REQ_T_EQUITY,
        Self::REQ_T_MARGIN,
        Self::SMA,
        Self::INIT_MARGIN_REQ,
        Self::MAINT_MARGIN_REQ,
        Self::AVAILABLE_FUNDS,
        Self::EXCESS_LIQUIDITY,
        Self::CUSHION,
        Self::FULL_INIT_MARGIN_REQ,
        Self::FULL_MAINT_MARGIN_REQ,
        Self::FULL_AVAILABLE_FUNDS,
        Self::FULL_EXCESS_LIQUIDITY,
        Self::LOOK_AHEAD_NEXT_CHANGE,
        Self::LOOK_AHEAD_INIT_MARGIN_REQ,
        Self::LOOK_AHEAD_MAINT_MARGIN_REQ,
        Self::LOOK_AHEAD_AVAILABLE_FUNDS,
        Self::LOOK_AHEAD_EXCESS_LIQUIDITY,
        Self::HIGHEST_SEVERITY,
        Self::DAY_TRADES_REMAINING,
        Self::LEVERAGE,
    ];
}

#[derive(Debug)]
pub enum AccountSummaryResult {
    /// Summary of account details such as net liquidation, cash balance, etc.
    Summary(AccountSummary),
    /// End marker for a batch of account summaries
    End,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PnL {
    /// DailyPnL for the position
    pub daily_pnl: f64,
    /// UnrealizedPnL total unrealized PnL for the position (since inception) updating in real time.
    pub unrealized_pnl: Option<f64>,
    /// RealizedPnL is the realized PnL for the position.
    pub realized_pnl: Option<f64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PnLSingle {
    /// Current size of the position
    pub position: f64,
    /// DailyPnL for the position
    pub daily_pnl: f64,
    /// UnrealizedPnL is the total unrealized PnL for the position (since inception) updating in real time
    pub unrealized_pnl: f64,
    /// RealizedPnL is the realized PnL for the position.
    pub realized_pnl: f64,
    /// Current market value of the position
    pub value: f64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Account holding position
    pub account: String,
    /// Contract
    pub contract: Contract,
    /// Number of shares held
    pub position: f64,
    /// Average cost of shares
    pub average_cost: f64,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum PositionUpdate {
    /// Update for a position in the account
    Position(Position),
    /// Indicates all positions have been transmitted
    PositionEnd,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum PositionUpdateMulti {
    Position(PositionMulti),
    PositionEnd,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PositionMulti {
    /// Account holding position
    pub account: String,
    /// Contract
    pub contract: Contract,
    /// Number of shares held
    pub position: f64,
    /// Average cost of shares
    pub average_cost: f64,
    /// Model code for the position
    pub model_code: String,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FamilyCode {
    /// Account ID for the account family
    pub account_id: String,
    /// Account family code
    pub family_code: String,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum AccountUpdate {
    AccountValue(AccountValue),
    PortfolioValue(AccountPortfolioValue),
    UpdateTime(AccountUpdateTime),
    End,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AccountValue {
    /// Key describing the value
    pub key: String,
    /// Value corresponding to the key
    pub value: String,
    /// Currency of the value
    pub currency: String,
    /// Account ID (optional)
    pub account: Option<String>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AccountPortfolioValue {
    /// Contract for the position
    pub contract: Contract,
    /// Number of shares held
    pub position: f64,
    /// Current market price of the contract
    pub market_price: f64,
    /// Current market value of the position (shares * market price)
    pub market_value: f64,
    /// Average cost per share
    pub average_cost: f64,
    /// Unrealized profit and loss
    pub unrealized_pnl: f64,
    /// Realized profit and loss
    pub realized_pnl: f64,
    /// Account holding the position
    pub account: Option<String>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AccountUpdateTime {
    /// Timestamp of the last account update
    pub timestamp: String,
}

#[derive(Debug, PartialEq)]
pub enum AccountUpdateMulti {
    AccountMultiValue(AccountMultiValue),
    End,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AccountMultiValue {
    /// Account ID
    pub account: String,
    /// Model code
    pub model_code: String,
    /// Key describing the value
    pub key: String,
    /// Value corresponding to the key
    pub value: String,
    /// Currency of the value
    pub currency: String,
}

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

#[cfg(feature = "sync")]
pub use sync::{
    account_summary, account_updates, account_updates_multi, family_codes, managed_accounts, pnl, pnl_single, positions, positions_multi, server_time,
};

#[cfg(feature = "async")]
pub use r#async::{
    account_summary, account_updates, account_updates_multi, family_codes, managed_accounts, pnl, pnl_single, positions, positions_multi, server_time,
};
