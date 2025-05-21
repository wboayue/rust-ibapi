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
    let account = "U1234567";
    let model_code = "TARGET2024";

    let message = super::encode_request_positions_multi(request_id, Some(account), Some(model_code)).expect("error encoding request");

    assert_eq!(message[0], OutgoingMessages::RequestPositionsMulti.to_field());
    assert_eq!(message[1], version.to_field());
    assert_eq!(message[2], request_id.to_field());
    assert_eq!(message[3], account);
    assert_eq!(message[4], model_code);
}

#[test]
fn test_encode_request_positions_multi_options() {
    let request_id_base = 9000;
    let version = 1;
    struct TestCase {
        name: &'static str,
        account: Option<&'static str>,
        model_code: Option<&'static str>,
        expected_account_field: &'static str,
        expected_model_field: &'static str,
    }
    let tests = [
        TestCase {
            name: "acc_none_model_some",
            account: None,
            model_code: Some("MODEL1"),
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
            account: Some("U123"),
            model_code: Some("MODEL2"),
            expected_account_field: "U123",
            expected_model_field: "MODEL2",
        },
    ];

    for (i, tc) in tests.iter().enumerate() {
        let request_id = request_id_base + i as i32;
        let message = super::encode_request_positions_multi(request_id, tc.account, tc.model_code).expect(tc.name);
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
    let account = "DU1234567";
    let model_code_none: Option<&str> = None;

    let request_no_model = super::encode_request_pnl(request_id, &account, model_code_none).expect("encode request pnl failed (no model)");

    assert_eq!(request_no_model[0], OutgoingMessages::RequestPnL.to_field(), "type (no model)");
    assert_eq!(request_no_model[1], request_id.to_field(), "request_id (no model)");
    assert_eq!(request_no_model[2], account, "account (no model)");
    assert_eq!(request_no_model[3], "", "model_code (no model)");

    let request_id_with_model = 3001;
    let model_code_some = Some("TestModelPnl");
    let request_with_model =
        super::encode_request_pnl(request_id_with_model, &account, model_code_some).expect("encode request pnl failed (with model)");

    assert_eq!(request_with_model[0], OutgoingMessages::RequestPnL.to_field(), "type (with model)");
    assert_eq!(request_with_model[1], request_id_with_model.to_field(), "request_id (with model)");
    assert_eq!(request_with_model[2], account, "account (with model)");
    assert_eq!(request_with_model[3], model_code_some.unwrap(), "model_code (with model)");
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
    let account = "DU1234567";
    let model_code_none: Option<&str> = None;
    let contract_id = 1001;

    let request_no_model =
        super::encode_request_pnl_single(request_id, &account, contract_id, model_code_none).expect("encode request pnl_single failed (no model)");

    assert_eq!(request_no_model[0], OutgoingMessages::RequestPnLSingle.to_field(), "type (no model)");
    assert_eq!(request_no_model[1], request_id.to_field(), "request_id (no model)");
    assert_eq!(request_no_model[2], account, "account (no model)");
    assert_eq!(request_no_model[3], "", "model_code (no model)");
    assert_eq!(request_no_model[4], contract_id.to_field(), "contract_id (no model)");

    let request_id_with_model = 3002;
    let account_with_model = "DU456";
    let contract_id_with_model = 1002;
    let model_code_some = Some("MyModelPnlSingle");
    let request_with_model = super::encode_request_pnl_single(request_id_with_model, &account_with_model, contract_id_with_model, model_code_some)
        .expect("encode request pnl_single failed (with model)");

    assert_eq!(request_with_model[0], OutgoingMessages::RequestPnLSingle.to_field(), "type (with model)");
    assert_eq!(request_with_model[1], request_id_with_model.to_field(), "request_id (with model)");
    assert_eq!(request_with_model[2], account_with_model, "account (with model)");
    assert_eq!(request_with_model[3], model_code_some.unwrap(), "model_code (with model)");
    assert_eq!(request_with_model[4], contract_id_with_model.to_field(), "contract_id (with model)");
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
    let group = "All";
    let tags: &[&str] = &["AccountType", "TotalCashValue"];

    let request = super::encode_request_account_summary(request_id, group, tags).expect("encode request account summary failed");

    assert_eq!(request[0], OutgoingMessages::RequestAccountSummary.to_field());
    assert_eq!(request[1], version.to_field());
    assert_eq!(request[2], request_id.to_field());
    assert_eq!(request[3], group.to_string());
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
    let account = "DU1234567";

    let request = super::encode_request_account_updates(server_version, &account).expect("encode request account updates");

    assert_eq!(request[0], OutgoingMessages::RequestAccountData.to_field());
    assert_eq!(request[1], version.to_field());
    assert_eq!(request[2], true.to_field());
    // For server_version < ACCOUNT_SUMMARY (which is 10), account is not sent.
    assert_eq!(request[3], "");

    let server_version_ge10 = 10;

    let request_sv_ge10 = super::encode_request_account_updates(server_version_ge10, &account).expect("encode request account updates for sv >= 10");

    assert_eq!(request_sv_ge10[0], OutgoingMessages::RequestAccountData.to_field());
    assert_eq!(request_sv_ge10[1], version.to_field());
    assert_eq!(request_sv_ge10[2], true.to_field());
    assert_eq!(request_sv_ge10[3], account.to_string());
}

#[test]
fn test_encode_cancel_account_updates() {
    let server_version = 9;
    let version = 2;

    let request = super::encode_cancel_account_updates(server_version).expect("encode cancel account updates");

    assert_eq!(request[0], OutgoingMessages::RequestAccountData.to_field());
    assert_eq!(request[1], version.to_field());
    assert_eq!(request[2], false.to_field());
    assert_eq!(request[3], "");

    let server_version_ge10 = 10;
    let account_empty = ""; // For cancel, account is empty string if server_version >= 10

    let request_sv_ge10 = super::encode_cancel_account_updates(server_version_ge10).expect("encode cancel account updates for sv >= 10");

    assert_eq!(request_sv_ge10[0], OutgoingMessages::RequestAccountData.to_field());
    assert_eq!(request_sv_ge10[1], version.to_field());
    assert_eq!(request_sv_ge10[2], false.to_field());
    assert_eq!(request_sv_ge10[3], account_empty.to_string());
}

#[test]
fn test_encode_request_account_updates_multi() {
    let request_id = 9000;
    let version = 1;
    let account = "DU1234567";
    let model_code = None;
    let subscribe = true;

    let request = super::encode_request_account_updates_multi(request_id, Some(&account), model_code).expect("encode request account updates");

    assert_eq!(request[0], OutgoingMessages::RequestAccountUpdatesMulti.to_field());
    assert_eq!(request[1], version.to_field());
    assert_eq!(request[2], request_id.to_field());
    assert_eq!(request[3], account.to_string());
    assert_eq!(request[4], model_code.to_field());
    assert_eq!(request[5], subscribe.to_field());
}

#[test]
fn test_encode_request_account_updates_multi_options() {
    let request_id_base = 9100;
    let version = 1;
    let subscribe = true;
    struct TestCase {
        name: &'static str,
        account: Option<&'static str>,
        model_code: Option<&'static str>,
        expected_account_field: &'static str,
        expected_model_field: &'static str,
    }
    let tests = [
        // Existing case: Some("DU1234567"), None -> "DU1234567", ""
        TestCase {
            name: "acc_some_model_none_orig",
            account: Some("DU1234567"),
            model_code: None,
            expected_account_field: "DU1234567",
            expected_model_field: "",
        },
        TestCase {
            name: "acc_none_model_some",
            account: None,
            model_code: Some("MODEL_X"),
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
            account: Some("U789"),
            model_code: Some("MODEL_Y"),
            expected_account_field: "U789",
            expected_model_field: "MODEL_Y",
        },
    ];
    for (i, tc) in tests.iter().enumerate() {
        let request_id = request_id_base + i as i32;
        let message = super::encode_request_account_updates_multi(request_id, tc.account, tc.model_code).expect(tc.name);
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
