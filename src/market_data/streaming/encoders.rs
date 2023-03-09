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