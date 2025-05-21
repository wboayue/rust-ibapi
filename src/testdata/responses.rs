// accounts

pub const MANAGED_ACCOUNT: &str = "15|1|DU1234567,DU7654321|";

pub const ACCOUNT_UPDATE_MULTI_CASH_BALANCE: &str = "73|1|9000|DU1234567||CashBalance|94629.71|USD||";
pub const ACCOUNT_UPDATE_MULTI_CURRENCY: &str = "73|1|9000|DU1234567||Currency|USD|USD||";
pub const ACCOUNT_UPDATE_MULTI_STOCK_MARKET_VALUE: &str = "73|1|9000|DU1234567||StockMarketValue|0.00|BASE||";
pub const ACCOUNT_UPDATE_MULTI_END: &str = "74|1|9000||";

pub const ACCOUNT_SUMMARY: &str = "63|1|9000|DU1234567|AccountType|FA||";
pub const ACCOUNT_SUMMARY_END: &str = "64|1|9000||";

pub const ACCOUNT_VALUE: &str = "6|1|CashBalance|1000.00|USD";
pub const PORTFOLIO_VALUE: &str = "9|1|SYM|STK|251212|0.0|P|USD|100.0|10.0|1000.0|";

// contracts

pub const MARKET_RULE: &str = "93|26|1|0|0.01|";

// positions

pub const POSITION: &str = "61|3|DU1234567|76792991|TSLA|STK||0.0|||NASDAQ|USD|TSLA|NMS|500|196.77|";
pub const POSITION_END: &str = "62|1|DU1234567|";

pub const POSITION_MULTI: &str = "71|3|6|DU1234567|76792991|TSLA|STK||0.0|||NASDAQ|USD|TSLA|NMS|500|196.77||";
pub const POSITION_MULTI_END: &str = "72|1|";
