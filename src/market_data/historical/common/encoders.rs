use time::{format_description::FormatItem, macros::format_description, OffsetDateTime};
use time_tz::OffsetDateTimeExt;

use crate::contracts::Contract;
use crate::Error;
use crate::ToField;

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

pub(crate) fn encode_request_head_timestamp(request_id: i32, contract: &Contract, what_to_show: WhatToShow, use_rth: bool) -> Result<Vec<u8>, Error> {
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_request_historical_data(
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

pub(crate) fn encode_cancel_historical_data(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelHistoricalData, crate::messages::OutgoingMessages::CancelHistoricalData)
}

pub(crate) fn encode_cancel_historical_ticks(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(
        request_id,
        CancelHistoricalTicks,
        crate::messages::OutgoingMessages::CancelHistoricalTicks
    )
}

pub(crate) fn encode_request_histogram_data(request_id: i32, contract: &Contract, use_rth: bool, period: BarSize) -> Result<Vec<u8>, Error> {
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
pub(crate) fn encode_cancel_histogram_data(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelHistogramData, crate::messages::OutgoingMessages::CancelHistogramData)
}

#[allow(dead_code)]
pub(crate) fn encode_cancel_head_timestamp(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelHeadTimestamp, crate::messages::OutgoingMessages::CancelHeadTimestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_utils::helpers::assert_proto_msg_id;
    use crate::contracts::Contract;
    use crate::market_data::historical::ToDuration;
    use time::macros::datetime;
    use time_tz::{self, PrimitiveDateTimeExt};

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
    fn test_encode_request_head_timestamp() {
        let contract = Contract::stock("MSFT").build();
        let bytes = encode_request_head_timestamp(9000, &contract, WhatToShow::Trades, false).unwrap();
        assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::RequestHeadTimestamp);

        use prost::Message;
        let req = crate::proto::HeadTimestampRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(9000));
        assert_eq!(req.what_to_show.as_deref(), Some("TRADES"));
        assert_eq!(req.format_date, Some(2));
        assert!(req.use_rth.is_none());
    }

    #[test]
    fn test_encode_request_historical_data() {
        let contract = Contract::stock("MSFT").build();
        let bytes = encode_request_historical_data(9000, &contract, None, 30.days(), BarSize::Day, None, false, true, &[]).unwrap();
        assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::RequestHistoricalData);

        use prost::Message;
        let req = crate::proto::HistoricalDataRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(9000));
        assert_eq!(req.contract.unwrap().symbol.as_deref(), Some("MSFT"));
        assert_eq!(req.bar_size_setting.as_deref(), Some("1 day"));
        assert!(req.end_date_time.is_none());
        assert_eq!(req.keep_up_to_date, Some(true));
    }

    #[test]
    fn test_encode_cancel_historical_data() {
        let bytes = encode_cancel_historical_data(9001).unwrap();
        assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::CancelHistoricalData);
    }

    #[test]
    fn test_encode_request_historical_ticks() {
        let contract = Contract::stock("MSFT").build();
        let start: Option<OffsetDateTime> = Some(datetime!(2023-04-10 14:00 UTC));
        let bytes = encode_request_historical_ticks(9000, &contract, start, None, 100, WhatToShow::Trades, false, false).unwrap();
        assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::RequestHistoricalTicks);

        use prost::Message;
        let req = crate::proto::HistoricalTicksRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(9000));
        assert_eq!(req.number_of_ticks, Some(100));
        assert!(req.start_date_time.is_some());
        assert!(req.end_date_time.is_none());
    }

    #[test]
    fn test_encode_cancel_historical_ticks() {
        let bytes = encode_cancel_historical_ticks(9001).unwrap();
        assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::CancelHistoricalTicks);
    }

    #[test]
    fn test_encode_request_histogram_data() {
        let contract = Contract::stock("MSFT").build();
        let bytes = encode_request_histogram_data(3000, &contract, true, BarSize::Week).unwrap();
        assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::RequestHistogramData);
    }

    #[test]
    fn test_encode_cancel_head_timestamp() {
        let bytes = encode_cancel_head_timestamp(9000).unwrap();
        assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::CancelHeadTimestamp);
    }

    #[test]
    fn test_encode_cancel_histogram_data() {
        let bytes = encode_cancel_histogram_data(3000).unwrap();
        assert_proto_msg_id(&bytes, crate::messages::OutgoingMessages::CancelHistogramData);
    }
}
