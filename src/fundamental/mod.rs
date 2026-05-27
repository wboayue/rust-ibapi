//! Fundamental company data: financial summaries, snapshots, ratios, and analyst
//! estimates retrieved from Interactive Brokers.
//!
//! The payload is an XML string sourced from Reuters; this crate does not
//! attempt to parse it — consumers feed `FundamentalData::data` into the XML
//! parser of their choice.

use serde::{Deserialize, Serialize};

// Common implementation modules
mod common;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

/// Fundamental data report payload as returned by IBKR.
///
/// `data` carries the report as an XML string. The schema varies by
/// [`FundamentalReportType`] and is documented by Reuters.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct FundamentalData {
    /// The XML report body. Empty when the server returned an empty payload.
    pub data: String,
}

/// Which Reuters fundamental report to request.
///
/// The wire vocabulary is fixed; unknown strings are rejected by
/// [`FromStr`](std::str::FromStr) as [`Error::Parse`](crate::Error::Parse).
/// Verified against IB Gateway server v220: each variant produced either
/// a populated XML response or a documented entitlement error (CalendarReport
/// — news feed subscription required). `ReportRatios` and
/// `ReportsFinStatements` from older IBKR docs are not accepted by TWS today
/// (code 430 "Missing reportType") and are intentionally omitted.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FundamentalReportType {
    /// Financial summary — wire value `"ReportsFinSummary"`.
    #[default]
    ReportsFinSummary,
    /// Company snapshot — wire value `"ReportSnapshot"`.
    ReportSnapshot,
    /// Analyst estimates (Reuters Estimates System Consensus) — wire value `"RESC"`.
    RESC,
    /// Company calendar (requires news-feed entitlement) — wire value `"CalendarReport"`.
    CalendarReport,
}

impl FundamentalReportType {
    /// Return the canonical IBKR wire string for this report type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReportsFinSummary => "ReportsFinSummary",
            Self::ReportSnapshot => "ReportSnapshot",
            Self::RESC => "RESC",
            Self::CalendarReport => "CalendarReport",
        }
    }

    fn from_wire(s: &str) -> Option<Self> {
        match s {
            "ReportsFinSummary" => Some(Self::ReportsFinSummary),
            "ReportSnapshot" => Some(Self::ReportSnapshot),
            "RESC" => Some(Self::RESC),
            "CalendarReport" => Some(Self::CalendarReport),
            _ => None,
        }
    }
}

impl_wire_enum!(FundamentalReportType);

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
