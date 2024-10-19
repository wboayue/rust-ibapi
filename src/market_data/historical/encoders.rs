use time::{format_description::FormatItem, macros::format_description};
use time_tz::OffsetDateTimeExt;

use crate::messages::OutgoingMessages;

use super::*;

const DATE_FORMAT: i32 = 2; // 1 for yyyyMMdd HH:mm:ss, 2 for system time format in seconds.
const END_DATE_FORMAT: &[FormatItem] = format_description!("[year][month][day] [hour]:[minute]:[second]");

impl ToField for OffsetDateTime {
    fn to_field(&self) -> String {
        let adjusted = self.to_timezone(time_tz::timezones::db::UTC);
        let formatted = adjusted.format(END_DATE_FORMAT).unwrap();
        format!("{formatted} UTC")
    }
}

impl ToField for Option<OffsetDateTime> {
    fn to_field(&self) -> String {
        match self {
            Some(date_time) => date_time.to_field(),
            None => "".into(),
        }
    }
}

// Encodes the head timestamp request
pub(super) fn encode_request_head_timestamp(
    request_id: i32,
    contract: &Contract,
    what_to_show: WhatToShow,
    use_rth: bool,
) -> Result<RequestMessage, Error> {
    let mut packet = RequestMessage::default();

    packet.push_field(&OutgoingMessages::RequestHeadTimestamp);
    packet.push_field(&request_id);
    contract.push_fields(&mut packet);
    packet.push_field(&use_rth);
    packet.push_field(&what_to_show);
    packet.push_field(&DATE_FORMAT);

    Ok(packet)
}

// Encodes the historical data request
#[allow(clippy::too_many_arguments)]
pub(super) fn encode_request_historical_data(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
    bar_size: BarSize,
    what_to_show: Option<WhatToShow>,
    use_rth: bool,
    keep_up_to_data: bool,
    chart_options: Vec<crate::contracts::TagValue>,
) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 6;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestHistoricalData);

    if server_version < server_versions::SYNT_REALTIME_BARS {
        message.push_field(&VERSION);
    }

    message.push_field(&request_id);

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
    message.push_field(&contract.primary_exchange);
    message.push_field(&contract.currency);
    message.push_field(&contract.local_symbol);

    if server_version >= server_versions::TRADING_CLASS {
        message.push_field(&contract.trading_class);
    }

    message.push_field(&contract.include_expired);

    message.push_field(&end_date);
    message.push_field(&bar_size);

    message.push_field(&duration);
    message.push_field(&use_rth);
    message.push_field(&what_to_show);

    message.push_field(&DATE_FORMAT);

    if contract.is_bag() {
        message.push_field(&contract.combo_legs.len());

        for combo_leg in &contract.combo_legs {
            message.push_field(&combo_leg.contract_id);
            message.push_field(&combo_leg.ratio);
            message.push_field(&combo_leg.action);
            message.push_field(&combo_leg.exchange);
        }
    }

    if server_version >= server_versions::SYNT_REALTIME_BARS {
        message.push_field(&keep_up_to_data);
    }

    if server_version >= server_versions::LINKING {
        message.push_field(&chart_options); // chart options
    }

    Ok(message)
}

// Encodes message to request historical ticks
#[allow(clippy::too_many_arguments)]
pub(super) fn encode_request_historical_ticks(
    request_id: i32,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    what_to_show: WhatToShow,
    use_rth: bool,
    ignore_size: bool,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestHistoricalTicks);
    message.push_field(&request_id);
    contract.push_fields(&mut message);
    message.push_field(&start);
    message.push_field(&end);
    message.push_field(&number_of_ticks);
    message.push_field(&what_to_show);
    message.push_field(&use_rth);
    message.push_field(&ignore_size);
    message.push_field(&""); // misc options

    Ok(message)
}

pub(super) fn encode_request_histogram_data(request_id: i32, contract: &Contract, use_rth: bool, period: BarSize) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestHistogramData);
    message.push_field(&request_id);
    contract.push_fields(&mut message);
    message.push_field(&use_rth);
    message.push_field(&period);

    Ok(message)
}

#[cfg(test)]
mod tests;
