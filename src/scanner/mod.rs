//! Market scanner functionality for discovering trading opportunities.
//!
//! This module provides access to Interactive Brokers' market scanner,
//! allowing users to scan for stocks and other instruments based on
//! various criteria and filters.

use serde::{Deserialize, Serialize};

use crate::contracts::TagValue;

// Common implementation modules
mod common;
// Test fixtures encode the request body with the production encoder.
#[cfg(test)]
pub(crate) use common::encoders::encode_subscription_params;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

// Public types - always available regardless of feature flags

/// Scanner subscription parameters.
///
/// [`new`](Self::new) takes the two fields TWS requires, the instrument and the scan code; every
/// other field is a public filter to assign. The codes (instruments, locations, scan codes, filter
/// tags) are listed in the XML that `Client::scanner_parameters` returns.
///
/// # Examples
///
/// ```
/// use ibapi::scanner::ScannerSubscription;
/// use time::macros::date;
///
/// let mut scan = ScannerSubscription::new("BOND", "NEAR_MATURITY_DATE");
/// scan.location_code = Some("BOND.US".to_string());
/// scan.number_of_rows = 10;
/// scan.maturity_date_above = Some(date!(2030 - 01 - 01));
/// assert_eq!(scan.instrument, "BOND");
/// ```
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScannerSubscription {
    /// The number of rows to be returned for the query
    pub number_of_rows: i32,
    /// The instrument's type for the scan. I.e. STK, FUT.HK, etc. Required: TWS rejects a scan
    /// without one with error 162 (`docs/wire-facts/scanner.md#scanner-required-fields-and-maturity-filters`).
    pub instrument: String,
    /// The request's location (STK.US, STK.US.MAJOR, etc). TWS accepts a scan without one.
    pub location_code: Option<String>,
    /// Same as TWS Market Scanner's "parameters" field, for example: TOP_PERC_GAIN. Required:
    /// TWS rejects a scan without one with error 162.
    pub scan_code: String,
    /// Filters out Contracts which price is below this value
    pub above_price: Option<f64>,
    /// Filters out contracts which price is above this value.
    pub below_price: Option<f64>,
    /// Filters out Contracts which volume is above this value.
    pub above_volume: Option<i64>,
    /// Filters out Contracts which option volume is above this value.
    pub average_option_volume_above: Option<i64>,
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
    /// Filter out Contracts with a maturity date earlier than this value. Sent as `YYYYMMDD`,
    /// the form TWS applies (live: 2026-11-15, paper).
    pub maturity_date_above: Option<time::Date>,
    /// Filter out Contracts with a maturity date older than this value. Sent as `YYYYMMDD`.
    pub maturity_date_below: Option<time::Date>,
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
    /// Further filters as tag-value pairs, for example `marketCapAbove` = `1000000000`. The tags
    /// are the filter codes in the `Client::scanner_parameters` XML. Optional when deserializing.
    #[serde(default)]
    pub filter_options: Vec<TagValue>,
}

impl ScannerSubscription {
    /// A scan of `instrument` (`"STK"`) by `scan_code` (`"TOP_PERC_GAIN"`), the two fields TWS
    /// requires, with no location, no filters and `number_of_rows` `-1`.
    pub fn new(instrument: impl Into<String>, scan_code: impl Into<String>) -> Self {
        ScannerSubscription {
            number_of_rows: -1,
            instrument: instrument.into(),
            location_code: None,
            scan_code: scan_code.into(),
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
            filter_options: Vec::new(),
        }
    }
}

/// Provides the data resulting from the market scanner request.
///
/// The proto's `distance`, `benchmark` and `projection` are not decoded: IBKR documents
/// `distance` and `projection` as internal use only and `benchmark` as always empty, and a live
/// gateway sends none of them (`docs/wire-facts/scanner.md`).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ScannerData {
    /// The ranking position of the contract in the scanner sort.
    pub rank: i32,
    /// The contract matching the scanner subscription.
    pub contract_details: crate::contracts::ContractDetails,
    /// Describes the combo legs when the scanner is returning EFP.
    pub leg: String,
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
