use crate::accounts::types::{AccountGroup, AccountId, ContractId};
use crate::common::test_utils::helpers::assert_proto_msg_id;
use crate::messages::OutgoingMessages;

#[test]
fn test_encode_request_positions() {
    let bytes = super::encode_request_positions().unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestPositions);
}

#[test]
fn test_encode_cancel_positions() {
    let bytes = super::encode_cancel_positions().unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::CancelPositions);
}

#[test]
fn test_encode_request_account_summary() {
    let group = AccountGroup("All".to_string());
    let bytes = super::encode_request_account_summary(3000, &group, &["AccountType", "NetLiquidation"]).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestAccountSummary);

    use prost::Message;
    let req = crate::proto::AccountSummaryRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(3000));
    assert_eq!(req.group.as_deref(), Some("All"));
    assert_eq!(req.tags.as_deref(), Some("AccountType,NetLiquidation"));
}

#[test]
fn test_encode_cancel_account_summary() {
    let bytes = super::encode_cancel_account_summary(3000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::CancelAccountSummary);
}

#[test]
fn test_encode_request_pnl() {
    use crate::accounts::types::ModelCode;

    let account = AccountId("DU123".to_string());
    let model = ModelCode("MyModel".to_string());
    let bytes = super::encode_request_pnl(3000, &account, Some(&model)).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestPnL);

    use prost::Message;
    let req = crate::proto::PnLRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(3000));
    assert_eq!(req.account.as_deref(), Some("DU123"));
    assert_eq!(req.model_code.as_deref(), Some("MyModel"));
}

#[test]
fn test_encode_cancel_pnl() {
    let bytes = super::encode_cancel_pnl(123).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::CancelPnL);
}

#[test]
fn test_encode_request_pnl_single() {
    let account = AccountId("DU123".to_string());
    let cid = ContractId(1001);
    let bytes = super::encode_request_pnl_single(3000, &account, cid, None).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestPnLSingle);

    use prost::Message;
    let req = crate::proto::PnLSingleRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(3000));
    assert_eq!(req.account.as_deref(), Some("DU123"));
    assert_eq!(req.con_id, Some(1001));
    assert!(req.model_code.is_none());
}

#[test]
fn test_encode_cancel_pnl_single() {
    let bytes = super::encode_cancel_pnl_single(456).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::CancelPnLSingle);
}

#[test]
fn test_encode_request_positions_multi() {
    use crate::accounts::types::ModelCode;

    let account = AccountId("U1234567".to_string());
    let model_code = ModelCode("TARGET2024".to_string());
    let bytes = super::encode_request_positions_multi(9000, Some(&account), Some(&model_code)).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestPositionsMulti);

    use prost::Message;
    let req = crate::proto::PositionsMultiRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(9000));
    assert_eq!(req.account.as_deref(), Some("U1234567"));
    assert_eq!(req.model_code.as_deref(), Some("TARGET2024"));
}

#[test]
fn test_encode_cancel_positions_multi() {
    let bytes = super::encode_cancel_positions_multi(9000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::CancelPositionsMulti);
}

#[test]
fn test_encode_request_account_updates() {
    let account = AccountId("DU123".to_string());
    let bytes = super::encode_request_account_updates(true, &account).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestAccountData);

    use prost::Message;
    let req = crate::proto::AccountDataRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.subscribe, Some(true));
    assert_eq!(req.acct_code.as_deref(), Some("DU123"));
}

#[test]
fn test_encode_cancel_account_updates() {
    let bytes = super::encode_cancel_account_updates().unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestAccountData);

    use prost::Message;
    let req = crate::proto::AccountDataRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.subscribe, Some(false));
    assert!(req.acct_code.is_none());
}

#[test]
fn test_encode_request_account_updates_multi() {
    use crate::accounts::types::ModelCode;

    let account = AccountId("DU1234567".to_string());
    let model_code = ModelCode("MODEL_X".to_string());
    let bytes = super::encode_request_account_updates_multi(9000, Some(&account), Some(&model_code)).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestAccountUpdatesMulti);

    use prost::Message;
    let req = crate::proto::AccountUpdatesMultiRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(9000));
    assert_eq!(req.account.as_deref(), Some("DU1234567"));
    assert_eq!(req.model_code.as_deref(), Some("MODEL_X"));
    assert_eq!(req.ledger_and_nlv, Some(true));
}

#[test]
fn test_encode_cancel_account_updates_multi() {
    let bytes = super::encode_cancel_account_updates_multi(9000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::CancelAccountUpdatesMulti);
}

#[test]
fn test_encode_request_managed_accounts() {
    let bytes = super::encode_request_managed_accounts().unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestManagedAccounts);
}

#[test]
fn test_encode_request_family_codes() {
    let bytes = super::encode_request_family_codes().unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestFamilyCodes);
}

#[test]
fn test_encode_request_server_time() {
    let bytes = super::encode_request_server_time().unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestCurrentTime);
}

#[test]
fn test_encode_request_server_time_millis() {
    let bytes = super::encode_request_server_time_millis().unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestCurrentTimeInMillis);
}

#[test]
fn test_encode_request_soft_dollar_tiers() {
    let bytes = super::encode_request_soft_dollar_tiers(3000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestSoftDollarTiers);

    use prost::Message;
    let req = crate::proto::SoftDollarTiersRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(3000));
}

#[test]
fn test_encode_request_user_info() {
    let bytes = super::encode_request_user_info(4000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestUserInfo);

    use prost::Message;
    let req = crate::proto::UserInfoRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(4000));
}

#[test]
fn test_encode_request_fa() {
    let bytes = super::encode_request_fa(1).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestFA);

    use prost::Message;
    let req = crate::proto::FaRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.fa_data_type, Some(1));
}

#[test]
fn test_encode_replace_fa() {
    let bytes = super::encode_replace_fa(5000, 3, "<xml/>").unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::ReplaceFA);

    use prost::Message;
    let req = crate::proto::FaReplace::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(5000));
    assert_eq!(req.fa_data_type, Some(3));
    assert_eq!(req.xml.as_deref(), Some("<xml/>"));
}

#[test]
fn test_encode_set_server_log_level() {
    let bytes = super::encode_set_server_log_level(4).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::ChangeServerLog);

    use prost::Message;
    let req = crate::proto::SetServerLogLevelRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.log_level, Some(4));
}

#[test]
fn test_encode_verify_request() {
    let bytes = super::encode_verify_request("TestApi", "1.0").unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::VerifyRequest);

    use prost::Message;
    let req = crate::proto::VerifyRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.api_name.as_deref(), Some("TestApi"));
    assert_eq!(req.api_version.as_deref(), Some("1.0"));
}

#[test]
fn test_encode_verify_message() {
    let bytes = super::encode_verify_message("challenge-data").unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::VerifyMessage);

    use prost::Message;
    let req = crate::proto::VerifyMessageRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.api_data.as_deref(), Some("challenge-data"));
}
