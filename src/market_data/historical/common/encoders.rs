use time::{format_description::FormatItem, macros::format_description, OffsetDateTime};
use time_tz::OffsetDateTimeExt;

use crate::contracts::Contract;
use crate::messages::{OutgoingMessages, RequestMessage};
use crate::{server_versions, Error, ToField};

use crate::market_data::historical::{BarSize, Duration, WhatToShow};

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
pub(crate) fn encode_request_head_timestamp(
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
pub(crate) fn encode_request_historical_data(
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
pub(crate) fn encode_request_historical_ticks(
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

pub(crate) fn encode_cancel_historical_data(request_id: i32) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();
    message.push_field(&OutgoingMessages::CancelHistoricalData);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    Ok(message)
}

pub(crate) fn encode_cancel_historical_ticks(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();
    message.push_field(&OutgoingMessages::CancelHistoricalTicks);
    message.push_field(&request_id);
    Ok(message)
}

pub(crate) fn encode_request_histogram_data(request_id: i32, contract: &Contract, use_rth: bool, period: BarSize) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestHistogramData);
    message.push_field(&request_id);
    contract.push_fields(&mut message);
    message.push_field(&use_rth);
    message.push_field(&period);

    Ok(message)
}

// === Protobuf Encoders ===

#[allow(dead_code)]
pub(crate) fn encode_request_head_timestamp_proto(
    request_id: i32,
    contract: &Contract,
    what_to_show: WhatToShow,
    use_rth: bool,
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::HeadTimestampRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        use_rth: if use_rth { Some(true) } else { None },
        what_to_show: Some(what_to_show.to_field()),
        format_date: Some(DATE_FORMAT),
    };
    Ok(encode_protobuf_message(
        crate::messages::OutgoingMessages::RequestHeadTimestamp as i32,
        &request.encode_to_vec(),
    ))
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_request_historical_data_proto(
    request_id: i32,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
    bar_size: BarSize,
    what_to_show: Option<WhatToShow>,
    use_rth: bool,
    keep_up_to_date: bool,
    chart_options: &[crate::contracts::TagValue],
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let end_str = end_date.to_field();
    let wts_str = what_to_show.to_field();
    let request = crate::proto::HistoricalDataRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        end_date_time: if end_str.is_empty() { None } else { Some(end_str) },
        duration: Some(duration.to_field()),
        bar_size_setting: Some(bar_size.to_field()),
        what_to_show: if wts_str.is_empty() { None } else { Some(wts_str) },
        use_rth: if use_rth { Some(true) } else { None },
        format_date: Some(DATE_FORMAT),
        keep_up_to_date: if keep_up_to_date { Some(true) } else { None },
        chart_options: crate::proto::encoders::tag_values_to_map(chart_options),
    };
    Ok(encode_protobuf_message(
        crate::messages::OutgoingMessages::RequestHistoricalData as i32,
        &request.encode_to_vec(),
    ))
}

#[allow(dead_code)]
pub(crate) fn encode_cancel_historical_data_proto(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelHistoricalData, crate::messages::OutgoingMessages::CancelHistoricalData)
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_request_historical_ticks_proto(
    request_id: i32,
    contract: &Contract,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    number_of_ticks: i32,
    what_to_show: WhatToShow,
    use_rth: bool,
    ignore_size: bool,
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let start_str = start.to_field();
    let end_str = end.to_field();
    let request = crate::proto::HistoricalTicksRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        start_date_time: if start_str.is_empty() { None } else { Some(start_str) },
        end_date_time: if end_str.is_empty() { None } else { Some(end_str) },
        number_of_ticks: Some(number_of_ticks),
        what_to_show: Some(what_to_show.to_field()),
        use_rth: if use_rth { Some(true) } else { None },
        ignore_size: if ignore_size { Some(true) } else { None },
        misc_options: Default::default(),
    };
    Ok(encode_protobuf_message(
        crate::messages::OutgoingMessages::RequestHistoricalTicks as i32,
        &request.encode_to_vec(),
    ))
}

#[allow(dead_code)]
pub(crate) fn encode_cancel_historical_ticks_proto(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(
        request_id,
        CancelHistoricalTicks,
        crate::messages::OutgoingMessages::CancelHistoricalTicks
    )
}

