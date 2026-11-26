use std::cell::Cell;
use std::sync::Arc;

use crate::common::test_utils::helpers::{proto_error_response, proto_response, TEST_REQ_ID_FIRST};
use crate::contracts::Contract;
use crate::market_data::historical::HistoricalDataFrame;
use crate::market_data::realtime::Bar;
use crate::messages::{IncomingMessages, TRANSPORT_RECONNECT_CODE};
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::market_data::realtime_bar_tick;
use crate::testdata::builders::ResponseProtoEncoder;
use crate::Error;

/// `open` call index, and the error it fails with before sending.
type OpenFailure = (usize, fn() -> Error);

struct Case {
    name: &'static str,
    /// Requests whose subscription gets a `ConnectionReset` instead of bars.
    resets: usize,
    open_fails: &'static [OpenFailure],
    reconnect_fails: bool,
    /// TWS error code sent on the first request after its bars.
    stream_error: Option<i32>,
    expected: &'static [&'static str],
    open_calls: usize,
}

const CASES: &[Case] = &[
    Case {
        name: "streams items, no gap notice on the first open",
        resets: 0,
        open_fails: &[],
        reconnect_fails: false,
        stream_error: None,
        expected: &["bar", "bar"],
        open_calls: 1,
    },
    Case {
        name: "resubscribes after a reset, gap notice first",
        resets: 1,
        open_fails: &[],
        reconnect_fails: false,
        stream_error: None,
        expected: &["gap", "bar", "bar"],
        open_calls: 2,
    },
    Case {
        name: "lost again while reopening, still one gap notice",
        resets: 1,
        open_fails: &[(1, || Error::ConnectionReset)],
        reconnect_fails: false,
        stream_error: None,
        expected: &["gap", "bar", "bar"],
        open_calls: 3,
    },
    Case {
        name: "first open lost, retried without a gap notice",
        resets: 0,
        open_fails: &[(0, || Error::ConnectionReset)],
        reconnect_fails: false,
        stream_error: None,
        expected: &["bar", "bar"],
        open_calls: 2,
    },
    Case {
        name: "other open error ends the stream",
        resets: 0,
        open_fails: &[(0, || Error::AlreadySubscribed)],
        reconnect_fails: false,
        stream_error: None,
        expected: &["AlreadySubscribed"],
        open_calls: 1,
    },
    Case {
        name: "reconnect gave up: its error ends the stream",
        resets: 1,
        open_fails: &[],
        reconnect_fails: true,
        stream_error: None,
        expected: &["ConnectionFailed"],
        open_calls: 1,
    },
    Case {
        name: "reconnect gave up while reopening: its error ends the stream",
        resets: 0,
        open_fails: &[(0, || Error::ConnectionReset)],
        reconnect_fails: true,
        stream_error: None,
        expected: &["ConnectionFailed"],
        open_calls: 1,
    },
    Case {
        name: "other stream error ends the stream, no resubscribe",
        resets: 0,
        open_fails: &[],
        reconnect_fails: false,
        stream_error: Some(420),
        expected: &["bar", "bar", "error 420"],
        open_calls: 1,
    },
];

fn bus(case: &Case) -> Arc<MessageBusStub> {
    let bar = |time| proto_response(IncomingMessages::RealTimeBars, realtime_bar_tick().time(time).encode_proto());
    let mut responses = vec![bar(1678323335), bar(1678323340)];
    if let Some(code) = case.stream_error {
        responses.push(proto_error_response(TEST_REQ_ID_FIRST, code, "Invalid Real-time Query"));
    }
    let mut stub = MessageBusStub::with_ordered_responses(responses).with_connection_resets(case.resets);
    if case.reconnect_fails {
        stub = stub.with_failed_reconnect();
    }
    Arc::new(stub)
}

/// The scripted failure for `open` call `call`, if any.
fn open_failure(case: &Case, call: usize) -> Option<Error> {
    case.open_fails.iter().find(|(i, _)| *i == call).map(|(_, error)| error())
}

fn label(item: Result<SubscriptionItem<Bar>, Error>) -> String {
    match item {
        Ok(SubscriptionItem::Data(_)) => "bar".into(),
        Ok(SubscriptionItem::Notice(n)) if n.code == TRANSPORT_RECONNECT_CODE => "gap".into(),
        Ok(SubscriptionItem::Notice(n)) => format!("notice {n}"),
        Err(Error::Notice(n)) => format!("error {}", n.code),
        Err(e) => format!("{e:?}"),
    }
}

