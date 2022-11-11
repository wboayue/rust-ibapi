use anyhow::{anyhow, Result};

use crate::client::Client;
use crate::domain::BidAsk;
use crate::domain::Contract;
use crate::domain::RealTimeBar;
use crate::domain::TagValue;

pub enum WhatToShow {
    Trades,
    MidPoint,
    Bid,
    Ask,
}

pub struct BarIterator {}

impl BarIterator {
    pub fn new() -> BarIterator {
        BarIterator {}
    }
}

impl Iterator for BarIterator {
    // we will be counting with usize
    type Item = RealTimeBar;

    // next() is the only required method
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

/// Requests real time bars
/// Currently, only 5 seconds bars are provided. This request is subject to the same pacing as any historical data request: no more than 60 API queries in more than 600 seconds.
/// Real time bars subscriptions are also included in the calculation of the number of Level 1 market data subscriptions allowed in an account.
///
/// Parameters
/// tickerId	the request's unique identifier.
/// contract	the Contract for which the depth is being requested
/// barSize	currently being ignored
/// whatToShow	the nature of the data being retrieved:
/// TRADES
/// MIDPOINT
/// BID
/// ASK
/// useRTH	set to 0 to obtain the data which was also generated ourside of the Regular Trading Hours, set to 1 to obtain only the RTH data
pub fn real_time_bars(
    client: &Client,
    contract: &Contract,
    what_to_show: &WhatToShow,
    use_rth: bool,
) -> Result<BarIterator> {
    Err(anyhow!("not implemented!"))
}

pub fn tick_by_tick_trades(client: &Client, contract: &Contract) -> Result<BidAsk> {
    Err(anyhow!("not implemented!"))
}

pub fn tick_by_tick_bid_ask(client: &Client, contract: &Contract) -> Result<BidAsk> {
    Err(anyhow!("not implemented!"))
}

/// Requests real time market data. Returns market data for an instrument either in real time or 10-15 minutes delayed (depending on the market data type specified)
/// Parameters
/// tickerId	the request's identifier
/// contract	the Contract for which the data is being requested
/// genericTickList	comma separated ids of the available generic ticks:
///     100 Option Volume (currently for stocks)
///     101 Option Open Interest (currently for stocks)
///     104 Historical Volatility (currently for stocks)
///     105 Average Option Volume (currently for stocks)
///     106 Option Implied Volatility (currently for stocks)
///     162 Index Future Premium
///     165 Miscellaneous Stats
///     221 Mark Price (used in TWS P&L computations)
///     225 Auction values (volume, price and imbalance)
///     233 RTVolume - contains the last trade price, last trade size, last trade time, total volume, VWAP, and single trade flag.
///     236 Shortable
///     256 Inventory
///     258 Fundamental Ratios
///     411 Realtime Historical Volatility
///     456 IBDividends
/// snapshot	for users with corresponding real time market data subscriptions. A true value will return a one-time snapshot, while a false value will provide streaming data.
/// regulatory	snapshot for US stocks requests NBBO snapshots for users which have "US Securities Snapshot Bundle" subscription but not corresponding Network A, B, or C subscription necessary for streaming * market data. One-time snapshot of current market price that will incur a fee of 1 cent to the account per snapshot.
pub fn market_data(
    client: &Client,
    contract: &Contract,
    generic_tick_list: &str,
    snapshot: bool,
    regulatory_snapshot: bool,
    market_data_options: &[TagValue],
) {
}
