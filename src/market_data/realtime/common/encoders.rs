use crate::contracts::{Contract, SecurityType, TagValue};
use crate::market_data::realtime::{BarSize, WhatToShow};
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::{server_versions, Error};

pub(crate) fn encode_request_realtime_bars(
    server_version: i32,
    ticker_id: i32,
    contract: &Contract,
    _bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_rth: bool,
    options: Vec<TagValue>,
) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 8;
    let mut packet = RequestMessage::default();
    packet.push_field(&OutgoingMessages::RequestRealTimeBars);
    packet.push_field(&VERSION);
    packet.push_field(&ticker_id);
    if server_version >= server_versions::TRADING_CLASS {
        packet.push_field(&contract.contract_id);
    }
    packet.push_field(&contract.symbol);
    packet.push_field(&contract.security_type);
    packet.push_field(&contract.last_trade_date_or_contract_month);
    packet.push_field(&contract.strike);
    packet.push_field(&contract.right);
    packet.push_field(&contract.multiplier);
    packet.push_field(&contract.exchange);
    packet.push_field(&contract.primary_exchange);
    packet.push_field(&contract.currency);
    packet.push_field(&contract.local_symbol);
    if server_version >= server_versions::TRADING_CLASS {
        packet.push_field(&contract.trading_class);
    }
    packet.push_field(&0); // bar size -- not used
    packet.push_field(&what_to_show.to_string());
    packet.push_field(&use_rth);
    if server_version >= server_versions::LINKING {
        packet.push_field(&options);
    }
    Ok(packet)
}
pub(crate) fn encode_cancel_realtime_bars(request_id: i32) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;
    let mut message = RequestMessage::default();
    message.push_field(&OutgoingMessages::CancelRealTimeBars);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    Ok(message)
}
pub(crate) fn encode_tick_by_tick(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    tick_type: &str,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();
    message.push_field(&OutgoingMessages::RequestTickByTickData);
    message.push_field(&request_id);
    message.push_field(&contract.contract_id);
    message.push_field(&contract.symbol);
    message.push_field(&contract.security_type);
    message.push_field(&contract.last_trade_date_or_contract_month);
    message.push_field(&contract.strike);
    message.push_field(&contract.right);
    message.push_field(&contract.multiplier);
    message.push_field(&contract.exchange);
    message.push_field(&contract.primary_exchange);
    message.push_field(&contract.currency);
    message.push_field(&contract.local_symbol);
    message.push_field(&contract.trading_class);
    message.push_field(&tick_type);
    if server_version >= server_versions::TICK_BY_TICK_IGNORE_SIZE {
        message.push_field(&number_of_ticks);
        message.push_field(&ignore_size);
    }
    Ok(message)
}
pub(crate) fn encode_cancel_tick_by_tick(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();
    message.push_field(&OutgoingMessages::CancelTickByTickData);
    message.push_field(&request_id);
    Ok(message)
}
pub(crate) fn encode_request_market_depth(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    number_of_rows: i32,
    is_smart_depth: bool,
) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 5;
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestMarketDepth);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    // Contract fields
    if server_version >= server_versions::TRADING_CLASS {
        message.push_field(&contract.contract_id);
    }
    message.push_field(&contract.symbol);
    message.push_field(&contract.security_type);
    message.push_field(&contract.last_trade_date_or_contract_month);
    message.push_field(&contract.strike);
    message.push_field(&contract.right);
    message.push_field(&contract.multiplier);
    message.push_field(&contract.exchange);
    if server_version >= server_versions::MKT_DEPTH_PRIM_EXCHANGE {
        message.push_field(&contract.primary_exchange);
    }
    message.push_field(&contract.currency);
    message.push_field(&contract.local_symbol);
    if server_version >= server_versions::TRADING_CLASS {
        message.push_field(&contract.trading_class);
    }
    message.push_field(&number_of_rows);
    if server_version >= server_versions::SMART_DEPTH {
        message.push_field(&is_smart_depth);
    }
    if server_version >= server_versions::LINKING {
        message.push_field(&"");
    }
    Ok(message)
}
pub(crate) fn encode_cancel_market_depth(server_version: i32, request_id: i32, is_smart_depth: bool) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    const VERSION: i32 = 1;
    message.push_field(&OutgoingMessages::CancelMarketDepth);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    if server_version >= server_versions::SMART_DEPTH {
        message.push_field(&is_smart_depth);
    }
    Ok(message)
}
pub(crate) fn encode_request_market_depth_exchanges() -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestMktDepthExchanges);
    Ok(message)
}
pub(crate) fn encode_request_market_data(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    generic_ticks: &[&str],
    snapshot: bool,
    regulatory_snapshot: bool,
) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 11;
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestMarketData);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    message.push_field(&contract.contract_id);
    message.push_field(&contract.symbol);
    message.push_field(&contract.security_type);
    message.push_field(&contract.last_trade_date_or_contract_month);
    message.push_field(&contract.strike);
    message.push_field(&contract.right);
    message.push_field(&contract.multiplier);
    message.push_field(&contract.exchange);
    message.push_field(&contract.primary_exchange);
    message.push_field(&contract.currency);
    message.push_field(&contract.local_symbol);
    message.push_field(&contract.trading_class);
    if contract.security_type == SecurityType::Spread {
        message.push_field(&contract.combo_legs.len());
        for leg in &contract.combo_legs {
            message.push_field(&leg.contract_id);
            message.push_field(&leg.ratio);
            message.push_field(&leg.action);
            message.push_field(&leg.exchange);
        }
    }
    if let Some(delta_neutral_contract) = &contract.delta_neutral_contract {
        message.push_field(&true);
        message.push_field(&delta_neutral_contract.contract_id);
        message.push_field(&delta_neutral_contract.delta);
        message.push_field(&delta_neutral_contract.price);
    } else {
        message.push_field(&false);
    }
    message.push_field(&generic_ticks.join(","));
    message.push_field(&snapshot);
    if server_version >= server_versions::REQ_SMART_COMPONENTS {
        message.push_field(&regulatory_snapshot);
    }
    message.push_field(&"");
    Ok(message)
}
pub(crate) fn encode_cancel_market_data(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    const VERSION: i32 = 1;
    message.push_field(&OutgoingMessages::CancelMarketData);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    Ok(message)
}

