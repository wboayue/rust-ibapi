use super::constants::{VERSION_1, VERSION_2};
use crate::accounts::types::{AccountGroup, AccountId, ContractId, ModelCode};
use crate::messages::OutgoingMessages;
use crate::messages::RequestMessage;
use crate::Error;

pub(in crate::accounts) fn encode_request_positions() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::RequestPositions, VERSION_1)
}

pub(in crate::accounts) fn encode_cancel_positions() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::CancelPositions, VERSION_1)
}

pub(in crate::accounts) fn encode_cancel_account_summary(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::CancelAccountSummary);
    message.push_field(&VERSION_1);
    message.push_field(&request_id);
    Ok(message)
}

pub(in crate::accounts) fn encode_request_positions_multi(
    request_id: i32,
    account: Option<&AccountId>,
    model_code: Option<&ModelCode>,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestPositionsMulti);
    message.push_field(&VERSION_1);
    message.push_field(&request_id);
    message.push_field(&account.map(|a| a.as_ref()));
    message.push_field(&model_code.map(|m| m.as_ref()));
    Ok(message)
}

pub(in crate::accounts) fn encode_cancel_positions_multi(request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::CancelPositionsMulti);
    message.push_field(&VERSION_1);
    message.push_field(&request_id);
    Ok(message)
}

pub(in crate::accounts) fn encode_request_family_codes() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::RequestFamilyCodes, VERSION_1)
}

pub(in crate::accounts) fn encode_request_pnl(request_id: i32, account: &AccountId, model_code: Option<&ModelCode>) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestPnL);
    message.push_field(&request_id);
    message.push_field(&account.as_ref());
    message.push_field(&model_code.map(|m| m.as_ref()));
    Ok(message)
}

pub(in crate::accounts) fn encode_cancel_pnl(request_id: i32) -> Result<RequestMessage, Error> {
    encode_simple_with_request_id(OutgoingMessages::CancelPnL, request_id)
}

pub(in crate::accounts) fn encode_request_pnl_single(
    request_id: i32,
    account: &AccountId,
    contract_id: ContractId,
    model_code: Option<&ModelCode>,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestPnLSingle);
    message.push_field(&request_id);
    message.push_field(&account.as_ref());
    message.push_field(&model_code.map(|m| m.as_ref()));
    message.push_field(&contract_id.value());
    Ok(message)
}

pub(in crate::accounts) fn encode_cancel_pnl_single(request_id: i32) -> Result<RequestMessage, Error> {
    encode_simple_with_request_id(OutgoingMessages::CancelPnLSingle, request_id)
}

pub(in crate::accounts) fn encode_request_account_summary(request_id: i32, group: &AccountGroup, tags: &[&str]) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestAccountSummary);
    message.push_field(&VERSION_1);
    message.push_field(&request_id);
    message.push_field(&group.as_str());
    message.push_field(&tags.join(","));
    Ok(message)
}

pub(in crate::accounts) fn encode_request_managed_accounts() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::RequestManagedAccounts, VERSION_1)
}

pub(in crate::accounts) fn encode_request_account_updates(server_version: i32, account: &AccountId) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestAccountData);
    message.push_field(&VERSION_2);
    message.push_field(&true); // subscribe
    if server_version > 9 {
        message.push_field(&account.as_ref());
    }
    Ok(message)
}

pub(in crate::accounts) fn encode_request_account_updates_multi(
    request_id: i32,
    account: Option<&AccountId>,
    model_code: Option<&ModelCode>,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestAccountUpdatesMulti);
    message.push_field(&VERSION_1);
    message.push_field(&request_id);
    message.push_field(&account.map(|a| a.as_ref()));
    message.push_field(&model_code.map(|m| m.as_ref()));
    message.push_field(&true); // subscribe
    Ok(message)
}

pub(in crate::accounts) fn encode_cancel_account_updates(server_version: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::RequestAccountData);
    message.push_field(&VERSION_2);
    message.push_field(&false); // subscribe
    if server_version > 9 {
        message.push_field(&"");
    }
    Ok(message)
}

pub(in crate::accounts) fn encode_cancel_account_updates_multi(_server_version: i32, request_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();
    message.push_field(&OutgoingMessages::CancelAccountUpdatesMulti);
    message.push_field(&VERSION_1);
    message.push_field(&request_id);
    Ok(message)
}

