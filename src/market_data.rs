pub mod historical;

pub mod streaming;

#[derive(Clone, Debug)]
pub struct RealTimeBar {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub wap: f64,
    pub count: i32,
}

pub enum BarSize {
    Secs5,
}

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
