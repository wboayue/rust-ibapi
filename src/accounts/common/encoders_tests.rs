use crate::accounts::types::{AccountGroup, AccountId, ContractId};
use crate::accounts::{FaDataType, ServerLogLevel};
use crate::common::test_utils::helpers::assert_proto_msg_id;
use crate::messages::OutgoingMessages;
use crate::testdata::builders::accounts::{
    cancel_account_summary, cancel_account_updates_multi, cancel_pnl, cancel_pnl_single, request_current_time, request_current_time_in_millis,
    request_family_codes, request_managed_accounts,
};
use crate::testdata::builders::positions::{cancel_positions, cancel_positions_multi, request_positions};
use crate::testdata::builders::RequestEncoder;

#[test]
fn encode_request_positions_works() {
    let bytes = super::encode_request_positions().unwrap();
    assert_eq!(bytes, request_positions().encode_request());
}

#[test]
fn encode_cancel_positions_works() {
    let bytes = super::encode_cancel_positions().unwrap();
    assert_eq!(bytes, cancel_positions().encode_request());
}

#[test]
fn encode_request_account_summary_works() {
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
fn encode_cancel_account_summary_works() {
    let bytes = super::encode_cancel_account_summary(3000).unwrap();
    assert_eq!(bytes, cancel_account_summary().request_id(3000).encode_request());
}

#[test]
fn encode_request_pnl_works() {
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
fn encode_cancel_pnl_works() {
    let bytes = super::encode_cancel_pnl(123).unwrap();
    assert_eq!(bytes, cancel_pnl().request_id(123).encode_request());
}

#[test]
fn encode_request_pnl_single_works() {
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
fn encode_cancel_pnl_single_works() {
    let bytes = super::encode_cancel_pnl_single(456).unwrap();
    assert_eq!(bytes, cancel_pnl_single().request_id(456).encode_request());
}

#[test]
fn encode_request_positions_multi_works() {
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
fn encode_cancel_positions_multi_works() {
    let bytes = super::encode_cancel_positions_multi(9000).unwrap();
    assert_eq!(bytes, cancel_positions_multi().request_id(9000).encode_request());
}

#[test]
fn encode_request_account_updates_works() {
    let account = AccountId("DU123".to_string());
    let bytes = super::encode_request_account_updates(true, &account).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestAccountData);

    use prost::Message;
    let req = crate::proto::AccountDataRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.subscribe, Some(true));
    assert_eq!(req.acct_code.as_deref(), Some("DU123"));
}

#[test]
fn encode_cancel_account_updates_works() {
    let bytes = super::encode_cancel_account_updates().unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestAccountData);

    use prost::Message;
    let req = crate::proto::AccountDataRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.subscribe, Some(false));
    assert!(req.acct_code.is_none());
}

#[test]
fn encode_request_account_updates_multi_works() {
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
fn encode_cancel_account_updates_multi_works() {
    let bytes = super::encode_cancel_account_updates_multi(9000).unwrap();
    assert_eq!(bytes, cancel_account_updates_multi().request_id(9000).encode_request());
}

#[test]
fn encode_request_managed_accounts_works() {
    let bytes = super::encode_request_managed_accounts().unwrap();
    assert_eq!(bytes, request_managed_accounts().encode_request());
}

#[test]
fn encode_request_family_codes_works() {
    let bytes = super::encode_request_family_codes().unwrap();
    assert_eq!(bytes, request_family_codes().encode_request());
}

#[test]
fn encode_request_server_time_works() {
    let bytes = super::encode_request_server_time().unwrap();
    assert_eq!(bytes, request_current_time().encode_request());
}

#[test]
fn encode_request_server_time_millis_works() {
    let bytes = super::encode_request_server_time_millis().unwrap();
    assert_eq!(bytes, request_current_time_in_millis().encode_request());
}

#[test]
fn encode_request_soft_dollar_tiers_works() {
    let bytes = super::encode_request_soft_dollar_tiers(3000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestSoftDollarTiers);

    use prost::Message;
    let req = crate::proto::SoftDollarTiersRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(3000));
}

#[test]
fn encode_request_user_info_works() {
    let bytes = super::encode_request_user_info(4000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestUserInfo);

    use prost::Message;
    let req = crate::proto::UserInfoRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(4000));
}

#[test]
fn encode_request_fa_works() {
    let bytes = super::encode_request_fa(FaDataType::Groups).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestFA);

    use prost::Message;
    let req = crate::proto::FaRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.fa_data_type, Some(1));
}

#[test]
fn encode_replace_fa_works() {
    let bytes = super::encode_replace_fa(5000, FaDataType::AccountAliases, "<xml/>").unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::ReplaceFA);

    use prost::Message;
    let req = crate::proto::FaReplace::decode(&bytes[4..]).unwrap();
    assert_eq!(req.req_id, Some(5000));
    assert_eq!(req.fa_data_type, Some(3));
    assert_eq!(req.xml.as_deref(), Some("<xml/>"));
}

#[test]
fn encode_set_server_log_level_works() {
    let bytes = super::encode_set_server_log_level(ServerLogLevel::Info).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::ChangeServerLog);

    use prost::Message;
    let req = crate::proto::SetServerLogLevelRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.log_level, Some(4));
}

#[test]
fn encode_verify_request_works() {
    let bytes = super::encode_verify_request("TestApi", "1.0").unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::VerifyRequest);

    use prost::Message;
    let req = crate::proto::VerifyRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.api_name.as_deref(), Some("TestApi"));
    assert_eq!(req.api_version.as_deref(), Some("1.0"));
}

#[test]
fn encode_verify_message_works() {
    let bytes = super::encode_verify_message("challenge-data").unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::VerifyMessage);

    use prost::Message;
    let req = crate::proto::VerifyMessageRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.api_data.as_deref(), Some("challenge-data"));
}
