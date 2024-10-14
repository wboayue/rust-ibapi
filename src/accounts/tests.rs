use std::sync::{Arc, Mutex, RwLock};

use crate::testdata::responses;
use crate::{accounts::AccountSummaryTags, server_versions, stubs::MessageBusStub, Client};

#[test]
fn test_pnl() {
    let message_bus = Arc::new(Mutex::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    }));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let account = "DU1234567";
    let model_code = Some("TARGET2024");

    let _ = client.pnl(account, model_code).expect("request pnl failed");
    let _ = client.pnl(account, None).expect("request pnl failed");

    let request_messages = client.message_bus.read().unwrap().request_messages();

    assert_eq!(request_messages[0].encode_simple(), "92|9000|DU1234567|TARGET2024|");
    assert_eq!(request_messages[1].encode_simple(), "93|9000|");

    assert_eq!(request_messages[2].encode_simple(), "92|9001|DU1234567||");
    assert_eq!(request_messages[3].encode_simple(), "93|9001|");
}

#[test]
fn test_pnl_single() {
    let message_bus = Arc::new(Mutex::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    }));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let account = "DU1234567";
    let contract_id = 1001;
    let model_code = Some("TARGET2024");

    let _ = client.pnl_single(account, contract_id, model_code).expect("request pnl failed");
    let _ = client.pnl_single(account, contract_id, None).expect("request pnl failed");

    let request_messages = client.message_bus.lock().unwrap().request_messages();

    assert_eq!(request_messages[0].encode_simple(), "94|9000|DU1234567|TARGET2024|1001|");
    assert_eq!(request_messages[1].encode_simple(), "95|9000|");

    assert_eq!(request_messages[2].encode_simple(), "94|9001|DU1234567||1001|");
    assert_eq!(request_messages[3].encode_simple(), "95|9001|");
}

#[test]
fn test_positions() {
    let message_bus = Arc::new(Mutex::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    }));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let _ = client.positions().expect("request positions failed");

    let request_messages = client.message_bus.lock().unwrap().request_messages();

    assert_eq!(request_messages[0].encode_simple(), "61|1|");
    assert_eq!(request_messages[1].encode_simple(), "64|1|");
}

#[test]
fn test_positions_multi() {
    let message_bus = Arc::new(Mutex::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    }));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let account = Some("DU1234567");
    let model_code = Some("TARGET2024");

    let _ = client.positions_multi(account, model_code).expect("request positions failed");
    let _ = client.positions_multi(None, model_code).expect("request positions failed");

    let request_messages = client.message_bus.lock().unwrap().request_messages();

    assert_eq!(request_messages[0].encode_simple(), "74|1|9000|DU1234567|TARGET2024|");
    assert_eq!(request_messages[1].encode_simple(), "75|1|9000|");

    assert_eq!(request_messages[2].encode_simple(), "74|1|9001||TARGET2024|");
    assert_eq!(request_messages[3].encode_simple(), "75|1|9001|");
}

#[test]
fn test_account_summary() {
    let message_bus = Arc::new(Mutex::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    }));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let group = "All";
    let tags = &[AccountSummaryTags::ACCOUNT_TYPE];

    let _ = client.account_summary(group, tags).expect("request account summary failed");

    let request_messages = client.message_bus.lock().unwrap().request_messages();

    assert_eq!(request_messages[0].encode_simple(), "62|1|9000|All|AccountType|");
    assert_eq!(request_messages[1].encode_simple(), "64|1|");
}

#[test]
fn test_managed_accounts() {
    let message_bus = Arc::new(Mutex::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![responses::MANAGED_ACCOUNT.into()],
    }));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let accounts = client.managed_accounts().expect("request managed accounts failed");

    assert_eq!(accounts, &["DU1234567", "DU7654321"]);
}
