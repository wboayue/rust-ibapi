use crate::messages::OutgoingMessages;
use crate::messages::RequestMessage;
use crate::Error;

pub(crate) fn request_positions() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::RequestPositions, 1)
}

fn encode_simple(message_type: OutgoingMessages, version: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&message_type);
    message.push_field(&version);

    Ok(message)
}

pub(crate) fn cancel_positions() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::CancelPositions, 1)
}

pub(crate) fn request_family_codes() -> Result<RequestMessage, Error> {
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

pub(crate) fn encode_request_pnl_single(
    request_id: i32,
    account: &str,
    contract_id: i32,
    model_code: Option<&str>,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestPnLSingle);
    message.push_field(&request_id);
    message.push_field(&account);

    if let Some(model_code) = model_code {
        message.push_field(&model_code);
    } else {
        message.push_field(&"");
    }

    // https://github.com/InteractiveBrokers/tws-api/blob/2724a8eaa67600ce2d876b010667a8f6a22fe298/source/csharpclient/client/EClient.cs#L2794
// https://github.com/InteractiveBrokers/tws-api/blob/2724a8eaa67600ce2d876b010667a8f6a22fe298/source/csharpclient/client/EDecoder.cs#L674
// https://github.com/InteractiveBrokers/tws-api/blob/2724a8eaa67600ce2d876b010667a8f6a22fe298/source/csharpclient/client/EClient.cs#L2744

    message.push_field(&contract_id);

    Ok(message)
}

#[cfg(test)]
mod tests;