#[cfg(feature = "sync")]
#[test]
fn resubscribing_sync() {
    use crate::client::sync::Client;

    for case in CASES {
        let bus = bus(case);
        let client = Client::stubbed(bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();
        let calls = Cell::new(0);

        let open = || {
            let call = calls.replace(calls.get() + 1);
            match open_failure(case, call) {
                Some(error) => Err(error),
                None => client.realtime_bars(&contract).subscribe(),
            }
        };
        let labels: Vec<String> = client.resubscribing(open).map(label).collect();

        assert_eq!(labels, case.expected, "{}", case.name);
        assert_eq!(calls.get(), case.open_calls, "{}: open calls", case.name);
    }
}

#[cfg(feature = "async")]
#[tokio::test]
async fn resubscribing_async() {
    use crate::client::r#async::Client;
    use futures::StreamExt;

    for case in CASES {
        let bus = bus(case);
        let client = Client::stubbed(bus.clone(), server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();
        let calls = Cell::new(0);

        let open = || {
            let call = calls.replace(calls.get() + 1);
            let failure = open_failure(case, call);
            let (client, contract) = (&client, &contract);
            async move {
                match failure {
                    Some(error) => Err(error),
                    None => client.realtime_bars(contract).subscribe().await,
                }
            }
        };
        let labels: Vec<String> = client.resubscribing(open).map(label).collect().await;

        assert_eq!(labels, case.expected, "{}", case.name);
        assert_eq!(calls.get(), case.open_calls, "{}: open calls", case.name);
    }
}

/// `historical_data(..).subscribe()` through `resubscribing`: `resets` cut the
/// first requests before any frame.
struct HistoricalCase {
    name: &'static str,
    resets: usize,
    expected: &'static [&'static str],
    requests: usize,
}

const HISTORICAL_CASES: &[HistoricalCase] = &[
    HistoricalCase {
        name: "ends after End, though the channel stays open",
        resets: 0,
        expected: &["bars", "end"],
        requests: 1,
    },
    HistoricalCase {
        name: "cut before End: the whole request again",
        resets: 1,
        expected: &["gap", "bars", "end"],
        requests: 2,
    },
];

fn historical_bus(case: &HistoricalCase) -> Arc<MessageBusStub> {
    use crate::testdata::builders::market_data::{historical_data_bar, historical_data_end_response, historical_data_response};
    let responses = vec![
        proto_response(
            IncomingMessages::HistoricalData,
            historical_data_response()
                .bar(historical_data_bar(1_678_886_400).ohlc(1.0, 2.0, 0.5, 1.5))
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::HistoricalDataEnd,
            historical_data_end_response()
                .start_date_str("20230315 09:30:00 UTC")
                .end_date_str("20230315 10:30:00 UTC")
                .encode_proto(),
        ),
    ];
    // Open channels, as in production: without the snapshot check the
    // reader would wait after End instead of ending.
    Arc::new(
        MessageBusStub::with_ordered_responses(responses)
            .with_connection_resets(case.resets)
            .keeping_channels_open(),
    )
}

fn historical_label(item: Result<SubscriptionItem<HistoricalDataFrame>, Error>) -> String {
    match item {
        Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(_))) => "bars".into(),
        Ok(SubscriptionItem::Data(HistoricalDataFrame::End { .. })) => "end".into(),
        Ok(SubscriptionItem::Data(HistoricalDataFrame::Update(_))) => "update".into(),
        other => label_notice_or_error(other),
    }
}

fn label_notice_or_error<T>(item: Result<SubscriptionItem<T>, Error>) -> String {
    match item {
        Ok(SubscriptionItem::Data(_)) => "data".into(),
        Ok(SubscriptionItem::Notice(n)) if n.code == TRANSPORT_RECONNECT_CODE => "gap".into(),
        Ok(SubscriptionItem::Notice(n)) => format!("notice {n}"),
        Err(Error::Notice(n)) => format!("error {}", n.code),
        Err(e) => format!("{e:?}"),
    }
}

/// Requests sent match the case, and a request read through `End` is not
/// cancelled. Cancels are checked only without resets: the sync stub's
/// `wait_connected` starts no new session, so the request the reset killed
/// still looks live to its drop.
fn assert_historical_requests(bus: &MessageBusStub, case: &HistoricalCase) {
    use crate::common::test_utils::helpers::count_proto_msgs;
    use crate::messages::OutgoingMessages;
    let messages = bus.request_messages();
    let requests = count_proto_msgs(&messages, OutgoingMessages::RequestHistoricalData);
    assert_eq!(requests, case.requests, "{}: requests", case.name);
    if case.resets == 0 {
        let cancels = count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalData);
        assert_eq!(cancels, 0, "{}: no cancel after End", case.name);
    }
}

#[cfg(feature = "sync")]
#[test]
fn resubscribing_historical_subscribe_sync() {
    use crate::client::sync::Client;
    use crate::market_data::historical::{BarSize, ToDuration};
    use std::sync::mpsc;
    use std::time::Duration;

    for case in HISTORICAL_CASES {
        let bus = historical_bus(case);
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        // On a thread, so a reader stuck after End fails the test instead of hanging it.
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let contract = Contract::stock("AAPL").build();
            let open = || client.historical_data(&contract, BarSize::Hour).duration(1.days()).subscribe();
            let labels: Vec<String> = client.resubscribing(open).map(historical_label).collect();
            let _ = tx.send(labels);
        });
        let labels = rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or_else(|_| panic!("{}: did not end", case.name));

        assert_eq!(labels, case.expected, "{}", case.name);
        assert_historical_requests(&bus, case);
    }
}

#[cfg(feature = "async")]
#[tokio::test]
async fn resubscribing_historical_subscribe_async() {
    use crate::client::r#async::Client;
    use crate::market_data::historical::{BarSize, ToDuration};
    use futures::StreamExt;
    use std::time::Duration;

    for case in HISTORICAL_CASES {
        let bus = historical_bus(case);
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_HISTORICAL_DATA);
        let contract = Contract::stock("AAPL").build();

        let open = || client.historical_data(&contract, BarSize::Hour).duration(1.days()).subscribe();
        let labels = tokio::time::timeout(
            Duration::from_secs(5),
            client.resubscribing(open).map(historical_label).collect::<Vec<_>>(),
        )
        .await
        .unwrap_or_else(|_| panic!("{}: did not end", case.name));

        assert_eq!(labels, case.expected, "{}", case.name);
        assert_historical_requests(&bus, case);
    }
}
