//! Market scanner functionality for discovering trading opportunities.
//!
//! This module provides access to Interactive Brokers' market scanner,
//! allowing users to scan for stocks and other instruments based on
//! various criteria and filters.

use serde::{Deserialize, Serialize};

// Common implementation modules
mod common;

// Feature-specific implementations
#[cfg(all(feature = "sync", not(feature = "async")))]
mod sync;

#[cfg(feature = "async")]
mod r#async;

// Public types - always available regardless of feature flags

/// Scanner subscription parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScannerSubscription {
    /// The number of rows to be returned for the query
    pub number_of_rows: i32,
    /// The instrument's type for the scan. I.e. STK, FUT.HK, etc.
    pub instrument: Option<String>,
    /// The request's location (STK.US, STK.US.MAJOR, etc).
    pub location_code: Option<String>,
    /// Same as TWS Market Scanner's "parameters" field, for example: TOP_PERC_GAIN
    pub scan_code: Option<String>,
    /// Filters out Contracts which price is below this value
    pub above_price: Option<f64>,
    /// Filters out contracts which price is above this value.
    pub below_price: Option<f64>,
    /// Filters out Contracts which volume is above this value.
    pub above_volume: Option<i32>,
    /// Filters out Contracts which option volume is above this value.
    pub average_option_volume_above: Option<i32>,
    /// Filters out Contracts which market cap is above this value.
    pub market_cap_above: Option<f64>,
    /// Filters out Contracts which market cap is below this value.
    pub market_cap_below: Option<f64>,
    /// Filters out Contracts which Moody's rating is below this value.
    pub moody_rating_above: Option<String>,
    /// Filters out Contracts which Moody's rating is above this value.
    pub moody_rating_below: Option<String>,
    /// Filters out Contracts with a S&P rating below this value.
    pub sp_rating_above: Option<String>,
    /// Filters out Contracts with a S&P rating above this value.
    pub sp_rating_below: Option<String>,
    /// Filter out Contracts with a maturity date earlier than this value.
    pub maturity_date_above: Option<String>,
    /// Filter out Contracts with a maturity date older than this value.
    pub maturity_date_below: Option<String>,
    /// Filter out Contracts with a coupon rate lower than this value.
    pub coupon_rate_above: Option<f64>,
    /// Filter out Contracts with a coupon rate higher than this value.
    pub coupon_rate_below: Option<f64>,
    /// Filters out Convertible bonds
    pub exclude_convertible: bool,
    /// For example, a pairing "Annual, true" used on the "top Option Implied Vol % Gainers" scan would return annualized volatilities.
    pub scanner_setting_pairs: Option<String>,
    /// CORP = Corporation, ADR = American Depositary Receipt, ETF = Exchange Traded Fund, REIT = Real Estate Investment Trust, CEF = Closed End Fund
    pub stock_type_filter: Option<String>,
}

impl Default for ScannerSubscription {
    fn default() -> Self {
        ScannerSubscription {
            number_of_rows: -1,
            instrument: None,
            location_code: None,
            scan_code: None,
            above_price: None,
            below_price: None,
            above_volume: None,
            average_option_volume_above: None,
            market_cap_above: None,
            market_cap_below: None,
            moody_rating_above: None,
            moody_rating_below: None,
            sp_rating_above: None,
            sp_rating_below: None,
            maturity_date_above: None,
            maturity_date_below: None,
            coupon_rate_above: None,
            coupon_rate_below: None,
            exclude_convertible: false,
            scanner_setting_pairs: None,
            stock_type_filter: None,
        }
    }
}

/// Provides the data resulting from the market scanner request.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ScannerData {
    /// The ranking position of the contract in the scanner sort.
    pub rank: i32,
    /// The contract matching the scanner subscription.
    pub contract_details: crate::contracts::ContractDetails,
    /// Describes the combo legs when the scanner is returning EFP.
    pub leg: String,
}

// Re-export API functions based on active feature
#[cfg(all(feature = "sync", not(feature = "async")))]
pub(crate) use sync::{scanner_parameters, scanner_subscription};

#[cfg(feature = "async")]
pub(crate) use r#async::{scanner_parameters, scanner_subscription};