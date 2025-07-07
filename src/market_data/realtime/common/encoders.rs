use crate::contracts::{Contract, SecurityType};
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::market_data::realtime::{BarSize, WhatToShow};
use crate::orders::TagValue;
use crate::{server_versions, Error};

#[cfg(test)]
mod tests;

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