pub(in crate::accounts) fn encode_request_server_time() -> Result<RequestMessage, Error> {
    encode_simple(OutgoingMessages::RequestCurrentTime, VERSION_1)
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
mod tests {
    use crate::accounts::types::{AccountGroup, AccountId, ContractId, ModelCode};
    use crate::{messages::OutgoingMessages, ToField};

    #[test]
    fn test_encode_request_positions() {
        let message = super::encode_request_positions().expect("error encoding request");

        assert_eq!(message[0], OutgoingMessages::RequestPositions.to_field());
        assert_eq!(message[1], "1");
    }

    #[test]
    fn test_encode_cancel_positions() {
        let message = super::encode_cancel_positions().expect("error encoding request");

        assert_eq!(message[0], OutgoingMessages::CancelPositions.to_field());
        assert_eq!(message[1], "1");
    }

    #[test]
    fn test_encode_request_positions_multi() {
        let request_id = 9000;
        let version = 1;
        let account = AccountId("U1234567".to_string());
        let model_code = ModelCode("TARGET2024".to_string());

        let message = super::encode_request_positions_multi(request_id, Some(&account), Some(&model_code)).expect("error encoding request");

        assert_eq!(message[0], OutgoingMessages::RequestPositionsMulti.to_field());
        assert_eq!(message[1], version.to_field());
        assert_eq!(message[2], request_id.to_field());
        assert_eq!(message[3], "U1234567");
        assert_eq!(message[4], "TARGET2024");
    }

    #[test]
    fn test_encode_request_positions_multi_options() {
        let request_id_base = 9000;
        let version = 1;
        struct TestCase {
            name: &'static str,
            account: Option<AccountId>,
            model_code: Option<ModelCode>,
            expected_account_field: &'static str,
            expected_model_field: &'static str,
        }
        let tests = [
            TestCase {
                name: "acc_none_model_some",
                account: None,
                model_code: Some(ModelCode("MODEL1".to_string())),
                expected_account_field: "",
                expected_model_field: "MODEL1",
            },
            TestCase {
                name: "acc_none_model_none",
                account: None,
                model_code: None,
                expected_account_field: "",
                expected_model_field: "",
            },
            // Optionally re-test existing case if not refactoring the original test
            TestCase {
                name: "acc_some_model_some",
                account: Some(AccountId("U123".to_string())),
                model_code: Some(ModelCode("MODEL2".to_string())),
                expected_account_field: "U123",
                expected_model_field: "MODEL2",
            },
        ];

        for (i, tc) in tests.iter().enumerate() {
            let request_id = request_id_base + i as i32;
            let message = super::encode_request_positions_multi(request_id, tc.account.as_ref(), tc.model_code.as_ref()).expect(tc.name);
            assert_eq!(message[0], OutgoingMessages::RequestPositionsMulti.to_field(), "Case: {} - type", tc.name);
            assert_eq!(message[1], version.to_field(), "Case: {} - version", tc.name);
            assert_eq!(message[2], request_id.to_field(), "Case: {} - request_id", tc.name);
            assert_eq!(message[3], tc.expected_account_field, "Case: {} - account", tc.name);
            assert_eq!(message[4], tc.expected_model_field, "Case: {} - model_code", tc.name);
        }
    }

    #[test]
    fn test_encode_cancel_positions_multi() {
        let request_id = 9000;
        let version = 1;

        let message = super::encode_cancel_positions_multi(request_id).expect("error encoding request");

        assert_eq!(message[0], OutgoingMessages::CancelPositionsMulti.to_field());
        assert_eq!(message[1], version.to_field());
        assert_eq!(message[2], request_id.to_field());
    }

    #[test]
    fn test_encode_request_family_codes() {
        let message = super::encode_request_family_codes().expect("error encoding request");

        assert_eq!(message[0], OutgoingMessages::RequestFamilyCodes.to_field());
        assert_eq!(message[1], "1");
    }

    #[test]
    fn test_encode_request_pnl() {
        let request_id = 3000;
        let account = AccountId("DU1234567".to_string());
        let model_code_none: Option<&ModelCode> = None;

        let request_no_model = super::encode_request_pnl(request_id, &account, model_code_none).expect("encode request pnl failed (no model)");

        assert_eq!(request_no_model[0], OutgoingMessages::RequestPnL.to_field(), "type (no model)");
        assert_eq!(request_no_model[1], request_id.to_field(), "request_id (no model)");
        assert_eq!(request_no_model[2], "DU1234567", "account (no model)");
        assert_eq!(request_no_model[3], "", "model_code (no model)");

        let request_id_with_model = 3001;
        let model_code_some = ModelCode("TestModelPnl".to_string());
        let request_with_model =
            super::encode_request_pnl(request_id_with_model, &account, Some(&model_code_some)).expect("encode request pnl failed (with model)");

        assert_eq!(request_with_model[0], OutgoingMessages::RequestPnL.to_field(), "type (with model)");
        assert_eq!(request_with_model[1], request_id_with_model.to_field(), "request_id (with model)");
        assert_eq!(request_with_model[2], "DU1234567", "account (with model)");
        assert_eq!(request_with_model[3], "TestModelPnl", "model_code (with model)");
    }

    #[test]
    fn test_encode_cancel_pnl() {
        let request_id = 123;
        let message = super::encode_cancel_pnl(request_id).expect("encoding failed");
        assert_eq!(message[0], OutgoingMessages::CancelPnL.to_field());
        assert_eq!(message[1], request_id.to_field());
    }

    #[test]
    fn test_encode_request_pnl_single() {
        let request_id = 3000;
        let account = AccountId("DU1234567".to_string());
        let model_code_none: Option<&ModelCode> = None;
        let contract_id = ContractId(1001);

        let request_no_model = super::encode_request_pnl_single(request_id, &account, contract_id, model_code_none)
            .expect("encode request pnl_single failed (no model)");

        assert_eq!(request_no_model[0], OutgoingMessages::RequestPnLSingle.to_field(), "type (no model)");
        assert_eq!(request_no_model[1], request_id.to_field(), "request_id (no model)");
        assert_eq!(request_no_model[2], "DU1234567", "account (no model)");
        assert_eq!(request_no_model[3], "", "model_code (no model)");
        assert_eq!(request_no_model[4], 1001.to_field(), "contract_id (no model)");

        let request_id_with_model = 3002;
        let account_with_model = AccountId("DU456".to_string());
        let contract_id_with_model = ContractId(1002);
        let model_code_some = ModelCode("MyModelPnlSingle".to_string());
        let request_with_model =
            super::encode_request_pnl_single(request_id_with_model, &account_with_model, contract_id_with_model, Some(&model_code_some))
                .expect("encode request pnl_single failed (with model)");

        assert_eq!(request_with_model[0], OutgoingMessages::RequestPnLSingle.to_field(), "type (with model)");
        assert_eq!(request_with_model[1], request_id_with_model.to_field(), "request_id (with model)");
        assert_eq!(request_with_model[2], "DU456", "account (with model)");
        assert_eq!(request_with_model[3], "MyModelPnlSingle", "model_code (with model)");
        assert_eq!(request_with_model[4], 1002.to_field(), "contract_id (with model)");
    }

    #[test]
    fn test_encode_cancel_pnl_single() {
        let request_id = 456;
        let message = super::encode_cancel_pnl_single(request_id).expect("encoding failed");
        assert_eq!(message[0], OutgoingMessages::CancelPnLSingle.to_field());
        assert_eq!(message[1], request_id.to_field());
    }

    #[test]
    fn test_encode_request_account_summary() {
        let version = 1;
        let request_id = 3000;
        let group = AccountGroup("All".to_string()); // Using the new AccountGroup struct
        let tags: &[&str] = &["AccountType", "TotalCashValue"];

        let request = super::encode_request_account_summary(request_id, &group, tags).expect("encode request account summary failed");

        assert_eq!(request[0], OutgoingMessages::RequestAccountSummary.to_field());
        assert_eq!(request[1], version.to_field());
        assert_eq!(request[2], request_id.to_field());
        assert_eq!(request[3], "All");
        assert_eq!(request[4], tags.join(","));
    }

    #[test]
    fn test_encode_request_managed_accounts() {
        let message = super::encode_request_managed_accounts().expect("encoding failed");
        assert_eq!(message[0], OutgoingMessages::RequestManagedAccounts.to_field());
        assert_eq!(message[1], "1"); // Version
    }

    #[test]
    fn test_encode_request_server_time() {
        let message = super::encode_request_server_time().expect("encoding failed");
        assert_eq!(message[0], OutgoingMessages::RequestCurrentTime.to_field());
        assert_eq!(message[1], "1"); // Version
    }

    #[test]
    fn test_encode_request_account_updates() {
        let server_version = 9;
        let version = 2;
        let account = AccountId("DU1234567".to_string());

        let request = super::encode_request_account_updates(server_version, &account).expect("encode request account updates");

        assert_eq!(request[0], OutgoingMessages::RequestAccountData.to_field());
        assert_eq!(request[1], version.to_field());
        assert_eq!(request[2], true.to_field());
        assert_eq!(request.len(), 3);

        let server_version_ge10 = 10;

        let request_sv_ge10 =
            super::encode_request_account_updates(server_version_ge10, &account).expect("encode request account updates for sv >= 10");

        assert_eq!(request_sv_ge10[0], OutgoingMessages::RequestAccountData.to_field());
        assert_eq!(request_sv_ge10[1], version.to_field());
        assert_eq!(request_sv_ge10[2], true.to_field());
        assert_eq!(request_sv_ge10[3], "DU1234567");
    }

    #[test]
    fn test_encode_cancel_account_updates() {
        let server_version = 9;
        let version = 2;

        let request = super::encode_cancel_account_updates(server_version).expect("encode cancel account updates");

        assert_eq!(request[0], OutgoingMessages::RequestAccountData.to_field());
        assert_eq!(request[1], version.to_field());
        assert_eq!(request[2], false.to_field());
        assert_eq!(request.len(), 3);

        let server_version_ge10 = 10;

        let request_sv_ge10 = super::encode_cancel_account_updates(server_version_ge10).expect("encode cancel account updates for sv >= 10");

        assert_eq!(request_sv_ge10[0], OutgoingMessages::RequestAccountData.to_field());
        assert_eq!(request_sv_ge10[1], version.to_field());
        assert_eq!(request_sv_ge10[2], false.to_field());
        assert_eq!(request_sv_ge10[3], "".to_string());
    }

    #[test]
    fn test_encode_request_account_updates_multi() {
        let request_id = 9000;
        let version = 1;
        let account = AccountId("DU1234567".to_string());
        let model_code = None;
        let subscribe = true;

        let request = super::encode_request_account_updates_multi(request_id, Some(&account), model_code).expect("encode request account updates");

        assert_eq!(request[0], OutgoingMessages::RequestAccountUpdatesMulti.to_field());
        assert_eq!(request[1], version.to_field());
        assert_eq!(request[2], request_id.to_field());
        assert_eq!(request[3], "DU1234567");
        assert_eq!(request[4], "");
        assert_eq!(request[5], subscribe.to_field());
    }

    #[test]
    fn test_encode_request_account_updates_multi_options() {
        let request_id_base = 9100;
        let version = 1;
        let subscribe = true;
        struct TestCase {
            name: &'static str,
            account: Option<AccountId>,
            model_code: Option<ModelCode>,
            expected_account_field: &'static str,
            expected_model_field: &'static str,
        }
        let tests = [
            // Existing case: Some("DU1234567"), None -> "DU1234567", ""
            TestCase {
                name: "acc_some_model_none_orig",
                account: Some(AccountId("DU1234567".to_string())),
                model_code: None,
                expected_account_field: "DU1234567",
                expected_model_field: "",
            },
            TestCase {
                name: "acc_none_model_some",
                account: None,
                model_code: Some(ModelCode("MODEL_X".to_string())),
                expected_account_field: "",
                expected_model_field: "MODEL_X",
            },
            TestCase {
                name: "acc_none_model_none",
                account: None,
                model_code: None,
                expected_account_field: "",
                expected_model_field: "",
            },
            TestCase {
                name: "acc_some_model_some",
                account: Some(AccountId("U789".to_string())),
                model_code: Some(ModelCode("MODEL_Y".to_string())),
                expected_account_field: "U789",
                expected_model_field: "MODEL_Y",
            },
        ];
        for (i, tc) in tests.iter().enumerate() {
            let request_id = request_id_base + i as i32;
            let message = super::encode_request_account_updates_multi(request_id, tc.account.as_ref(), tc.model_code.as_ref()).expect(tc.name);
            assert_eq!(
                message[0],
                OutgoingMessages::RequestAccountUpdatesMulti.to_field(),
                "Case: {} - type",
                tc.name
            );
            assert_eq!(message[1], version.to_field(), "Case: {} - version", tc.name);
            assert_eq!(message[2], request_id.to_field(), "Case: {} - request_id", tc.name);
            assert_eq!(message[3], tc.expected_account_field, "Case: {} - account", tc.name);
            assert_eq!(message[4], tc.expected_model_field, "Case: {} - model_code", tc.name);
            assert_eq!(message[5], subscribe.to_field(), "Case: {} - subscribe", tc.name);
        }
    }
}
