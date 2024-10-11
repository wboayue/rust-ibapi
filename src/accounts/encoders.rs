use crate::messages::OutgoingMessages;
use crate::messages::RequestMessage;
use crate::Error;

pub(crate) fn encode_request_positions() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::RequestPositions, 1)
}

pub(crate) fn encode_cancel_positions() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::CancelPositions, 1)
}

pub(crate) fn encode_request_family_codes() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::RequestFamilyCodes, 1)
}

pub(crate) fn encode_request_pnl(request_id: i32, account: &str, model_code: Option<&str>) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestPnL);
    message.push_field(&request_id);
    message.push_field(&account);

    if let Some(model_code) = model_code {
        message.push_field(&model_code);
    } else {
        message.push_field(&"");
    }

    Ok(message)
}

pub(crate) fn encode_cancel_pnl(request_id: i32) -> Result<RequestMessage, Error> {
    encode_simple_with_request_id(OutgoingMessages::CancelPnL, request_id)
}

pub(crate) fn encode_request_pnl_single(request_id: i32, account: &str, contract_id: i32, model_code: Option<&str>) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestPnLSingle);
    message.push_field(&request_id);
    message.push_field(&account);

    if let Some(model_code) = model_code {
        message.push_field(&model_code);
    } else {
        message.push_field(&"");
    }

    message.push_field(&contract_id);

    Ok(message)
}

pub(crate) fn encode_cancel_pnl_single(request_id: i32) -> Result<RequestMessage, Error> {
    encode_simple_with_request_id(OutgoingMessages::CancelPnLSingle, request_id)
}

fn encode_simple(message_type: OutgoingMessages, version: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&message_type);
    message.push_field(&version);

    Ok(message)
}

fn encode_simple_with_request_id(message_type: OutgoingMessages, request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&message_type);
    message.push_field(&request_id);

    Ok(message)
}

#[cfg(test)]
mod tests;