#[allow(dead_code)]
pub(crate) fn encode_request_histogram_data_proto(request_id: i32, contract: &Contract, use_rth: bool, period: BarSize) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::HistogramDataRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        use_rth: Some(use_rth),
        time_period: Some(period.to_field()),
    };
    Ok(encode_protobuf_message(
        crate::messages::OutgoingMessages::RequestHistogramData as i32,
        &request.encode_to_vec(),
    ))
}

#[allow(dead_code)]
pub(crate) fn encode_cancel_histogram_data_proto(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelHistogramData, crate::messages::OutgoingMessages::CancelHistogramData)
}

#[allow(dead_code)]
pub(crate) fn encode_cancel_head_timestamp_proto(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelHeadTimestamp, crate::messages::OutgoingMessages::CancelHeadTimestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::historical::ToDuration;
    use crate::messages::OutgoingMessages;
    use crate::ToField;
    use time::macros::datetime;
    use time_tz::{self, PrimitiveDateTimeExt};

    #[test]
    fn test_encode_request_head_timestamp() {
        let request_id = 9000;
        let contract = Contract::stock("MSFT").build();
        let what_to_show = WhatToShow::Trades;
        let use_rth = false;

        let message = encode_request_head_timestamp(request_id, &contract, what_to_show, use_rth).expect("error encoding request head timestamp");

        assert_eq!(message[0], OutgoingMessages::RequestHeadTimestamp.to_field(), "message.type");
        assert_eq!(message[1], request_id.to_field(), "message.request_id");
        assert_eq!(message[2], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(message[3], contract.symbol.to_field(), "message.symbol");
        assert_eq!(message[4], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            message[5], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(message[6], contract.strike.to_field(), "message.strike");
        assert_eq!(message[7], contract.right, "message.right");
        assert_eq!(message[8], contract.multiplier, "message.multiplier");
        assert_eq!(message[9], contract.exchange.to_field(), "message.exchange");
        assert_eq!(message[10], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(message[11], contract.currency.to_field(), "message.currency");
        assert_eq!(message[12], contract.local_symbol, "message.local_symbol");
        assert_eq!(message[13], contract.trading_class, "message.trading_class");
        assert_eq!(message[14], contract.include_expired.to_field(), "message.include_expired");
        assert_eq!(message[15], use_rth.to_field(), "message.use_rth");
        assert_eq!(message[16], what_to_show.to_field(), "message.what_to_show");
        assert_eq!(message[17], DATE_FORMAT.to_field(), "message.date_format");
    }

    #[test]
    fn test_encode_request_historical_data() {
        let request_id = 9000;
        let contract = Contract::stock("MSFT").build();
        let end_date = Some(datetime!(2023-04-10 14:00 UTC));
        let duration = 30.days();
        let bar_size = BarSize::Day;
        let what_to_show: Option<WhatToShow> = None;
        let use_rth = false;
        let keep_up_to_date = true;
        let chart_options = Vec::<crate::contracts::TagValue>::default();

        let message = encode_request_historical_data(
            server_versions::SYNT_REALTIME_BARS,
            request_id,
            &contract,
            end_date,
            duration,
            bar_size,
            what_to_show,
            use_rth,
            keep_up_to_date,
            chart_options,
        )
        .expect("error encoding historical data");

        assert_eq!(message[0], OutgoingMessages::RequestHistoricalData.to_field(), "message.type");
        assert_eq!(message[1], request_id.to_field(), "message.request_id");
        assert_eq!(message[2], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(message[3], contract.symbol.to_field(), "message.symbol");
        assert_eq!(message[4], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            message[5], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(message[6], contract.strike.to_field(), "message.strike");
        assert_eq!(message[7], contract.right, "message.right");
        assert_eq!(message[8], contract.multiplier, "message.multiplier");
        assert_eq!(message[9], contract.exchange.to_field(), "message.exchange");
        assert_eq!(message[10], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(message[11], contract.currency.to_field(), "message.currency");
        assert_eq!(message[12], contract.local_symbol, "message.local_symbol");
        assert_eq!(message[13], contract.trading_class, "message.trading_class");
        assert_eq!(message[14], contract.include_expired.to_field(), "message.include_expired");
        assert_eq!(message[15], end_date.to_field(), "message.end_date");
        assert_eq!(message[16], bar_size.to_field(), "message.bar_size");
        assert_eq!(message[17], duration.to_field(), "message.duration");
        assert_eq!(message[18], use_rth.to_field(), "message.use_rth");
        assert_eq!(message[19], what_to_show.to_field(), "message.what_to_show");
        assert_eq!(message[20], DATE_FORMAT.to_field(), "message.date_format");

        let mut i: usize = 21;
        if contract.is_bag() {
            assert_eq!(message[i], contract.combo_legs.len().to_field(), "message.combo_legs_count");
            i += 1;

            for combo_leg in &contract.combo_legs {
                assert_eq!(message[i], combo_leg.contract_id.to_field(), "message.contract_id");
                i += 1;
                assert_eq!(message[i], combo_leg.ratio.to_field(), "message.ratio");
                i += 1;
                assert_eq!(message[i], combo_leg.action.to_field(), "message.action");
                i += 1;
                assert_eq!(message[i], combo_leg.exchange.to_field(), "message.exchange");
                i += 1;
            }
        }

        assert_eq!(message[i], keep_up_to_date.to_field(), "message.keep_up_to_date");
        assert_eq!(message[i + 1], "", "message.chart_options");
    }

    #[test]
    fn test_encode_interval() {
        let ny = time_tz::timezones::db::america::NEW_YORK;

        let empty_end: Option<OffsetDateTime> = None;
        let valid_end_utc: Option<OffsetDateTime> = Some(datetime!(2023-04-15 10:00 UTC));
        let valid_end_ny: Option<OffsetDateTime> = Some(datetime!(2023-04-15 10:00).assume_timezone(ny).unwrap());

        assert_eq!(empty_end.to_field(), "", "encode empty end");
        assert_eq!(valid_end_utc.to_field(), "20230415 10:00:00 UTC", "encode end utc");
        assert_eq!(valid_end_ny.to_field(), "20230415 14:00:00 UTC", "encode end from America/NewYork");
    }

    #[test]
    fn test_encode_request_historical_ticks() {
        let request_id = 9000;
        let contract = Contract::stock("MSFT").build();
        let start: Option<OffsetDateTime> = Some(datetime!(2023-04-10 14:00 UTC));
        let end: Option<OffsetDateTime> = None;
        let what_to_show = WhatToShow::Trades;
        let number_of_ticks = 100;
        let ignore_size = false;
        let use_rth = false;

        let message = encode_request_historical_ticks(request_id, &contract, start, end, number_of_ticks, what_to_show, use_rth, ignore_size)
            .expect("error encoding historical ticks");

        assert_eq!(message[0], OutgoingMessages::RequestHistoricalTicks.to_field(), "message.type");
        assert_eq!(message[1], request_id.to_field(), "message.request_id");
        assert_eq!(message[2], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(message[3], contract.symbol.to_field(), "message.symbol");
        assert_eq!(message[4], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            message[5], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(message[6], contract.strike.to_field(), "message.strike");
        assert_eq!(message[7], contract.right, "message.right");
        assert_eq!(message[8], contract.multiplier, "message.multiplier");
        assert_eq!(message[9], contract.exchange.to_field(), "message.exchange");
        assert_eq!(message[10], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(message[11], contract.currency.to_field(), "message.currency");
        assert_eq!(message[12], contract.local_symbol, "message.local_symbol");
        assert_eq!(message[13], contract.trading_class, "message.trading_class");
        assert_eq!(message[14], contract.include_expired.to_field(), "message.include_expired");
        assert_eq!(message[15], start.to_field(), "message.start");
        assert_eq!(message[16], end.to_field(), "message.end");
        assert_eq!(message[17], number_of_ticks.to_field(), "message.number_of_ticks");
        assert_eq!(message[18], what_to_show.to_field(), "message.what_to_show");
        assert_eq!(message[19], use_rth.to_field(), "message.use_rth");
        assert_eq!(message[20], ignore_size.to_field(), "message.ignore_size");
        assert_eq!(message[21], "", "message.misc_options");
    }

    #[test]
    fn test_encode_cancel_historical_data() {
        let request_id = 9001;

        let message = encode_cancel_historical_data(request_id).expect("error encoding cancel historical data");

        assert_eq!(message[0], OutgoingMessages::CancelHistoricalData.to_field(), "message.type");
        assert_eq!(message[1], "1", "message.version");
        assert_eq!(message[2], request_id.to_field(), "message.request_id");
    }

    #[test]
    fn test_encode_cancel_historical_ticks() {
        let request_id = 9001;

        let message = encode_cancel_historical_ticks(request_id).expect("error encoding cancel historical ticks");

        assert_eq!(message[0], OutgoingMessages::CancelHistoricalTicks.to_field(), "message.type");
        assert_eq!(message[1], request_id.to_field(), "message.request_id");
    }

    #[test]
    fn test_encode_request_histogram_data() {
        let request_id = 3000;
        let contract = Contract::stock("MSFT").build();
        let period = BarSize::Week;
        let use_rth = true;

        let message = encode_request_histogram_data(request_id, &contract, use_rth, period).expect("error encoding request histogram data");

        assert_eq!(message[0], OutgoingMessages::RequestHistogramData.to_field(), "message.message_type");
        assert_eq!(message[1], request_id.to_field(), "message.request_id");
        assert_eq!(message[2], contract.contract_id.to_field(), "message.contract_id");
        assert_eq!(message[3], contract.symbol.to_field(), "message.symbol");
        assert_eq!(message[4], contract.security_type.to_field(), "message.security_type");
        assert_eq!(
            message[5], contract.last_trade_date_or_contract_month,
            "message.last_trade_date_or_contract_month"
        );
        assert_eq!(message[6], contract.strike.to_field(), "message.strike");
        assert_eq!(message[7], contract.right, "message.right");
        assert_eq!(message[8], contract.multiplier, "message.multiplier");
        assert_eq!(message[9], contract.exchange.to_field(), "message.exchange");
        assert_eq!(message[10], contract.primary_exchange.to_field(), "message.primary_exchange");
        assert_eq!(message[11], contract.currency.to_field(), "message.currency");
        assert_eq!(message[12], contract.local_symbol, "message.local_symbol");
        assert_eq!(message[13], contract.trading_class, "message.trading_class");
        assert_eq!(message[14], contract.include_expired.to_field(), "message.include_expired");
        assert_eq!(message[15], use_rth.to_field(), "message.use_rth");
        assert_eq!(message[16], period.to_field(), "message.duration");
    }

    #[cfg(test)]
    mod proto_tests {
        use super::*;
        use crate::common::test_utils::helpers::assert_proto_msg_id;
        use crate::market_data::historical::ToDuration;

        #[test]
        fn test_encode_request_head_timestamp_proto() {
            let contract = Contract::stock("MSFT").build();
            let bytes = encode_request_head_timestamp_proto(9000, &contract, WhatToShow::Trades, false).unwrap();
            assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::RequestHeadTimestamp);
        }

        #[test]
        fn test_encode_request_historical_data_proto() {
            let contract = Contract::stock("MSFT").build();
            let bytes = encode_request_historical_data_proto(9000, &contract, None, 30.days(), BarSize::Day, None, false, true, &[]).unwrap();
            assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::RequestHistoricalData);
        }

        #[test]
        fn test_encode_cancel_historical_data_proto() {
            let bytes = encode_cancel_historical_data_proto(9001).unwrap();
            assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::CancelHistoricalData);
        }

        #[test]
        fn test_encode_request_histogram_data_proto() {
            let contract = Contract::stock("MSFT").build();
            let bytes = encode_request_histogram_data_proto(3000, &contract, true, BarSize::Week).unwrap();
            assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::RequestHistogramData);
        }

        #[test]
        fn test_encode_cancel_head_timestamp_proto() {
            let bytes = encode_cancel_head_timestamp_proto(9000).unwrap();
            assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::CancelHeadTimestamp);
        }

        #[test]
        fn test_encode_cancel_histogram_data_proto() {
            let bytes = encode_cancel_histogram_data_proto(3000).unwrap();
            assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::CancelHistogramData);
        }
    }
}
