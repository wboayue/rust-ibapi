use std::cell::Cell;
use std::sync::Arc;

use crate::common::test_utils::helpers::proto_response;
use crate::contracts::Contract;
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
    expected: &'static [&'static str],
    open_calls: usize,
}

const CASES: &[Case] = &[
    Case {
        name: "streams items, no gap notice on the first open",
        resets: 0,
        open_fails: &[],
        reconnect_fails: false,
        expected: &["bar", "bar"],
        open_calls: 1,
    },
    Case {
        name: "resubscribes after a reset, gap notice first",
        resets: 1,
        open_fails: &[],
        reconnect_fails: false,
        expected: &["gap", "bar", "bar"],
        open_calls: 2,
    },
    Case {
        name: "lost again while reopening, still one gap notice",
        resets: 1,
        open_fails: &[(1, || Error::ConnectionReset)],
        reconnect_fails: false,
        expected: &["gap", "bar", "bar"],
        open_calls: 3,
    },
    Case {
        name: "first open lost, retried without a gap notice",
        resets: 0,
        open_fails: &[(0, || Error::ConnectionReset)],
        reconnect_fails: false,
        expected: &["bar", "bar"],
        open_calls: 2,
    },
    Case {
        name: "other open error ends the stream",
        resets: 0,
        open_fails: &[(0, || Error::AlreadySubscribed)],
        reconnect_fails: false,
        expected: &["AlreadySubscribed"],
        open_calls: 1,
    },
    Case {
        name: "reconnect gave up: its error ends the stream",
        resets: 1,
        open_fails: &[],
        reconnect_fails: true,
        expected: &["ConnectionFailed"],
        open_calls: 1,
    },
];

fn bus(case: &Case) -> Arc<MessageBusStub> {
    let bar = |time| proto_response(IncomingMessages::RealTimeBars, realtime_bar_tick().time(time).encode_proto());
    let mut stub = MessageBusStub::with_ordered_responses(vec![bar(1678323335), bar(1678323340)]).with_connection_resets(case.resets);
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
