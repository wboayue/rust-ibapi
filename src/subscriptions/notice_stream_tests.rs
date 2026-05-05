//! Unit tests for the sync and async `NoticeStream` impls.

use crate::messages::Notice;

fn make_notice(code: i32, message: &str) -> Notice {
    Notice {
        code,
        message: message.into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    }
}

#[cfg(feature = "sync")]
mod sync_tests {
    use super::*;
    use crate::subscriptions::notice_stream::sync_impl::NoticeStream;
    use crossbeam::channel;
    use std::time::Duration;

    #[test]
    fn next_returns_pushed_notice() {
        let (sender, receiver) = channel::unbounded();
        let stream = NoticeStream::new(receiver);

        sender.send(make_notice(2104, "farm OK")).unwrap();
        let notice = stream.next().expect("pending notice not received");
        assert_eq!(notice.code, 2104);
        assert_eq!(notice.message, "farm OK");
    }

    #[test]
    fn try_next_is_non_blocking() {
        let (sender, receiver) = channel::unbounded();
        let stream = NoticeStream::new(receiver);

        assert!(stream.try_next().is_none(), "empty channel should yield None");

        sender.send(make_notice(1100, "lost")).unwrap();
        assert_eq!(stream.try_next().expect("notice").code, 1100);
    }

    #[test]
    fn next_timeout_returns_none_when_idle() {
        let (_sender, receiver) = channel::unbounded::<Notice>();
        let stream = NoticeStream::new(receiver);

        let start = std::time::Instant::now();
        assert!(stream.next_timeout(Duration::from_millis(50)).is_none());
        assert!(start.elapsed() >= Duration::from_millis(45));
    }

    #[test]
    fn next_returns_none_when_sender_dropped() {
        let (sender, receiver) = channel::unbounded::<Notice>();
        let stream = NoticeStream::new(receiver);
        drop(sender);
        assert!(stream.next().is_none());
    }

    #[test]
    fn iter_yields_buffered_notices_in_order() {
        let (sender, receiver) = channel::unbounded();
        let stream = NoticeStream::new(receiver);

        sender.send(make_notice(2104, "a")).unwrap();
        sender.send(make_notice(2107, "b")).unwrap();
        drop(sender);

        let codes: Vec<i32> = stream.iter().map(|n| n.code).collect();
        assert_eq!(codes, vec![2104, 2107]);
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use crate::subscriptions::notice_stream::async_impl::NoticeStream;
    use futures::StreamExt;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn next_returns_pushed_notice() {
        let (sender, receiver) = broadcast::channel(8);
        let mut stream = NoticeStream::new(receiver);

        sender.send(make_notice(2104, "farm OK")).unwrap();
        let notice = stream.next().await.expect("pending notice not received");
        assert_eq!(notice.code, 2104);
    }

    #[tokio::test]
    async fn next_returns_none_when_sender_dropped() {
        let (sender, receiver) = broadcast::channel::<Notice>(8);
        let mut stream = NoticeStream::new(receiver);
        drop(sender);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn fan_out_to_two_subscribers() {
        let (sender, _) = broadcast::channel(8);
        let mut a = NoticeStream::new(sender.subscribe());
        let mut b = NoticeStream::new(sender.subscribe());

        sender.send(make_notice(1100, "lost")).unwrap();

        assert_eq!(a.next().await.unwrap().code, 1100);
        assert_eq!(b.next().await.unwrap().code, 1100);
    }

    #[tokio::test]
    async fn lag_is_skipped() {
        let (sender, receiver) = broadcast::channel(2);
        let mut stream = NoticeStream::new(receiver);

        // Overflow the channel; receiver lags.
        for code in 1..=4 {
            sender.send(make_notice(code, "")).unwrap();
        }
        // First recv lags; loop in `next` skips the lag and returns the most recent.
        let n = stream.next().await.expect("notice");
        assert!(n.code >= 3, "expected most recent post-lag notice, got {}", n.code);
    }

    #[tokio::test]
    async fn stream_adapter_collects() {
        let (sender, receiver) = broadcast::channel(8);
        let mut s = NoticeStream::new(receiver);

        for code in [2104, 2107, 1102] {
            sender.send(make_notice(code, "")).unwrap();
        }
        drop(sender);

        let collected: Vec<i32> = s.stream().map(|n| n.code).collect().await;
        assert_eq!(collected, vec![2104, 2107, 1102]);
    }
}