#[cfg(test)]
pub(crate) mod test_constants {
    // Market data request message field indexes for non-combo stock contracts
    pub const MARKET_DATA_MSG_TYPE_IDX: usize = 0;
    #[allow(dead_code)]
    pub const MARKET_DATA_VERSION_IDX: usize = 1;
    #[allow(dead_code)]
    pub const MARKET_DATA_REQUEST_ID_IDX: usize = 2;
    #[allow(dead_code)]
    pub const MARKET_DATA_CONTRACT_ID_IDX: usize = 3;
    #[allow(dead_code)]
    pub const MARKET_DATA_SYMBOL_IDX: usize = 4;
    #[allow(dead_code)]
    pub const MARKET_DATA_SECURITY_TYPE_IDX: usize = 5;
    #[allow(dead_code)]
    pub const MARKET_DATA_EXPIRY_IDX: usize = 6;
    #[allow(dead_code)]
    pub const MARKET_DATA_STRIKE_IDX: usize = 7;
    #[allow(dead_code)]
    pub const MARKET_DATA_RIGHT_IDX: usize = 8;
    #[allow(dead_code)]
    pub const MARKET_DATA_MULTIPLIER_IDX: usize = 9;
    #[allow(dead_code)]
    pub const MARKET_DATA_EXCHANGE_IDX: usize = 10;
    #[allow(dead_code)]
    pub const MARKET_DATA_PRIMARY_EXCHANGE_IDX: usize = 11;
    #[allow(dead_code)]
    pub const MARKET_DATA_CURRENCY_IDX: usize = 12;
    #[allow(dead_code)]
    pub const MARKET_DATA_LOCAL_SYMBOL_IDX: usize = 13;
    #[allow(dead_code)]
    pub const MARKET_DATA_TRADING_CLASS_IDX: usize = 14;
    #[allow(dead_code)]
    pub const MARKET_DATA_HAS_DELTA_NEUTRAL_IDX: usize = 15; // false for stocks
    pub const MARKET_DATA_GENERIC_TICKS_IDX: usize = 16;
    pub const MARKET_DATA_SNAPSHOT_IDX: usize = 17;
    pub const MARKET_DATA_REGULATORY_SNAPSHOT_IDX: usize = 18; // Only for server >= REQ_SMART_COMPONENTS
    #[allow(dead_code)]
    pub const MARKET_DATA_OPTIONS_IDX: usize = 19; // Empty string
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{contracts::Contract, contracts::SecurityType, contracts::TagValue, messages::OutgoingMessages, ToField};

