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
    /// Identifies the account type (e.g. cash, margin, IRA).
    pub const ACCOUNT_TYPE: &'static str = "AccountType";
    /// Net liquidation value of the account including cash and positions.
    pub const NET_LIQUIDATION: &'static str = "NetLiquidation";
    /// Total cash across currencies converted to the base currency.
    pub const TOTAL_CASH_VALUE: &'static str = "TotalCashValue";
    /// Settled cash available for trading.
    pub const SETTLED_CASH: &'static str = "SettledCash";
    /// Accrued cash such as interest or dividends due.
    pub const ACCRUED_CASH: &'static str = "AccruedCash";
    /// Maximum capital available to open new positions.
    pub const BUYING_POWER: &'static str = "BuyingPower";
    /// Equity with loan value after margin calculations.
    pub const EQUITY_WITH_LOAN_VALUE: &'static str = "EquityWithLoanValue";
    /// Equity with loan value recorded on the previous trading day.
    pub const PREVIOUS_EQUITY_WITH_LOAN_VALUE: &'static str = "PreviousEquityWithLoanValue";
    /// Gross market value of all positions.
    pub const GROSS_POSITION_VALUE: &'static str = "GrossPositionValue";
    /// Regulation-T equity available in the account.
    pub const REQ_T_EQUITY: &'static str = "RegTEquity";
    /// Regulation-T margin requirement.
    pub const REQ_T_MARGIN: &'static str = "RegTMargin";
    /// Special Memorandum Account value as defined by Regulation-T.
    pub const SMA: &'static str = "SMA";
    /// Initial margin requirement for current positions.
    pub const INIT_MARGIN_REQ: &'static str = "InitMarginReq";
    /// Maintenance margin requirement for current positions.
    pub const MAINT_MARGIN_REQ: &'static str = "MaintMarginReq";
    /// Funds currently available for trading.
    pub const AVAILABLE_FUNDS: &'static str = "AvailableFunds";
    /// Excess liquidity above maintenance requirements.
    pub const EXCESS_LIQUIDITY: &'static str = "ExcessLiquidity";
    /// Cushion percentage representing excess liquidity scaled by equity.
    pub const CUSHION: &'static str = "Cushion";
    /// Full initial margin requirement across all related accounts.
    pub const FULL_INIT_MARGIN_REQ: &'static str = "FullInitMarginReq";
    /// Full maintenance margin requirement across all related accounts.
    pub const FULL_MAINT_MARGIN_REQ: &'static str = "FullMaintMarginReq";
    /// Full funds available for trading across all related accounts.
    pub const FULL_AVAILABLE_FUNDS: &'static str = "FullAvailableFunds";
    /// Full excess liquidity across all related accounts.
    pub const FULL_EXCESS_LIQUIDITY: &'static str = "FullExcessLiquidity";
    /// Estimated time of the next margin change event.
    pub const LOOK_AHEAD_NEXT_CHANGE: &'static str = "LookAheadNextChange";
    /// Projected initial margin requirement at the next change.
    pub const LOOK_AHEAD_INIT_MARGIN_REQ: &'static str = "LookAheadInitMarginReq";
    /// Projected maintenance margin requirement at the next change.
    pub const LOOK_AHEAD_MAINT_MARGIN_REQ: &'static str = "LookAheadMaintMarginReq";
    /// Projected funds available for trading at the next change.
    pub const LOOK_AHEAD_AVAILABLE_FUNDS: &'static str = "LookAheadAvailableFunds";
    /// Projected excess liquidity at the next change.
    pub const LOOK_AHEAD_EXCESS_LIQUIDITY: &'static str = "LookAheadExcessLiquidity";
    /// Highest pending warning severity for the account.
    pub const HIGHEST_SEVERITY: &'static str = "HighestSeverity";
    /// Day trades remaining before hitting the PDT limit.
    pub const DAY_TRADES_REMAINING: &'static str = "DayTradesRemaining";
    /// Effective account leverage based on net liquidation.
    pub const LEVERAGE: &'static str = "Leverage";

    /// Convenience slice containing every supported account summary tag.
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

/// Result of an account summary request emitted by the [Client](crate::client::Client).
#[derive(Debug)]
pub enum AccountSummaryResult {
    /// Summary of account details such as net liquidation, cash balance, etc.
    Summary(AccountSummary),
    /// End marker for a batch of account summaries
    End,
}

/// Aggregated profit and loss metrics for the entire account.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PnL {
    /// DailyPnL for the position
    pub daily_pnl: f64,
    /// UnrealizedPnL total unrealized PnL for the position (since inception) updating in real time.
    pub unrealized_pnl: Option<f64>,
    /// RealizedPnL is the realized PnL for the position.
    pub realized_pnl: Option<f64>,
}

/// Real-time profit and loss metrics for a single position.
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

/// Open position held within the account.
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

/// Messages emitted while streaming position updates.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum PositionUpdate {
    /// Update for a position in the account
    Position(Position),
    /// Indicates all positions have been transmitted
    PositionEnd,
}

/// Messages emitted while streaming model-code scoped position updates.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum PositionUpdateMulti {
    /// Position update scoped to a specific account/model code pair.
    Position(PositionMulti),
    /// Indicates all positions have been transmitted.
    PositionEnd,
}

/// Position scoped to a specific account and model code.
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

/// Family code assigned to a group of accounts.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FamilyCode {
    /// Account ID for the account family
    pub account_id: String,
    /// Account family code
    pub family_code: String,
}

/// Account update events delivered while streaming high-level account data.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum AccountUpdate {
    /// Key/value update describing an account metric.
    AccountValue(AccountValue),
    /// Update describing a position's valuation data.
    PortfolioValue(AccountPortfolioValue),
    /// Timestamp indicating when the account snapshot was generated.
    UpdateTime(AccountUpdateTime),
    /// Indicates the end of the account update stream.
    End,
}

/// Single account value update emitted by the API.
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

/// Aggregated valuation details for a single contract within the account.
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

/// Timestamp wrapper for account update streams.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AccountUpdateTime {
    /// Timestamp of the last account update
    pub timestamp: String,
}

/// Account update events scoped to an account/model code pair.
#[derive(Debug, PartialEq)]
pub enum AccountUpdateMulti {
    /// Key/value update for a specific account/model code pair.
    AccountMultiValue(AccountMultiValue),
    /// Indicates the end of the scoped account update stream.
    End,
}

/// Key/value pair returned for a specific account/model code pair.
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
pub mod blocking {
    pub use super::sync::{
        account_summary, account_updates, account_updates_multi, family_codes, managed_accounts, pnl, pnl_single, positions, positions_multi,
        server_time,
    };
}

#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync::{
    account_summary, account_updates, account_updates_multi, family_codes, managed_accounts, pnl, pnl_single, positions, positions_multi, server_time,
};

#[cfg(feature = "async")]
pub use r#async::{
    account_summary, account_updates, account_updates_multi, family_codes, managed_accounts, pnl, pnl_single, positions, positions_multi, server_time,
};
