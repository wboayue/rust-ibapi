//! Encoders for Wall Street Horizon messages

use time::Date;

use crate::messages::{OutgoingMessages, RequestMessage};
use crate::protocol::{is_supported, Features};
use crate::wsh::AutoFill;
use crate::Error;

pub(in crate::wsh) fn encode_request_wsh_metadata(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestWshMetaData);
    message.push_field(&request_id);

    Ok(message)
}

pub(in crate::wsh) fn encode_cancel_wsh_metadata(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::CancelWshMetaData);
    message.push_field(&request_id);

    Ok(message)
}

#[allow(clippy::too_many_arguments)]
pub(in crate::wsh) fn encode_request_wsh_event_data(
    server_version: i32,
    request_id: i32,
    contract_id: Option<i32>,
    filter: Option<&str>,
    start_date: Option<Date>,
    end_date: Option<Date>,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestWshEventData);
    message.push_field(&request_id);
    message.push_field(&contract_id);

    if is_supported(server_version, Features::WSH_EVENT_DATA_FILTERS) {
        message.push_field(&filter);
        if let Some(auto_fill) = auto_fill {
            message.push_field(&auto_fill.watchlist);
            message.push_field(&auto_fill.portfolio);
            message.push_field(&auto_fill.competitors);
        } else {
            message.push_field(&false);
            message.push_field(&false);
            message.push_field(&false);
        }
    }

    if is_supported(server_version, Features::WSH_EVENT_DATA_FILTERS_DATE) {
        message.push_field(&start_date);
        message.push_field(&end_date);
        message.push_field(&limit);
    }

    Ok(message)
}

pub(in crate::wsh) fn encode_cancel_wsh_event_data(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::CancelWshEventData);
    message.push_field(&request_id);

    Ok(message)
}