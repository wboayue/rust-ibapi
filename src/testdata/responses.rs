//pub const POSITION: &str = "61\03\0DU1234567\076792991\0TSLA\0STK\0\00.0\0\0\0NASDAQ\0USD\0TSLA\0NMS\0500\0196.77\0";

// accounts

pub const MANAGED_ACCOUNT: &str = "15|1|DU1234567,DU7654321|";

pub const ACCOUNT_UPDATE_MULTI_CASH_BALANCE: &str = "73|1|9000|DU1234567||CashBalance|94629.71|USD||";
pub const ACCOUNT_UPDATE_MULTI_CURRENCY: &str = "73|1|9000|DU1234567||Currency|USD|USD||";
pub const ACCOUNT_UPDATE_MULTI_STOCK_MARKET_VALUE: &str = "73|1|9000|DU1234567||StockMarketValue|0.00|BASE||";
pub const ACCOUNT_UPDATE_MULTI_END: &str = "74|1|9000||";

// contracts

pub const MARKET_RULE: &str = "93|26|1|0|0.01|";

// Market Depth

pub const MARKET_DEPTH_1: &str = "13|1|9000|0|OVERNIGHT|0|1|235.84|300|1||";
pub const MARKET_DEPTH_2: &str = "13|1|9000|0|OVERNIGHT|0|0|236.09|200|1||";
pub const MARKET_DEPTH_3: &str = "4|2|9000|2152|Exchanges - Depth: IEX; Top: BYX; AMEX; PEARL; MEMX; EDGEA; OVERNIGHT; CHX; NYSENAT; IBEOS; PSX; LTSE; ISE; DRCTEDGE; Need additional market data permissions - Depth: BATS; ARCA; ISLAND; BEX; NYSE; ||";
pub const MARKET_DEPTH_4: &str = "13|1|9000|1|OVERNIGHT|0|1|235.84|300|1||";
pub const MARKET_DEPTH_5: &str = "13|1|9000|0|IBEOS|1|1|235.84|100|1||";
pub const MARKET_DEPTH_6: &str = "13|1|9000|1|IBEOS|0|0|236.26|100|1||";
pub const MARKET_DEPTH_7: &str = "13|1|9000|1|OVERNIGHT|1|1|235.84|200|1||";
pub const MARKET_DEPTH_8: &str = "13|1|9000|0|OVERNIGHT|1|1|235.84|200|1||";
pub const MARKET_DEPTH_9: &str = "13|1|9000|1|IBEOS|1|1|235.82|100|1||";
