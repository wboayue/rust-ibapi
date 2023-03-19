use anyhow::Result;

use super::{BarSize, WhatToShow};
use crate::client::RequestMessage;
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::orders::TagValue;
use crate::server_versions;

pub(crate) fn encode_request_realtime_bars(
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

pub(crate) fn cancel_realtime_bars(request_id: i32) -> Result<RequestMessage> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::CancelRealTimeBars);
    message.push_field(&VERSION);
    message.push_field(&request_id);

    Ok(message)
}

pub(crate) fn tick_by_tick(
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

pub(crate) fn cancel_tick_by_tick(request_id: i32) -> Result<RequestMessage> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::CancelTickByTickData);
    message.push_field(&request_id);

    Ok(message)
}

#[cfg(test)]
mod tests {
    use crate::{contracts::contract_samples, ToField};

    use super::*;

    #[test]
    fn cancel_tick_by_tick() {
        let request_id = 9000;

        let results = super::cancel_tick_by_tick(request_id);

        match results {
            Ok(message) => {
                assert_eq!(message[0], "98", "message.type");
                assert_eq!(message[1], request_id.to_string(), "message.request_id");
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
                assert_eq!(message[0], OutgoingMessages::CancelRealTimeBars.to_field(), "message.type");
                assert_eq!(message[1], "1", "message.version");
                assert_eq!(message[2], request_id.to_string(), "message.request_id");
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
                assert_eq!(message[0], OutgoingMessages::ReqTickByTickData.to_field(), "message.type");
                assert_eq!(message[1], request_id.to_field(), "message.request_id");
                assert_eq!(message[2], contract.contract_id.to_field(), "message.contract_id");
                assert_eq!(message[3], contract.symbol, "message.symbol");
                assert_eq!(message[4], contract.security_type.to_field(), "message.security_type");
                assert_eq!(
                    message[5], contract.last_trade_date_or_contract_month,
                    "message.last_trade_date_or_contract_month"
                );
                assert_eq!(message[6], contract.strike.to_field(), "message.strike");
                assert_eq!(message[7], contract.right, "message.right");
                assert_eq!(message[8], contract.multiplier, "message.multiplier");
                assert_eq!(message[9], contract.exchange, "message.exchange");
                assert_eq!(message[10], contract.primary_exchange, "message.primary_exchange");
                assert_eq!(message[11], contract.currency, "message.currency");
                assert_eq!(message[12], contract.local_symbol, "message.local_symbol");
                assert_eq!(message[13], contract.trading_class, "message.trading_class");
                assert_eq!(message[14], tick_type, "message.tick_type");
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
                assert_eq!(message[0], OutgoingMessages::RequestRealTimeBars.to_field(), "message.type");
                assert_eq!(message[1], "8", "message.version");
                assert_eq!(message[2], request_id.to_field(), "message.request_id");
                assert_eq!(message[3], contract.contract_id.to_field(), "message.contract_id");
                assert_eq!(message[4], contract.symbol, "message.symbol");
                assert_eq!(message[5], contract.security_type.to_field(), "message.security_type");
                assert_eq!(
                    message[6], contract.last_trade_date_or_contract_month,
                    "message.last_trade_date_or_contract_month"
                );
                assert_eq!(message[7], contract.strike.to_field(), "message.strike");
                assert_eq!(message[8], contract.right, "message.right");
                assert_eq!(message[9], contract.multiplier, "message.multiplier");
                assert_eq!(message[10], contract.exchange, "message.exchange");
                assert_eq!(message[11], contract.primary_exchange, "message.primary_exchange");
                assert_eq!(message[12], contract.currency, "message.currency");
                assert_eq!(message[13], contract.local_symbol, "message.local_symbol");
                assert_eq!(message[14], contract.trading_class, "message.trading_class");
                assert_eq!(message[15], "0", "message.bar_size");
                assert_eq!(message[16], what_to_show.to_field(), "message.what_to_show"); // implement to_field
                assert_eq!(message[17], use_rth.to_field(), "message.use_rth");
                assert_eq!(message[18], "", "message.options"); // TODO what should this be?
            }
            Err(err) => {
                assert!(false, "error encoding realtime_bars request: {err}");
            }
        }
    }
}
