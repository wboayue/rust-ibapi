use super::*;

use crate::common::test_utils::helpers::text_response;
use crate::messages::ResponseMessage;

fn sample_response_messages() -> Vec<String> {
    vec!["1|9001|".to_string(), "2|9002|".to_string()]
}

#[test]
fn default_creates_empty_stub() {
    let stub = MessageBusStub::default();
    assert!(stub.request_messages().is_empty());
    assert!(stub.response_messages.is_empty());
    assert!(stub.ordered_responses.is_empty());
    assert!(stub.response_messages_decoded().is_empty());
}

#[test]
fn with_responses_stores_text_payloads() {
    let stub = MessageBusStub::with_responses(sample_response_messages());
    assert_eq!(stub.response_messages, sample_response_messages());
    assert!(stub.ordered_responses.is_empty());
}

#[test]
fn with_ordered_responses_stores_messages() {
    let messages = vec![text_response("1|9001|"), text_response("2|9002|")];
    let stub = MessageBusStub::with_ordered_responses(messages.clone());
    assert!(stub.response_messages.is_empty());
    assert_eq!(stub.ordered_responses.len(), messages.len());
}

#[test]
fn request_messages_returns_clone() {
    let stub = MessageBusStub::default();
    stub.request_messages.write().unwrap().push(b"hello".to_vec());

    let snapshot = stub.request_messages();
    assert_eq!(snapshot, vec![b"hello".to_vec()]);

    // Mutating the snapshot does not mutate the stub state.
    let mut snapshot = snapshot;
    snapshot.clear();
    assert_eq!(stub.request_messages(), vec![b"hello".to_vec()]);
}

#[test]
fn response_messages_decoded_translates_pipe_to_nul() {
    let stub = MessageBusStub::with_responses(vec!["1|9001|payload".to_string()]);
    let decoded = stub.response_messages_decoded();
    assert_eq!(decoded.len(), 1);
    let mut msg = decoded.into_iter().next().unwrap();
    assert_eq!(msg.next_string().unwrap(), "1");
    assert_eq!(msg.next_string().unwrap(), "9001");
    assert_eq!(msg.next_string().unwrap(), "payload");
}

#[test]
fn response_messages_decoded_prefers_ordered_responses() {
    let stub = MessageBusStub {
        request_messages: std::sync::RwLock::new(vec![]),
        // text payloads are deliberately ignored when ordered_responses is non-empty
        response_messages: vec!["should-not-decode".to_string()],
        ordered_responses: vec![text_response("X|1|")],
    };
    let decoded = stub.response_messages_decoded();
    assert_eq!(decoded.len(), 1);
    let mut msg = decoded.into_iter().next().unwrap();
    assert_eq!(msg.next_string().unwrap(), "X");
}

#[cfg(feature = "sync")]
mod sync_tests {
    use super::*;
    use crate::transport::MessageBus;

    fn drain_subscription(sub: &crate::transport::InternalSubscription) -> Vec<ResponseMessage> {
        let mut out = Vec::new();
        while let Some(item) = sub.try_next() {
            match item {
                Ok(message) => out.push(message),
                Err(e) => panic!("unexpected error in stub stream: {e:?}"),
            }
        }
        out
    }

    #[test]
    fn send_request_captures_message_and_yields_responses() {
        let stub = MessageBusStub::with_responses(vec!["1|9001|hi".to_string()]);
        let sub = MessageBus::send_request(&stub, 42, b"req-bytes").expect("send_request");

        assert_eq!(stub.request_messages(), vec![b"req-bytes".to_vec()]);
        assert_eq!(sub.request_id, Some(42));
        assert_eq!(drain_subscription(&sub).len(), 1);
    }

    #[test]
    fn send_order_request_captures_and_yields() {
        let stub = MessageBusStub::with_responses(vec!["1|9001|".to_string()]);
        let sub = MessageBus::send_order_request(&stub, 7, b"order-bytes").expect("send_order_request");

        assert_eq!(stub.request_messages(), vec![b"order-bytes".to_vec()]);
        assert_eq!(sub.request_id, Some(7));
        assert_eq!(drain_subscription(&sub).len(), 1);
    }

