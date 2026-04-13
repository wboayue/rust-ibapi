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
    use prost::Message;
    let acct: &str = account;
    let request = crate::proto::AccountDataRequest {
        subscribe: if subscribe { Some(true) } else { None },
        acct_code: if acct.is_empty() { None } else { Some(acct.to_string()) },
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

#[cfg(test)]
mod tests {
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
}