    /// Helper function to create a basic test contract
    fn create_test_contract() -> Contract {
        Contract::stock("AAPL").build()
    }

    #[cfg(test)]
    mod tick_by_tick_tests {
        use super::*;
        use crate::contracts::{Currency, Exchange, Symbol};

        #[test]
        fn test_encode_cancel_tick_by_tick() {
            let request_id = 9000;
            let message = encode_cancel_tick_by_tick(request_id).expect("Failed to encode cancel tick by tick");

            assert_eq!(message[0], OutgoingMessages::CancelTickByTickData.to_field(), "Wrong message type");
            assert_eq!(message[1], request_id.to_string(), "Wrong request ID");
            assert_eq!(message.len(), 2, "Unexpected message length");
        }

        #[test]
        fn test_encode_tick_by_tick() {
            let request_id = 9000;
            let server_version = server_versions::TICK_BY_TICK;
            let contract = Contract {
                symbol: Symbol::from("GBL"),
                security_type: SecurityType::Future,
                exchange: Exchange::from("EUREX"),
                currency: Currency::from("EUR"),
                last_trade_date_or_contract_month: "202303".to_owned(),
                ..Contract::default()
            };
            let tick_type = "AllLast";
            let number_of_ticks = 1;
            let ignore_size = true;

            let message = encode_tick_by_tick(server_version, request_id, &contract, tick_type, number_of_ticks, ignore_size)
                .expect("Failed to encode tick by tick");

            // Verify message structure
            assert_eq!(message[0], OutgoingMessages::RequestTickByTickData.to_field(), "Wrong message type");
            assert_eq!(message[1], request_id.to_field(), "Wrong request ID");

            // Verify contract fields
            assert_eq!(message[2], contract.contract_id.to_field(), "Wrong contract ID");
            assert_eq!(message[3], contract.symbol.to_field(), "Wrong symbol");
            assert_eq!(message[4], contract.security_type.to_field(), "Wrong security type");
            assert_eq!(message[5], contract.last_trade_date_or_contract_month, "Wrong trade date");
            assert_eq!(message[6], contract.strike.to_field(), "Wrong strike price");
            assert_eq!(message[7], contract.right, "Wrong right");
            assert_eq!(message[8], contract.multiplier, "Wrong multiplier");
            assert_eq!(message[9], contract.exchange.to_field(), "Wrong exchange");
            assert_eq!(message[10], contract.primary_exchange.to_field(), "Wrong primary exchange");
            assert_eq!(message[11], contract.currency.to_field(), "Wrong currency");
            assert_eq!(message[12], contract.local_symbol, "Wrong local symbol");
            assert_eq!(message[13], contract.trading_class, "Wrong trading class");

            // Verify tick parameters
            assert_eq!(message[14], tick_type, "Wrong tick type");

            // Version specific fields
            if server_version >= server_versions::TICK_BY_TICK_IGNORE_SIZE {
                assert_eq!(message[15], number_of_ticks.to_string(), "Wrong number of ticks");
                assert_eq!(message[16], ignore_size.to_string(), "Wrong ignore size flag");
            }
        }

        #[test]
        fn test_tick_by_tick_with_old_server() {
            let request_id = 9000;
            let server_version = server_versions::TICK_BY_TICK - 1; // Version before TICK_BY_TICK
            let contract = Contract {
                symbol: Symbol::from("GBL"),
                security_type: SecurityType::Future,
                exchange: Exchange::from("EUREX"),
                currency: Currency::from("EUR"),
                last_trade_date_or_contract_month: "202303".to_owned(),
                ..Contract::default()
            };
            let tick_type = "AllLast";
            let number_of_ticks = 1;
            let ignore_size = true;

            let message = encode_tick_by_tick(server_version, request_id, &contract, tick_type, number_of_ticks, ignore_size)
                .expect("Failed to encode tick by tick");

            // Verify no version specific fields are included
            assert_eq!(message.len(), 15, "Unexpected message length for old server version");
        }
    }