    #[test]
    fn send_message_captures_only() {
        let stub = MessageBusStub::default();
        MessageBus::send_message(&stub, b"fire-and-forget").expect("send_message");
        assert_eq!(stub.request_messages(), vec![b"fire-and-forget".to_vec()]);
    }

    #[test]
    fn cancel_subscription_captures_packet() {
        let stub = MessageBusStub::default();
        MessageBus::cancel_subscription(&stub, 99, b"cancel-bytes").expect("cancel_subscription");
        assert_eq!(stub.request_messages(), vec![b"cancel-bytes".to_vec()]);
    }

    #[test]
    fn send_shared_request_uses_shared_receiver() {
        let stub = MessageBusStub::with_responses(vec!["1|9001|".to_string()]);
        let sub = MessageBus::send_shared_request(&stub, OutgoingMessages::RequestMarketData, b"shared-bytes").expect("send_shared_request");

        assert_eq!(stub.request_messages(), vec![b"shared-bytes".to_vec()]);
        assert_eq!(sub.request_id, None);
        assert_eq!(sub.message_type, Some(OutgoingMessages::RequestMarketData));
        assert_eq!(drain_subscription(&sub).len(), 1);
    }

    #[test]
    fn cancel_shared_subscription_captures_packet() {
        let stub = MessageBusStub::default();
        MessageBus::cancel_shared_subscription(&stub, OutgoingMessages::RequestMarketData, b"cancel-shared").expect("cancel_shared_subscription");
        assert_eq!(stub.request_messages(), vec![b"cancel-shared".to_vec()]);
    }

    #[test]
    fn create_order_update_subscription_then_already_subscribed() {
        let stub = MessageBusStub::with_responses(vec!["1|9001|".to_string()]);
        let _sub = MessageBus::create_order_update_subscription(&stub).expect("first subscribe");

        match MessageBus::create_order_update_subscription(&stub) {
            Err(Error::AlreadySubscribed) => {}
            other => panic!("expected AlreadySubscribed, got {other:?}"),
        }
    }

    #[test]
    fn cancel_order_subscription_releases_tracker() {
        let stub = MessageBusStub::default();
        let _first = MessageBus::create_order_update_subscription(&stub).expect("subscribe");

        MessageBus::cancel_order_subscription(&stub, 0, b"cancel-order").expect("cancel_order_subscription");
        assert_eq!(stub.request_messages(), vec![b"cancel-order".to_vec()]);

        let _second = MessageBus::create_order_update_subscription(&stub).expect("re-subscribe after cancel");
    }

    #[test]
    fn notice_subscribe_returns_closed_stream() {
        let stub = MessageBusStub::default();
        let stream = MessageBus::notice_subscribe(&stub);
        // Sender end is dropped immediately, so the receiver is at end-of-stream.
        assert!(stream.next().is_none());
    }

