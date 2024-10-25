use crate::{contracts::contract_samples, ToField};

use super::*;

#[test]
fn test_cancel_tick_by_tick() {
    let request_id = 9000;
    let message = super::encode_cancel_tick_by_tick(request_id).expect("error encoding cancel_tick_by_tick");

    assert_eq!(message[0], "98", "message.type");
    assert_eq!(message[1], request_id.to_string(), "message.request_id");
}

#[test]
fn test_cancel_realtime_bars() {
    let request_id = 9000;

    let message = super::encode_cancel_realtime_bars(request_id).expect("error encoding cancel_tick_by_tick");

    assert_eq!(message[0], OutgoingMessages::CancelRealTimeBars.to_field(), "message.type");
    assert_eq!(message[1], "1", "message.version");
    assert_eq!(message[2], request_id.to_string(), "message.request_id");
}

#[test]
fn test_tick_by_tick() {
    let request_id = 9000;
    let server_version = server_versions::TICK_BY_TICK;
    let contract = contract_samples::simple_future();
    let tick_type = "AllLast";
    let number_of_ticks = 1;
    let ignore_size = true;

    let message = super::encode_tick_by_tick(server_version, request_id, &contract, tick_type, number_of_ticks, ignore_size)
        .expect("error encoding tick_by_tick");

    assert_eq!(message[0], OutgoingMessages::RequestTickByTickData.to_field(), "message.type");
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

#[test]
fn test_realtime_bars() {
    let request_id = 9000;
    let server_version = server_versions::TICK_BY_TICK;
    let contract = contract_samples::simple_future();
    let bar_size = BarSize::Sec5;
    let what_to_show = WhatToShow::Trades;
    let use_rth = true;
    let options = vec![];

    let message = super::encode_request_realtime_bars(server_version, request_id, &contract, &bar_size, &what_to_show, use_rth, options)
        .expect("error encoding realtime_bars");

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
