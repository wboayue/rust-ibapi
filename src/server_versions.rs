#![allow(dead_code)]
//! Server version constants for TWS API feature compatibility.
//!
//! These constants represent the minimum server version required for specific features.
//! They are used internally to check if a feature is supported by the connected TWS/Gateway
//! before sending requests that depend on that feature.

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
/// Minimum server version for short sale slot value (old version).
pub const SSHORTX_OLD: i32 = 51;
/// Minimum server version for short sale slot value.
pub const SSHORTX: i32 = 52;
/// Minimum server version for global cancel requests.
pub const REQ_GLOBAL_CANCEL: i32 = 53;
/// Minimum server version for hedge orders.
pub const HEDGE_ORDERS: i32 = 54;
/// Minimum server version for market data type requests.
pub const REQ_MARKET_DATA_TYPE: i32 = 55;
/// Minimum server version for opting out of SMART routing.
pub const OPT_OUT_SMART_ROUTING: i32 = 56;
/// Minimum server version for SMART combo routing parameters.
pub const SMART_COMBO_ROUTING_PARAMS: i32 = 57;
/// Minimum server version for delta neutral contract ID.
pub const DELTA_NEUTRAL_CONID: i32 = 58;
/// Minimum server version for scale orders v3.
pub const SCALE_ORDERS3: i32 = 60;
/// Minimum server version for order combo legs price.
pub const ORDER_COMBO_LEGS_PRICE: i32 = 61;
/// Minimum server version for trailing percent orders.
pub const TRAILING_PERCENT: i32 = 62;
/// Minimum server version for delta neutral open/close.
pub const DELTA_NEUTRAL_OPEN_CLOSE: i32 = 66;
/// Minimum server version for position requests.
pub const POSITIONS: i32 = 67;
/// Minimum server version for account summary requests.
pub const ACCOUNT_SUMMARY: i32 = 67;
/// Minimum server version for trading class support.
pub const TRADING_CLASS: i32 = 68;
/// Minimum server version for scale table.
pub const SCALE_TABLE: i32 = 69;
/// Minimum server version for order linking.
pub const LINKING: i32 = 70;
/// Minimum server version for algorithm ID.
pub const ALGO_ID: i32 = 71;
/// Minimum server version for optional capabilities.
pub const OPTIONAL_CAPABILITIES: i32 = 72;
/// Minimum server version for order solicited flag.
pub const ORDER_SOLICITED: i32 = 73;
/// Minimum server version for linking authentication.
pub const LINKING_AUTH: i32 = 74;
/// Minimum server version for primary exchange.
pub const PRIMARYEXCH: i32 = 75;
/// Minimum server version for randomizing size and price.
pub const RANDOMIZE_SIZE_AND_PRICE: i32 = 76;
/// Minimum server version for fractional positions.
pub const FRACTIONAL_POSITIONS: i32 = 101;
/// Minimum server version for pegged to benchmark orders.
pub const PEGGED_TO_BENCHMARK: i32 = 102;
/// Minimum server version for models support.
pub const MODELS_SUPPORT: i32 = 103;
/// Minimum server version for security definition option parameters request.
pub const SEC_DEF_OPT_PARAMS_REQ: i32 = 104;
/// Minimum server version for extended operator.
pub const EXT_OPERATOR: i32 = 105;
/// Minimum server version for soft dollar tier.
pub const SOFT_DOLLAR_TIER: i32 = 106;
/// Minimum server version for requesting family codes.
pub const REQ_FAMILY_CODES: i32 = 107;
/// Minimum server version for requesting matching symbols.
pub const REQ_MATCHING_SYMBOLS: i32 = 108;
/// Minimum server version for past limit orders.
pub const PAST_LIMIT: i32 = 109;
/// Minimum server version for market data size multiplier.
pub const MD_SIZE_MULTIPLIER: i32 = 110;
/// Minimum server version for cash quantity orders.
pub const CASH_QTY: i32 = 111;
pub const REQ_MKT_DEPTH_EXCHANGES: i32 = 112;
/// Minimum server version for tick news.
pub const TICK_NEWS: i32 = 113;
/// Minimum server version for requesting SMART components.
pub const REQ_SMART_COMPONENTS: i32 = 114;
/// Minimum server version for requesting news providers.
pub const REQ_NEWS_PROVIDERS: i32 = 115;
/// Minimum server version for requesting news articles.
pub const REQ_NEWS_ARTICLE: i32 = 116;
/// Minimum server version for requesting historical news.
pub const REQ_HISTORICAL_NEWS: i32 = 117;
/// Minimum server version for requesting head timestamp.
pub const REQ_HEAD_TIMESTAMP: i32 = 118;
/// Minimum server version for requesting histogram data.
pub const REQ_HISTOGRAM: i32 = 119;
/// Minimum server version for service data type.
pub const SERVICE_DATA_TYPE: i32 = 120;
/// Minimum server version for aggregate group.
pub const AGG_GROUP: i32 = 121;
/// Minimum server version for underlying info.
pub const UNDERLYING_INFO: i32 = 122;
/// Minimum server version for canceling head timestamp.
pub const CANCEL_HEADTIMESTAMP: i32 = 123;
/// Minimum server version for synthetic real-time bars.
pub const SYNT_REALTIME_BARS: i32 = 124;
/// Minimum server version for CFD reroute.
pub const CFD_REROUTE: i32 = 125;
/// Minimum server version for market rules.
pub const MARKET_RULES: i32 = 126;
/// Minimum server version for profit and loss (PnL) requests.
pub const PNL: i32 = 127;
/// Minimum server version for news query origins.
pub const NEWS_QUERY_ORIGINS: i32 = 128;
/// Minimum server version for unrealized PnL data.
pub const UNREALIZED_PNL: i32 = 129;
/// Minimum server version for historical ticks.
pub const HISTORICAL_TICKS: i32 = 130;
/// Minimum server version for market cap price.
pub const MARKET_CAP_PRICE: i32 = 131;
/// Minimum server version for pre-open bid/ask.
pub const PRE_OPEN_BID_ASK: i32 = 132;
/// Minimum server version for real expiration date.
pub const REAL_EXPIRATION_DATE: i32 = 134;
/// Minimum server version for realized PnL data.
pub const REALIZED_PNL: i32 = 135;
/// Minimum server version for last liquidity.
pub const LAST_LIQUIDITY: i32 = 136;
/// Minimum server version for tick-by-tick data.
pub const TICK_BY_TICK: i32 = 137;
/// Minimum server version for decision maker.
pub const DECISION_MAKER: i32 = 138;
/// Minimum server version for MiFID execution.
pub const MIFID_EXECUTION: i32 = 139;
/// Minimum server version for tick-by-tick ignore size parameter.
pub const TICK_BY_TICK_IGNORE_SIZE: i32 = 140;
/// Minimum server version for auto price for hedge.
pub const AUTO_PRICE_FOR_HEDGE: i32 = 141;
/// Minimum server version for what-if extended fields.
pub const WHAT_IF_EXT_FIELDS: i32 = 142;
/// Minimum server version for scanner generic options.
pub const SCANNER_GENERIC_OPTS: i32 = 143;
/// Minimum server version for API bind order.
pub const API_BIND_ORDER: i32 = 144;
/// Minimum server version for order container.
pub const ORDER_CONTAINER: i32 = 145;
/// Minimum server version for SMART depth.
pub const SMART_DEPTH: i32 = 146;
/// Minimum server version for removing null all casting.
pub const REMOVE_NULL_ALL_CASTING: i32 = 147;
/// Minimum server version for D-peg orders.
pub const D_PEG_ORDERS: i32 = 148;
/// Minimum server version for market depth primary exchange.
pub const MKT_DEPTH_PRIM_EXCHANGE: i32 = 149;
/// Minimum server version for completed orders.
pub const COMPLETED_ORDERS: i32 = 150;
/// Minimum server version for price management algorithm.
pub const PRICE_MGMT_ALGO: i32 = 151;
/// Minimum server version for stock type.
pub const STOCK_TYPE: i32 = 152;
/// Minimum server version for encoding messages in ASCII7.
pub const ENCODE_MSG_ASCII7: i32 = 153;
/// Minimum server version for sending all family codes.
pub const SEND_ALL_FAMILY_CODES: i32 = 154;
/// Minimum server version for no default open/close.
pub const NO_DEFAULT_OPEN_CLOSE: i32 = 155;
/// Minimum server version for price-based volatility.
pub const PRICE_BASED_VOLATILITY: i32 = 156;
/// Minimum server version for replace FA end.
pub const REPLACE_FA_END: i32 = 157;
/// Minimum server version for duration.
pub const DURATION: i32 = 158;
/// Minimum server version for market data in shares.
pub const MARKET_DATA_IN_SHARES: i32 = 159;
/// Minimum server version for post to ATS.
pub const POST_TO_ATS: i32 = 160;
/// Minimum server version for WSHE calendar.
pub const WSHE_CALENDAR: i32 = 161;
/// Minimum server version for auto cancel parent.
pub const AUTO_CANCEL_PARENT: i32 = 162;
/// Minimum server version for fractional size support.
pub const FRACTIONAL_SIZE_SUPPORT: i32 = 163;
/// Minimum server version for size rules support.
pub const SIZE_RULES: i32 = 164;
/// Minimum server version for historical schedule.
pub const HISTORICAL_SCHEDULE: i32 = 165;
/// Minimum server version for advanced order reject.
pub const ADVANCED_ORDER_REJECT: i32 = 166;
/// Minimum server version for user info.
pub const USER_INFO: i32 = 167;
/// Minimum server version for crypto aggregated trades.
pub const CRYPTO_AGGREGATED_TRADES: i32 = 168;
/// Minimum server version for manual order time.
pub const MANUAL_ORDER_TIME: i32 = 169;
/// Minimum server version for PEG BEST/PEG MID offsets.
pub const PEGBEST_PEGMID_OFFSETS: i32 = 170;
/// Minimum server version for WSH event data filters.
pub const WSH_EVENT_DATA_FILTERS: i32 = 171;
/// Minimum server version for IPO prices.
pub const IPO_PRICES: i32 = 172;
/// Minimum server version for WSH event data filters with date.
pub const WSH_EVENT_DATA_FILTERS_DATE: i32 = 173;
/// Minimum server version for instrument timezone.
pub const INSTRUMENT_TIMEZONE: i32 = 174;
/// Minimum server version for HMDS market data in shares.
pub const HMDS_MARKET_DATA_IN_SHARES: i32 = 175;
/// Minimum server version for bond issuer ID.
pub const BOND_ISSUERID: i32 = 176;
/// Minimum server version for FA profile desupport.
pub const FA_PROFILE_DESUPPORT: i32 = 177;
