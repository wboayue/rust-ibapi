use crate::messages::OutgoingMessages;
use crate::messages::RequestMessage;
use crate::Error;

#[cfg(test)]
#[path = "encoders_temp/tests.rs"]
mod tests;

pub(super) fn encode_request_positions() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::RequestPositions, 1)
}

pub(super) fn encode_cancel_positions() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::CancelPositions, 1)
}

pub(super) fn encode_cancel_account_summary(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    const VERSION: i32 = 1;

    message.push_field(&OutgoingMessages::CancelAccountSummary);
    message.push_field(&VERSION);
    message.push_field(&request_id);

    Ok(message)
}

pub(super) fn encode_request_positions_multi(request_id: i32, account: Option<&str>, model_code: Option<&str>) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    const VERSION: i32 = 1;

    message.push_field(&OutgoingMessages::RequestPositionsMulti);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    message.push_field(&account);
    message.push_field(&model_code);

    Ok(message)
}

pub(super) fn encode_cancel_positions_multi(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    const VERSION: i32 = 1;

    message.push_field(&OutgoingMessages::CancelPositionsMulti);
    message.push_field(&VERSION);
    message.push_field(&request_id);

    Ok(message)
}

pub(super) fn encode_request_family_codes() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::RequestFamilyCodes, 1)
}

pub(super) fn encode_request_pnl(request_id: i32, account: &str, model_code: Option<&str>) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestPnL);
    message.push_field(&request_id);
    message.push_field(&account);
    message.push_field(&model_code);

    Ok(message)
}

pub(super) fn encode_cancel_pnl(request_id: i32) -> Result<RequestMessage, Error> {
    encode_simple_with_request_id(OutgoingMessages::CancelPnL, request_id)
}

pub(super) fn encode_request_pnl_single(request_id: i32, account: &str, contract_id: i32, model_code: Option<&str>) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestPnLSingle);
    message.push_field(&request_id);
    message.push_field(&account);
    message.push_field(&model_code);
    message.push_field(&contract_id);

    Ok(message)
}

pub(super) fn encode_cancel_pnl_single(request_id: i32) -> Result<RequestMessage, Error> {
    encode_simple_with_request_id(OutgoingMessages::CancelPnLSingle, request_id)
}

pub(super) fn encode_request_account_summary(request_id: i32, group: &str, tags: &[&str]) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestAccountSummary);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    message.push_field(&group);
    message.push_field(&tags.join(","));

    Ok(message)
}

pub(super) fn encode_request_managed_accounts() -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;
    encode_simple(OutgoingMessages::RequestManagedAccounts, VERSION)
}

pub(super) fn encode_request_account_updates(server_version: i32, account: &str) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 2;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestAccountData);
    message.push_field(&VERSION);
    message.push_field(&true); // subscribe
    if server_version > 9 {
        message.push_field(&account);
    }

    Ok(message)
}

pub(super) fn encode_request_account_updates_multi(
    request_id: i32,
    account: Option<&str>,
    model_code: Option<&str>,
) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestAccountUpdatesMulti);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    message.push_field(&account);
    message.push_field(&model_code);
    message.push_field(&true); // subscribe

    Ok(message)
}

pub(super) fn encode_cancel_account_updates(server_version: i32) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 2;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestAccountData);
    message.push_field(&VERSION);
    message.push_field(&false); // subscribe
    if server_version > 9 {
        message.push_field(&"");
    }

    Ok(message)
}

pub(super) fn encode_cancel_account_updates_multi(_server_version: i32, request_id: i32) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::CancelAccountUpdatesMulti);
    message.push_field(&VERSION);
    message.push_field(&request_id);

    Ok(message)
}

pub(super) fn encode_request_server_time() -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;
    encode_simple(OutgoingMessages::RequestCurrentTime, VERSION)
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