    #[cfg(test)]
    mod realtime_bars_tests {
        use super::*;
        use crate::contracts::{Currency, Exchange, Symbol};

        #[test]
        fn test_encode_cancel_realtime_bars() {
            let request_id = 9000;
            let message = encode_cancel_realtime_bars(request_id).expect("Failed to encode cancel realtime bars");

            assert_eq!(message[0], OutgoingMessages::CancelRealTimeBars.to_field(), "Wrong message type");
            assert_eq!(message[1], "1", "Wrong version");
            assert_eq!(message[2], request_id.to_string(), "Wrong request ID");
            assert_eq!(message.len(), 3, "Unexpected message length");
        }

        #[test]
        fn test_encode_request_realtime_bars() {
            let request_id = 9000;
            let server_version = server_versions::TICK_BY_TICK;
            let contract = Contract {
                symbol: Symbol::from("GBL"),
                security_type: SecurityType::Future,
                exchange: Exchange::from("EUREX"),
                currency: Currency::from("EUR"),
                last_trade_date_or_contract_month: "202303".to_owned(),
                ..Contract::default()
            };
            let bar_size = BarSize::Sec5;
            let what_to_show = WhatToShow::Trades;
            let use_rth = true;
            let options: Vec<TagValue> = vec![];

            let message = encode_request_realtime_bars(server_version, request_id, &contract, &bar_size, &what_to_show, use_rth, options)
                .expect("Failed to encode realtime bars request");

            // Verify message structure
            assert_eq!(message[0], OutgoingMessages::RequestRealTimeBars.to_field(), "Wrong message type");
            assert_eq!(message[1], "8", "Wrong version");
            assert_eq!(message[2], request_id.to_field(), "Wrong request ID");

            // Verify contract fields
            assert_eq!(message[3], contract.contract_id.to_field(), "Wrong contract ID");
            assert_eq!(message[4], contract.symbol.to_field(), "Wrong symbol");
            assert_eq!(message[5], contract.security_type.to_field(), "Wrong security type");
            assert_eq!(message[6], contract.last_trade_date_or_contract_month, "Wrong trade date");
            assert_eq!(message[7], contract.strike.to_field(), "Wrong strike price");
            assert_eq!(message[8], contract.right, "Wrong right");
            assert_eq!(message[9], contract.multiplier, "Wrong multiplier");
            assert_eq!(message[10], contract.exchange.to_field(), "Wrong exchange");
            assert_eq!(message[11], contract.primary_exchange.to_field(), "Wrong primary exchange");
            assert_eq!(message[12], contract.currency.to_field(), "Wrong currency");
            assert_eq!(message[13], contract.local_symbol, "Wrong local symbol");
            assert_eq!(message[14], contract.trading_class, "Wrong trading class");

            // Verify bar parameters
            assert_eq!(message[15], "0", "Wrong bar size");
            assert_eq!(message[16], what_to_show.to_field(), "Wrong what to show value");
            assert_eq!(message[17], use_rth.to_field(), "Wrong use RTH flag");
            assert_eq!(message[18], "", "Wrong options field");
        }

        #[test]
        fn test_encode_request_realtime_bars_with_options() {
            let request_id = 9000;
            let server_version = server_versions::TICK_BY_TICK;
            let contract = Contract {
                symbol: Symbol::from("GBL"),
                security_type: SecurityType::Future,
                exchange: Exchange::from("EUREX"),
                currency: Currency::from("EUR"),
                last_trade_date_or_contract_month: "202303".to_owned(),
                ..Contract::default()
            };
            let bar_size = BarSize::Sec5;
            let what_to_show = WhatToShow::Trades;
            let use_rth = true;
            let options = vec![TagValue {
                tag: "aggregateGroup".to_string(),
                value: "1".to_string(),
            }];

            let message = encode_request_realtime_bars(server_version, request_id, &contract, &bar_size, &what_to_show, use_rth, options)
                .expect("Failed to encode realtime bars request");

            assert_eq!(message[18], "aggregateGroup=1;", "Wrong options encoding");
        }
    }

