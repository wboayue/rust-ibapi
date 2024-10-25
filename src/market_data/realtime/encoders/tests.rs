use super::*;
use crate::{contracts::contract_samples, contracts::Contract, messages::OutgoingMessages, orders::TagValue, ToField};

/// Helper function to create a basic test contract
fn create_test_contract() -> Contract {
    Contract::stock("AAPL")
}

#[cfg(test)]
mod tick_by_tick_tests {
    use super::*;

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
        let contract = contract_samples::simple_future();
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
        assert_eq!(message[3], contract.symbol, "Wrong symbol");
        assert_eq!(message[4], contract.security_type.to_field(), "Wrong security type");
        assert_eq!(message[5], contract.last_trade_date_or_contract_month, "Wrong trade date");
        assert_eq!(message[6], contract.strike.to_field(), "Wrong strike price");
        assert_eq!(message[7], contract.right, "Wrong right");
        assert_eq!(message[8], contract.multiplier, "Wrong multiplier");
        assert_eq!(message[9], contract.exchange, "Wrong exchange");
        assert_eq!(message[10], contract.primary_exchange, "Wrong primary exchange");
        assert_eq!(message[11], contract.currency, "Wrong currency");
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
        let contract = contract_samples::simple_future();
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
        let contract = contract_samples::simple_future();
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
        assert_eq!(message[4], contract.symbol, "Wrong symbol");
        assert_eq!(message[5], contract.security_type.to_field(), "Wrong security type");
        assert_eq!(message[6], contract.last_trade_date_or_contract_month, "Wrong trade date");
        assert_eq!(message[7], contract.strike.to_field(), "Wrong strike price");
        assert_eq!(message[8], contract.right, "Wrong right");
        assert_eq!(message[9], contract.multiplier, "Wrong multiplier");
        assert_eq!(message[10], contract.exchange, "Wrong exchange");
        assert_eq!(message[11], contract.primary_exchange, "Wrong primary exchange");
        assert_eq!(message[12], contract.currency, "Wrong currency");
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
        let contract = contract_samples::simple_future();
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
