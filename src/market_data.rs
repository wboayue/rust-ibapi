use time::OffsetDateTime;

use crate::ToField;

pub mod historical;
pub mod realtime;

#[derive(Clone, Debug, Copy)]
pub enum WhatToShow {
    Trades,
    MidPoint,
    Bid,
    Ask,
}

impl ToString for WhatToShow {
    fn to_string(&self) -> String {
        match self {
            Self::Trades => "TRADES".to_string(),
            Self::MidPoint => "MIDPOINT".to_string(),
            Self::Bid => "BID".to_string(),
            Self::Ask => "ASK".to_string(),
        }
    }
}

impl ToField for WhatToShow {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

// TRADES
// MIDPOINT
// BID
// ASK
// BID_ASK
// HISTORICAL_VOLATILITY
// OPTION_IMPLIED_VOLATILITY
// FEE_RATE
// SCHEDULE

/// Bar describes the historical data bar.
pub struct Bar {
    /// The bar's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
    pub time: OffsetDateTime,
    /// The bar's open price.
    pub open: f64,
    /// The bar's high price.
    pub high: f64,
    /// The bar's low price.
    pub low: f64,
    /// The bar's close price.
    pub close: f64,
    /// The bar's traded volume if available (only available for TRADES)
    pub volume: i64,
    /// The bar's Weighted Average Price (only available for TRADES)
    pub wap: f64,
    /// The number of trades during the bar's timespan (only available for TRADES)
    pub count: i32,
}

#[derive(Debug)]
pub struct HistogramData {
    pub price: f64,
    pub count: i32,
}