    #[cfg(test)]
    mod market_data_tests {
        use super::*;

        #[test]
        fn test_encode_request_market_data() {
            let server_version = server_versions::SIZE_RULES;
            let request_id = 9000;
            let contract = create_test_contract();
            let generic_ticks = &["100", "101", "104"];
            let snapshot = false;
            let regulatory_snapshot = false;

            let message = encode_request_market_data(server_version, request_id, &contract, generic_ticks, snapshot, regulatory_snapshot)
                .expect("Failed to encode market data request");

            // Verify basic message structure
            assert_eq!(message[0], OutgoingMessages::RequestMarketData.to_field(), "Wrong message type");
            assert_eq!(message[1], "11", "Wrong version");
            assert_eq!(message[2], request_id.to_field(), "Wrong request ID");

            // Verify contract fields
            assert_eq!(message[3], contract.contract_id.to_field(), "Wrong contract ID");

            // Verify generic ticks
            assert_eq!(message[16], "100,101,104", "Wrong generic ticks");

            // Verify snapshot flags
            assert_eq!(message[17], snapshot.to_field(), "Wrong snapshot flag");
            assert_eq!(message[18], regulatory_snapshot.to_field(), "Wrong regulatory snapshot flag");
        }

        #[test]
        fn test_encode_cancel_market_data() {
            let request_id = 9000;
            let message = encode_cancel_market_data(request_id).expect("Failed to encode cancel market data");

            assert_eq!(message[0], OutgoingMessages::CancelMarketData.to_field(), "Wrong message type");
            assert_eq!(message[1], "1", "Wrong version");
            assert_eq!(message[2], request_id.to_string(), "Wrong request ID");
            assert_eq!(message.len(), 3, "Unexpected message length");
        }
    }

    #[cfg(test)]
    mod market_depth_tests {
        use super::*;

        #[test]
        fn test_encode_request_market_depth_basic() {
            const VERSION: i32 = 5;

            let server_version = server_versions::SIZE_RULES;
            let request_id = 9000;
            let contract = create_test_contract();
            let number_of_rows = 5;
            let is_smart_depth = false;

            let message = encode_request_market_depth(server_version, request_id, &contract, number_of_rows, is_smart_depth)
                .expect("Failed to encode market depth request");

            // Verify basic message structure
            assert_eq!(message[0], OutgoingMessages::RequestMarketDepth.to_field());
            assert_eq!(message[1], VERSION.to_field(), "Unexpected version");
            assert_eq!(message[2], request_id.to_field(), "Unexpected request ID");

            // Contract fields
            assert_eq!(message[3], contract.contract_id.to_field(), "Unexpected contract ID");

            // Verify any additional fields
            assert_eq!(message[15], number_of_rows.to_field(), "Unexpected number of rows");
            assert_eq!(message[16], is_smart_depth.to_field(), "Unexpected smart depth flag");
        }

        #[test]
        fn test_encode_cancel_market_depth() {
            let request_id = 9000;
            let server_version = server_versions::SMART_DEPTH;
            let is_smart_depth = true;

            let message = encode_cancel_market_depth(server_version, request_id, is_smart_depth).expect("Failed to encode cancel realtime bars");

            assert_eq!(message[0], OutgoingMessages::CancelMarketDepth.to_field(), "Wrong message type");
            assert_eq!(message[1], "1", "Wrong version");
            assert_eq!(message[2], request_id.to_field(), "Wrong request ID");
            assert_eq!(message[3], is_smart_depth.to_field(), "Wrong smart depth flag");
            assert_eq!(message.len(), 4, "Unexpected message length");
        }
    }

    #[cfg(test)]
    mod market_depth_exchanges_tests {
        use super::*;

        #[test]
        fn test_encode_request_market_depth_exchanges() {
            let message = encode_request_market_depth_exchanges().expect("Failed to encode market depth exchanges request");

            assert_eq!(message[0], OutgoingMessages::RequestMktDepthExchanges.to_field());
            assert_eq!(message.len(), 1, "Unexpected message length");
        }
    }
}
