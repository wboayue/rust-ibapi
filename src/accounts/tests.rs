use std::sync::{Arc, Mutex, RwLock};

use crate::{accounts::pnl, server_versions, stubs::MessageBusStub, Client};

#[test]
fn test_pnl() {
    let message_bus = Arc::new(Mutex::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    }));

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let account = "xyzzx";
    let model_code = Some("A");

    let _responses = client.pnl(account, model_code).expect("msg");

    let request_messages = client.message_bus.lock().expect("MessageBus is poisoned").request_messages();

    assert_eq!(request_messages[0].encode_simple(), "92|9000|xyzzx|A|");

    // assert!(responses.is_ok(), "failed to place order: {}", results.err().unwrap());
}
