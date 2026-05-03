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
    use crate::proto::encoders::some_bool;
    use prost::Message;
    let request = crate::proto::HeadTimestampRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        use_rth: some_bool(use_rth),
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
    use crate::proto::encoders::{some_bool, some_str};
    use prost::Message;
    let end_str = end_date.to_field();
    let wts_str = what_to_show.to_field();
    let request = crate::proto::HistoricalDataRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        end_date_time: some_str(&end_str),
        duration: Some(duration.to_field()),
        bar_size_setting: Some(bar_size.to_field()),
        what_to_show: some_str(&wts_str),
        use_rth: some_bool(use_rth),
        format_date: Some(DATE_FORMAT),
        keep_up_to_date: some_bool(keep_up_to_date),
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
    use crate::proto::encoders::{some_bool, some_str};
    use prost::Message;
    let start_str = start.to_field();
    let end_str = end.to_field();
    let request = crate::proto::HistoricalTicksRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        start_date_time: some_str(&start_str),
        end_date_time: some_str(&end_str),
        number_of_ticks: Some(number_of_ticks),
        what_to_show: Some(what_to_show.to_field()),
        use_rth: some_bool(use_rth),
        ignore_size: some_bool(ignore_size),
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

// Per-encoder body assertions live in the migrated sync/async tests via
// `assert_request<B>(builder)`; cancel encoders are exercised through their
// production paths (e.g. subscription drop handlers).
#[cfg(test)]
mod tests {
    use super::*;
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
}
