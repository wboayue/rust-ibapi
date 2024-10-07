use std::sync::{Arc, Mutex, RwLock};

use crate::{server_versions, stubs::MessageBusStub, Client};

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

    let request_messages = client.message_bus.lock().expect("MessageBus is poisoned").request_messages();

    assert_eq!(request_messages[0].encode_simple(), "92|9000|DU1234567|TARGET2024|");
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

    let request_messages = client.message_bus.lock().expect("MessageBus is poisoned").request_messages();

    assert_eq!(request_messages[0].encode_simple(), "94|9000|DU1234567|TARGET2024|1001|");
}
