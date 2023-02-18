pub mod historical;

pub mod streaming;

pub enum BarSize {
    Secs5
}

pub enum WhatToShow {
    Trades,
    MidPoint,
    Bid,
    Ask
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
