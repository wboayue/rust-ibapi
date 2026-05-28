use crate::accounts::types::{AccountGroup, AccountId, ContractId, ModelCode};
use crate::messages::OutgoingMessages;
use crate::Error;

pub(in crate::accounts) fn encode_request_positions() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(PositionsRequest, OutgoingMessages::RequestPositions)
}

pub(in crate::accounts) fn encode_cancel_positions() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(CancelPositions, OutgoingMessages::CancelPositions)
}

pub(in crate::accounts) fn encode_request_account_summary(request_id: i32, group: &AccountGroup, tags: &[&str]) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::AccountSummaryRequest {
        req_id: Some(request_id),
        group: Some(group.as_str().to_string()),
        tags: Some(tags.join(",")),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestAccountSummary as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::accounts) fn encode_cancel_account_summary(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelAccountSummary, OutgoingMessages::CancelAccountSummary)
}

pub(in crate::accounts) fn encode_request_pnl(request_id: i32, account: &AccountId, model_code: Option<&ModelCode>) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::PnLRequest {
        req_id: Some(request_id),
        account: Some(account.to_string()),
        model_code: model_code.map(|m| m.to_string()),
    };
    Ok(encode_protobuf_message(OutgoingMessages::RequestPnL as i32, &request.encode_to_vec()))
}

pub(in crate::accounts) fn encode_cancel_pnl(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelPnL, OutgoingMessages::CancelPnL)
}

pub(in crate::accounts) fn encode_request_pnl_single(
    request_id: i32,
    account: &AccountId,
    contract_id: ContractId,
    model_code: Option<&ModelCode>,
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::PnLSingleRequest {
        req_id: Some(request_id),
        account: Some(account.to_string()),
        model_code: model_code.map(|m| m.to_string()),
        con_id: Some(contract_id.value()),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestPnLSingle as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::accounts) fn encode_cancel_pnl_single(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelPnLSingle, OutgoingMessages::CancelPnLSingle)
}

pub(in crate::accounts) fn encode_request_positions_multi(
    request_id: i32,
    account: Option<&AccountId>,
    model_code: Option<&ModelCode>,
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::PositionsMultiRequest {
        req_id: Some(request_id),
        account: account.map(|a| a.to_string()),
        model_code: model_code.map(|m| m.to_string()),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestPositionsMulti as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::accounts) fn encode_cancel_positions_multi(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelPositionsMulti, OutgoingMessages::CancelPositionsMulti)
}

pub(in crate::accounts) fn encode_request_account_updates(subscribe: bool, account: &AccountId) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use crate::proto::encoders::{some_bool, some_str};
    use prost::Message;
    let acct: &str = account;
    let request = crate::proto::AccountDataRequest {
        subscribe: some_bool(subscribe),
        acct_code: some_str(acct),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestAccountData as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::accounts) fn encode_cancel_account_updates() -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::AccountDataRequest {
        subscribe: Some(false),
        acct_code: None,
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestAccountData as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::accounts) fn encode_request_account_updates_multi(
    request_id: i32,
    account: Option<&AccountId>,
    model_code: Option<&ModelCode>,
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::AccountUpdatesMultiRequest {
        req_id: Some(request_id),
        account: account.map(|a| a.to_string()),
        model_code: model_code.map(|m| m.to_string()),
        ledger_and_nlv: Some(true),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestAccountUpdatesMulti as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::accounts) fn encode_cancel_account_updates_multi(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelAccountUpdatesMulti, OutgoingMessages::CancelAccountUpdatesMulti)
}

pub(crate) fn encode_request_managed_accounts() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(ManagedAccountsRequest, OutgoingMessages::RequestManagedAccounts)
}

pub(in crate::accounts) fn encode_request_family_codes() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(FamilyCodesRequest, OutgoingMessages::RequestFamilyCodes)
}

pub(in crate::accounts) fn encode_request_server_time() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(CurrentTimeRequest, OutgoingMessages::RequestCurrentTime)
}

pub(in crate::accounts) fn encode_request_server_time_millis() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(CurrentTimeInMillisRequest, OutgoingMessages::RequestCurrentTimeInMillis)
}

pub(in crate::accounts) fn encode_request_soft_dollar_tiers(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, SoftDollarTiersRequest, OutgoingMessages::RequestSoftDollarTiers)
}

pub(in crate::accounts) fn encode_request_user_info(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, UserInfoRequest, OutgoingMessages::RequestUserInfo)
}

pub(in crate::accounts) fn encode_request_fa(fa_data_type: i32) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::FaRequest {
        fa_data_type: Some(fa_data_type),
    };
    Ok(encode_protobuf_message(OutgoingMessages::RequestFA as i32, &request.encode_to_vec()))
}

pub(in crate::accounts) fn encode_replace_fa(request_id: i32, fa_data_type: i32, xml: &str) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::FaReplace {
        req_id: Some(request_id),
        fa_data_type: Some(fa_data_type),
        xml: Some(xml.to_string()),
    };
    Ok(encode_protobuf_message(OutgoingMessages::ReplaceFA as i32, &request.encode_to_vec()))
}

pub(in crate::accounts) fn encode_set_server_log_level(log_level: i32) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::SetServerLogLevelRequest { log_level: Some(log_level) };
    Ok(encode_protobuf_message(
        OutgoingMessages::ChangeServerLog as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::accounts) fn encode_verify_request(api_name: &str, api_version: &str) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::VerifyRequest {
        api_name: Some(api_name.to_string()),
        api_version: Some(api_version.to_string()),
    };
    Ok(encode_protobuf_message(OutgoingMessages::VerifyRequest as i32, &request.encode_to_vec()))
}

pub(in crate::accounts) fn encode_verify_message(api_data: &str) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::VerifyMessageRequest {
        api_data: Some(api_data.to_string()),
    };
    Ok(encode_protobuf_message(OutgoingMessages::VerifyMessage as i32, &request.encode_to_vec()))
}

#[cfg(test)]
#[path = "encoders_tests.rs"]
mod tests;
