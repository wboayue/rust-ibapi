use super::*;
use crate::messages::{encode_protobuf_message, OutgoingMessages, ResponseMessage};
use crate::stubs::MessageBusStub;
use std::sync::Arc;

#[derive(Debug)]
struct EndOfStreamItem;

impl StreamDecoder<EndOfStreamItem> for EndOfStreamItem {
    fn decode(_context: &DecoderContext, _msg: &mut ResponseMessage) -> Result<EndOfStreamItem, Error> {
        Err(Error::EndOfStream)
    }

    fn cancel_message(_server_version: i32, _id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        Ok(encode_protobuf_message(OutgoingMessages::CancelMarketData as i32, &[]))
    }
}

#[test]
fn test_subscription_skips_unexpected_messages_without_limit() {
    use std::sync::atomic::AtomicUsize;

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug)]
    struct SkipThenSuccess;

    impl StreamDecoder<SkipThenSuccess> for SkipThenSuccess {
        fn decode(_context: &DecoderContext, _msg: &mut ResponseMessage) -> Result<SkipThenSuccess, Error> {
            let n = CALL_COUNT.fetch_add(1, Ordering::Relaxed);
            if n < 20 {
                Err(Error::UnexpectedResponse(ResponseMessage::from("stray\0")))
            } else {
                Ok(SkipThenSuccess)
            }
        }
    }

    CALL_COUNT.store(0, Ordering::Relaxed);

    // 20 stray messages + 1 valid (more than the old MAX_DECODE_RETRIES=10)
    let mut responses: Vec<String> = (0..21).map(|_| "1|msg".to_string()).collect();
    // Sentinel to avoid blocking on the channel after success
    responses.push("1|done".to_string());

    let stub = MessageBusStub::with_responses(responses);
    let message_bus = Arc::new(stub);

    let sub: Subscription<SkipThenSuccess> = {
        let internal = message_bus.send_request(1, &[]).unwrap();
        Subscription::new(message_bus.clone(), internal, DecoderContext::default())
    };

    let result = sub.next();
    assert!(result.is_some(), "subscription should survive 20 skips and return valid message");
    assert_eq!(CALL_COUNT.load(Ordering::Relaxed), 21);
}

#[test]
fn test_routed_item_error_terminates_subscription() {
    use crate::subscriptions::common::RoutedItem;
    use crate::transport::SubscriptionBuilder;
    use crossbeam::channel;

    #[derive(Debug)]
    struct DataItem;

    impl StreamDecoder<DataItem> for DataItem {
        fn decode(_context: &DecoderContext, _msg: &mut ResponseMessage) -> Result<DataItem, Error> {
            Ok(DataItem)
        }
    }

    let (sender, receiver) = channel::unbounded::<RoutedItem>();
    let (signaler, _) = channel::unbounded();
    sender.send(RoutedItem::Error(Error::ConnectionReset)).unwrap();

    let internal = SubscriptionBuilder::new().receiver(receiver).signaler(signaler).request_id(1).build();

    let stub = Arc::new(MessageBusStub::default());
    let sub: Subscription<DataItem> = Subscription::new(stub, internal, DecoderContext::default());

    // First call surfaces the terminal error via the Err arm.
    assert!(matches!(sub.next(), Some(Err(Error::ConnectionReset))));
    // Subsequent calls return None — the stream is terminated.
    assert!(sub.next().is_none());
}

#[test]
fn test_no_retries_after_end_of_stream() {
    let stub = MessageBusStub::with_responses(vec![
        "1|data".to_string(),  // triggers EndOfStream via decoder
        "1|stray".to_string(), // stray message after stream ended
    ]);
    let message_bus = Arc::new(stub);

    let sub: Subscription<EndOfStreamItem> = {
        let internal = message_bus.send_request(1, &[]).unwrap();
        Subscription::new(message_bus.clone(), internal, DecoderContext::default())
    };

    // First call hits EndOfStream, returns None
    assert!(sub.next().is_none());

    // Second call should return None immediately (stream_ended guard)
    assert!(sub.next().is_none());
    assert!(sub.stream_ended.load(Ordering::Relaxed));
}
