#![allow(dead_code)]
//! Server version constants for TWS API feature compatibility.
//!
//! These constants represent the minimum server version required for specific features.
//! They are used internally to check if a feature is supported by the connected TWS/Gateway
//! before sending requests that depend on that feature.

// shouldn't these all be deprecated?
// pub const HISTORICAL_DATA: i32 = 24;
// pub const CURRENT_TIME: i32 = 33;
/// Minimum server version for real-time bars functionality.
pub const REAL_TIME_BARS: i32 = 34;
/// Minimum server version for scale orders.
pub const SCALE_ORDERS: i32 = 35;
/// Minimum server version for snapshot market data.
pub const SNAPSHOT_MKT_DATA: i32 = 35;
/// Minimum server version for short sale combo legs.
pub const SSHORT_COMBO_LEGS: i32 = 35;
/// Minimum server version for what-if orders.
pub const WHAT_IF_ORDERS: i32 = 36;
/// Minimum server version for contract ID support.
pub const CONTRACT_CONID: i32 = 37;

/// Minimum server version for PTA (Principal Trading Adviser) orders.
pub const PTA_ORDERS: i32 = 39;
/// Minimum server version for fundamental data requests.
pub const FUNDAMENTAL_DATA: i32 = 40;
/// Minimum server version for delta neutral contracts.
pub const DELTA_NEUTRAL: i32 = 40;
/// Minimum server version for contract data chain.
pub const CONTRACT_DATA_CHAIN: i32 = 40;
/// Minimum server version for scale orders v2.
pub const SCALE_ORDERS2: i32 = 40;
/// Minimum server version for algorithmic orders.
pub const ALGO_ORDERS: i32 = 41;
/// Minimum server version for execution data chain.
pub const EXECUTION_DATA_CHAIN: i32 = 42;
/// Minimum server version for not-held orders.
pub const NOT_HELD: i32 = 44;
/// Minimum server version for security ID type.
pub const SEC_ID_TYPE: i32 = 45;
/// Minimum server version for placing orders with contract ID.
pub const PLACE_ORDER_CONID: i32 = 46;
/// Minimum server version for requesting market data with contract ID.
pub const REQ_MKT_DATA_CONID: i32 = 47;
/// Minimum server version for requesting implied volatility calculations.
pub const REQ_CALC_IMPLIED_VOLAT: i32 = 49;
/// Minimum server version for requesting option price calculations.
pub const REQ_CALC_OPTION_PRICE: i32 = 50;
pub const SSHORTX_OLD: i32 = 51;
pub const SSHORTX: i32 = 52;
/// Minimum server version for global cancel requests.
pub const REQ_GLOBAL_CANCEL: i32 = 53;
pub const HEDGE_ORDERS: i32 = 54;
/// Minimum server version for market data type requests.
pub const REQ_MARKET_DATA_TYPE: i32 = 55;
pub const OPT_OUT_SMART_ROUTING: i32 = 56;
pub const SMART_COMBO_ROUTING_PARAMS: i32 = 57;
pub const DELTA_NEUTRAL_CONID: i32 = 58;
pub const SCALE_ORDERS3: i32 = 60;
pub const ORDER_COMBO_LEGS_PRICE: i32 = 61;
pub const TRAILING_PERCENT: i32 = 62;
pub const DELTA_NEUTRAL_OPEN_CLOSE: i32 = 66;
/// Minimum server version for position requests.
pub const POSITIONS: i32 = 67;
/// Minimum server version for account summary requests.
pub const ACCOUNT_SUMMARY: i32 = 67;
/// Minimum server version for trading class support.
pub const TRADING_CLASS: i32 = 68;
pub const SCALE_TABLE: i32 = 69;
/// Minimum server version for order linking.
pub const LINKING: i32 = 70;
pub const ALGO_ID: i32 = 71;
pub const OPTIONAL_CAPABILITIES: i32 = 72;
pub const ORDER_SOLICITED: i32 = 73;
pub const LINKING_AUTH: i32 = 74;
pub const PRIMARYEXCH: i32 = 75;
pub const RANDOMIZE_SIZE_AND_PRICE: i32 = 76;
pub const FRACTIONAL_POSITIONS: i32 = 101;
pub const PEGGED_TO_BENCHMARK: i32 = 102;
pub const MODELS_SUPPORT: i32 = 103;
pub const SEC_DEF_OPT_PARAMS_REQ: i32 = 104;
pub const EXT_OPERATOR: i32 = 105;
pub const SOFT_DOLLAR_TIER: i32 = 106;
pub const REQ_FAMILY_CODES: i32 = 107;
pub const REQ_MATCHING_SYMBOLS: i32 = 108;
pub const PAST_LIMIT: i32 = 109;
pub const MD_SIZE_MULTIPLIER: i32 = 110;
pub const CASH_QTY: i32 = 111;
pub const REQ_MKT_DEPTH_EXCHANGES: i32 = 112;
pub const TICK_NEWS: i32 = 113;
pub const REQ_SMART_COMPONENTS: i32 = 114;
pub const REQ_NEWS_PROVIDERS: i32 = 115;
pub const REQ_NEWS_ARTICLE: i32 = 116;
pub const REQ_HISTORICAL_NEWS: i32 = 117;
pub const REQ_HEAD_TIMESTAMP: i32 = 118;
pub const REQ_HISTOGRAM: i32 = 119;
pub const SERVICE_DATA_TYPE: i32 = 120;
pub const AGG_GROUP: i32 = 121;
pub const UNDERLYING_INFO: i32 = 122;
pub const CANCEL_HEADTIMESTAMP: i32 = 123;
pub const SYNT_REALTIME_BARS: i32 = 124;
pub const CFD_REROUTE: i32 = 125;
pub const MARKET_RULES: i32 = 126;
/// Minimum server version for profit and loss (PnL) requests.
pub const PNL: i32 = 127;
pub const NEWS_QUERY_ORIGINS: i32 = 128;
/// Minimum server version for unrealized PnL data.
pub const UNREALIZED_PNL: i32 = 129;
pub const HISTORICAL_TICKS: i32 = 130;
pub const MARKET_CAP_PRICE: i32 = 131;
pub const PRE_OPEN_BID_ASK: i32 = 132;
pub const REAL_EXPIRATION_DATE: i32 = 134;
/// Minimum server version for realized PnL data.
pub const REALIZED_PNL: i32 = 135;
pub const LAST_LIQUIDITY: i32 = 136;
pub const TICK_BY_TICK: i32 = 137;
pub const DECISION_MAKER: i32 = 138;
pub const MIFID_EXECUTION: i32 = 139;
pub const TICK_BY_TICK_IGNORE_SIZE: i32 = 140;
pub const AUTO_PRICE_FOR_HEDGE: i32 = 141;
pub const WHAT_IF_EXT_FIELDS: i32 = 142;
pub const SCANNER_GENERIC_OPTS: i32 = 143;
pub const API_BIND_ORDER: i32 = 144;
pub const ORDER_CONTAINER: i32 = 145;
pub const SMART_DEPTH: i32 = 146;
pub const REMOVE_NULL_ALL_CASTING: i32 = 147;
pub const D_PEG_ORDERS: i32 = 148;
pub const MKT_DEPTH_PRIM_EXCHANGE: i32 = 149;
pub const COMPLETED_ORDERS: i32 = 150;
pub const PRICE_MGMT_ALGO: i32 = 151;
pub const STOCK_TYPE: i32 = 152;
pub const ENCODE_MSG_ASCII7: i32 = 153;
pub const SEND_ALL_FAMILY_CODES: i32 = 154;
pub const NO_DEFAULT_OPEN_CLOSE: i32 = 155;
pub const PRICE_BASED_VOLATILITY: i32 = 156;
pub const REPLACE_FA_END: i32 = 157;
pub const DURATION: i32 = 158;
pub const MARKET_DATA_IN_SHARES: i32 = 159;
pub const POST_TO_ATS: i32 = 160;
pub const WSHE_CALENDAR: i32 = 161;
pub const AUTO_CANCEL_PARENT: i32 = 162;
pub const FRACTIONAL_SIZE_SUPPORT: i32 = 163;
/// Minimum server version for size rules support.
pub const SIZE_RULES: i32 = 164;
pub const HISTORICAL_SCHEDULE: i32 = 165;
pub const ADVANCED_ORDER_REJECT: i32 = 166;
pub const USER_INFO: i32 = 167;
pub const CRYPTO_AGGREGATED_TRADES: i32 = 168;
pub const MANUAL_ORDER_TIME: i32 = 169;
pub const PEGBEST_PEGMID_OFFSETS: i32 = 170;
pub const WSH_EVENT_DATA_FILTERS: i32 = 171;
pub const IPO_PRICES: i32 = 172;
pub const WSH_EVENT_DATA_FILTERS_DATE: i32 = 173;
pub const INSTRUMENT_TIMEZONE: i32 = 174;
pub const HMDS_MARKET_DATA_IN_SHARES: i32 = 175;
pub const BOND_ISSUERID: i32 = 176;
pub const FA_PROFILE_DESUPPORT: i32 = 177;