    #[test]
    fn ensure_shutdown_is_noop_and_is_connected_returns_true() {
        let stub = MessageBusStub::default();
        MessageBus::ensure_shutdown(&stub);
        assert!(MessageBus::is_connected(&stub));
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use crate::transport::AsyncMessageBus;

    // The stub drops the broadcast sender before returning from each `send_*`
    // call, so once preloaded messages drain, `next()` returns `None` cleanly —
    // no timeout needed.
    async fn drain_async(sub: &mut crate::transport::AsyncInternalSubscription) -> Vec<ResponseMessage> {
        let mut out = Vec::new();
        while let Some(result) = sub.next().await {
            match result {
                Ok(message) => out.push(message),
                Err(e) => panic!("unexpected error in stub stream: {e:?}"),
            }
        }
        out
    }

    #[tokio::test]
    async fn send_request_captures_and_yields() {
        let stub = MessageBusStub::with_responses(vec!["1|9001|".to_string()]);
        let mut sub = AsyncMessageBus::send_request(&stub, 1, b"req".to_vec()).await.expect("send_request");

        assert_eq!(stub.request_messages(), vec![b"req".to_vec()]);
        assert_eq!(drain_async(&mut sub).await.len(), 1);
    }

    #[tokio::test]
    async fn send_order_request_captures_and_yields() {
        let stub = MessageBusStub::with_responses(vec!["1|9001|".to_string()]);
        let mut sub = AsyncMessageBus::send_order_request(&stub, 2, b"order".to_vec())
            .await
            .expect("send_order_request");

        assert_eq!(stub.request_messages(), vec![b"order".to_vec()]);
        assert_eq!(drain_async(&mut sub).await.len(), 1);
    }

    #[tokio::test]
    async fn send_shared_request_captures_and_yields() {
        let stub = MessageBusStub::with_responses(vec!["1|9001|".to_string()]);
        let mut sub = AsyncMessageBus::send_shared_request(&stub, OutgoingMessages::RequestMarketData, b"shared".to_vec())
            .await
            .expect("send_shared_request");

        assert_eq!(stub.request_messages(), vec![b"shared".to_vec()]);
        assert_eq!(drain_async(&mut sub).await.len(), 1);
    }

    #[tokio::test]
    async fn send_message_captures_only() {
        let stub = MessageBusStub::default();
        AsyncMessageBus::send_message(&stub, b"async-fire".to_vec()).await.expect("send_message");
        assert_eq!(stub.request_messages(), vec![b"async-fire".to_vec()]);
    }

    #[tokio::test]
    async fn cancel_subscription_captures_message() {
        let stub = MessageBusStub::default();
        AsyncMessageBus::cancel_subscription(&stub, 1, b"async-cancel".to_vec())
            .await
            .expect("cancel_subscription");
        assert_eq!(stub.request_messages(), vec![b"async-cancel".to_vec()]);
    }

    #[tokio::test]
    async fn cancel_order_subscription_is_noop() {
        let stub = MessageBusStub::default();
        AsyncMessageBus::cancel_order_subscription(&stub, 7, b"ignored".to_vec())
            .await
            .expect("cancel_order_subscription");
        assert!(stub.request_messages().is_empty());
    }

    #[tokio::test]
    async fn create_order_update_subscription_then_already_subscribed_and_cleanup() {
        let stub = MessageBusStub::default();
        let stub_id = &stub as *const _ as usize;

        let sub = AsyncMessageBus::create_order_update_subscription(&stub).await.expect("first subscribe");
        assert!(super::ORDER_UPDATE_SUBSCRIPTION_TRACKER.lock().unwrap().contains(&stub_id));

        match AsyncMessageBus::create_order_update_subscription(&stub).await {
            Err(Error::AlreadySubscribed) => {}
            Err(e) => panic!("expected AlreadySubscribed, got error {e:?}"),
            Ok(_) => panic!("expected AlreadySubscribed, got Ok"),
        }

        // Drop sends CleanupSignal::OrderUpdateStream; yield until the spawned
        // task receives it and clears the tracker entry. Bounded with a short
        // timeout so a regression hangs the test rather than the suite.
        drop(sub);
        tokio::time::timeout(std::time::Duration::from_millis(500), async {
            while super::ORDER_UPDATE_SUBSCRIPTION_TRACKER.lock().unwrap().contains(&stub_id) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cleanup task did not clear tracker within 500ms");

        let _again = AsyncMessageBus::create_order_update_subscription(&stub)
            .await
            .expect("re-subscribe after cleanup");
    }

    #[tokio::test]
    async fn notice_subscribe_returns_closed_stream() {
        let stub = MessageBusStub::default();
        let mut stream = AsyncMessageBus::notice_subscribe(&stub);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn ensure_shutdown_request_shutdown_sync_and_is_connected() {
        let stub = MessageBusStub::default();
        AsyncMessageBus::ensure_shutdown(&stub).await;
        AsyncMessageBus::request_shutdown_sync(&stub);
        assert!(AsyncMessageBus::is_connected(&stub));
    }
}
