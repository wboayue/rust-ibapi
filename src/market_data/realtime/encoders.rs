use anyhow::Result;

use super::{BarSize, WhatToShow};
use crate::client::RequestMessage;
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::orders::TagValue;
use crate::server_versions;

pub fn encode_request_realtime_bars(
    server_version: i32,
    ticker_id: i32,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_rth: bool,
    options: Vec<TagValue>,
) -> Result<RequestMessage> {
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

pub fn cancel_realtime_bars(request_id: i32) -> Result<RequestMessage> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::CancelRealTimeBars);
    message.push_field(&VERSION);
    message.push_field(&request_id);

    Ok(message)
}

pub fn tick_by_tick(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    tick_type: &str,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<RequestMessage> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::ReqTickByTickData);
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

pub fn cancel_tick_by_tick(request_id: i32) -> Result<RequestMessage> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::CancelTickByTickData);
    message.push_field(&request_id);

    Ok(message)
}

#[cfg(test)]
mod tests {
    use crate::contracts::contract_samples;

    use super::*;

    #[test]
    fn cancel_tick_by_tick() {
        let request_id = 9000;

        let results = super::cancel_tick_by_tick(request_id);

        match results {
            Ok(message) => {
                assert_eq!(message.encode(), "98\09000\0", "message.encode()");
            }
            Err(err) => {
                assert!(false, "error encoding cancel_tick_by_tick request: {err}");
            }
        }
    }

    #[test]
    fn cancel_realtime_bars() {
        let request_id = 9000;

        let results = super::cancel_realtime_bars(request_id);

        match results {
            Ok(message) => {
                assert_eq!(message.encode(), "51\01\09000\0", "message.encode()");
            }
            Err(err) => {
                assert!(false, "error encoding cancel_tick_by_tick request: {err}");
            }
        }
    }

    #[test]
    fn tick_by_tick() {
        let request_id = 9000;
        let server_version = server_versions::TICK_BY_TICK;
        let contract = contract_samples::simple_future();
        let tick_type = "AllLast";
        let number_of_ticks = 1;
        let ignore_size = true;
    
        let results = super::tick_by_tick(server_version, request_id, &contract, tick_type, number_of_ticks, ignore_size);

        match results {
            Ok(message) => {
                assert_eq!(message.encode(), "97\09000\00\0GBL\0FUT\0202303\00\0\0\0EUREX\0\0EUR\0\0\0AllLast\0", "message.encode()");
            }
            Err(err) => {
                assert!(false, "error encoding tick_by_tick request: {err}");
            }
        }
    }

    #[test]
    fn realtime_bars() {
        let request_id = 9000;
        let server_version = server_versions::TICK_BY_TICK;
        let contract = contract_samples::simple_future();
        let bar_size = BarSize::Secs5;
        let what_to_show = WhatToShow::Trades;
        let use_rth = true;
        let options = vec![];

        let results = super::encode_request_realtime_bars(server_version, request_id, &contract, &bar_size, &what_to_show, use_rth, options);

        match results {
            Ok(message) => {
                assert_eq!(message.encode(), "50\08\09000\00\0GBL\0FUT\0202303\00\0\0\0EUREX\0\0EUR\0\0\00\0TRADES\01\0\0", "message.encode()");
            }
            Err(err) => {
                assert!(false, "error encoding realtime_bars request: {err}");
            }
        }
    }
}
