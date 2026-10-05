use super::*;
use crate::client::ids::{OrderId, RequestId};
use crate::connection::common::{ConnectionHandler, ConnectionProtocol};
use crate::connection::sync::Connection;
use crate::tests::assert_send_and_sync;
use crate::transport::common::MAX_RECONNECT_ATTEMPTS;

// Additional imports for connection tests
use crate::client::sync::Client;
use crate::common::test_utils::helpers;
use crate::common::test_utils::helpers::{
    binary_proto, body, error_frame, execution_data_frame, farm_ok_frame_42, farm_ok_frame_unrouted, proto_response, NoticeTestData, FARM_OK_MSG,
};
use crate::contracts::Contract;
use crate::messages::{encode_length, encode_raw_length, OutgoingMessages, RequestMessage, TRANSPORT_RECONNECT_CODE};
use crate::orders::common::encoders::encode_place_order;
use crate::orders::{order_builder, Action};
use crate::testdata::builders::contracts::contract_data;
use crate::testdata::builders::orders::order_bound;
use crate::testdata::builders::ResponseProtoEncoder;
use crate::transport::raw_capture::{test_support, RawFrameTap};
use crate::transport::sync::MemoryStream;
use crate::transport::MessageBus;
use log::{debug, trace};
use std::collections::VecDeque;
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn encode_request_contract_data(_server_version: i32, request_id: i32, contract: &Contract) -> Result<Vec<u8>, Error> {
    // Build the protobuf-encoded contract data request directly
    use crate::messages::{encode_protobuf_message, OutgoingMessages};
    use prost::Message;
    let request = crate::proto::ContractDataRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestContractData as i32,
        &request.encode_to_vec(),
    ))
}

#[test]
fn test_thread_safe() {
    assert_send_and_sync::<Connection<TcpSocket>>();
    assert_send_and_sync::<TcpMessageBus<TcpSocket>>();
}

// Connection test helpers

fn mock_socket_error(kind: ErrorKind) -> Error {
    let message = format!("Simulated {} error", kind);
    debug!("mock -> {message}");
    let io_error = std::io::Error::new(kind, message);
    Error::Io(io_error)
}

#[derive(Debug)]
struct MockSocket {
    // Read only
    exchanges: Vec<Exchange>,
    expected_retries: usize,
    reconnect_call_count: AtomicUsize,

    // Accessed from reader thread
    // Mutated by reader thread
    keep_alive: AtomicBool,

    // Accessed from reader thread
    // Mutated by writer threads
    write_call_count: AtomicUsize,
    responses_len: AtomicUsize,

    // Accessed from read thread
    // Mutated by reader thread & writer threads
    read_call_count: AtomicUsize,
}

impl MockSocket {
    pub fn new(exchanges: Vec<Exchange>, expected_retries: usize) -> Self {
        Self {
            exchanges,
            expected_retries,
            keep_alive: AtomicBool::new(false),
            reconnect_call_count: AtomicUsize::new(0),
            write_call_count: AtomicUsize::new(0),
            responses_len: AtomicUsize::new(0),
            read_call_count: AtomicUsize::new(0),
        }
    }
}

impl Reconnect for MockSocket {
    fn reconnect(&self) -> Result<(), Error> {
        let reconnect_call_count = self.reconnect_call_count.load(Ordering::SeqCst);

        if reconnect_call_count == self.expected_retries {
            return Ok(());
        }

        self.reconnect_call_count.fetch_add(1, Ordering::SeqCst);
        Err(mock_socket_error(ErrorKind::ConnectionRefused))
    }
    fn sleep(&self, _duration: std::time::Duration, _shutdown: &ShutdownSignal) {}
    fn shutdown_read(&self) -> Result<(), Error> {
        self.keep_alive.store(true, Ordering::SeqCst);
        Ok(())
    }
}

impl Stream for MockSocket {}

impl Io for MockSocket {
    fn read_message(&self) -> Result<Vec<u8>, Error> {
        trace!("===== mock read =====");

        if self.keep_alive.load(Ordering::SeqCst) {
            return Err(mock_socket_error(ErrorKind::WouldBlock));
        }

        // if response_index > responses len (too many reads for the given exchange)
        // the next read executed before the next write
        // and happens if the mock socket is used with the dispatcher thread
        // this blocks the dispatcher thread until the write has executed
        while self.read_call_count.load(Ordering::SeqCst) >= self.responses_len.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // The state may have changed while waiting
        let write_call_count = self.write_call_count.load(Ordering::SeqCst);
        let read_call_count = self.read_call_count.load(Ordering::SeqCst);
        let exchange = &self.exchanges[write_call_count - 1];
        let responses = &exchange.responses;

        trace!(
            "mock read: responses.len(): {}, read_call_count: {}, write_call_count: {}, exchange_index: {}",
            responses.len(),
            read_call_count,
            write_call_count,
            write_call_count - 1
        );

        let response = responses.get(read_call_count).unwrap();

        // disconnect if a null byte response is encountered. Protobuf fixtures
        // carry no text fields at all, so this reads the first one if present.
        if response.fields.first().is_some_and(|field| field == "\0") {
            return Err(mock_socket_error(ErrorKind::ConnectionReset));
        }

        let encoded = response.encode();

        // if there are no more remaining exchanges or responses
        // set keep_alive - so the client can gracefully disconnect
        if write_call_count >= self.exchanges.len() && read_call_count >= responses.len() - 1 {
            self.keep_alive.store(true, Ordering::SeqCst);
        }

        self.read_call_count.fetch_add(1, Ordering::SeqCst);

        debug!("mock read {:?}", &encoded);

        // Handshake responses use pure text format.
        // Protobuf-framed responses: 4-byte BE (msg_id + PROTOBUF_MSG_ID) + proto bytes.
        // Other responses use binary-text format (4-byte BE msg_id + text payload).
        if exchange.is_handshake {
            let expected = encode_length(&encoded);
            read_message(&mut expected.as_slice(), &RawFrameTap::disabled())
        } else if let Some(raw) = response.raw_bytes() {
            let msg_id = response.message_type() as i32;
            Ok(crate::messages::encode_protobuf_message(msg_id, raw))
        } else {
            let fields: Vec<&str> = encoded.split_terminator('\0').collect();
            let msg_id: i32 = fields[0].parse().unwrap_or(0);
            let text_payload: String = fields[1..].iter().map(|f| format!("{f}\0")).collect();
            let mut data = Vec::new();
            data.extend_from_slice(&msg_id.to_be_bytes());
            data.extend_from_slice(text_payload.as_bytes());
            Ok(data)
        }
    }

    fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        trace!("===== mock write =====");
        let write_call_count = self.write_call_count.load(Ordering::SeqCst);
        trace!("mock write: write_call_count: {write_call_count}");

        let exchange = self.exchanges.get(write_call_count).unwrap();
        let request = &exchange.request;

        let is_handshake = buf.starts_with(b"API\0");

        // strip API\0 if handshake
        let buf = if is_handshake {
            &buf[4..] // strip prefix
        } else {
            buf
        };

        // Length-prefix the expected request bytes
        let expected = crate::messages::encode_raw_length(request);
        let expected = &expected;

        debug!("mock write {:?}", &buf[4..]);
        debug!("mock write: write_call_count={write_call_count}, is_handshake={is_handshake}");

        assert_eq!(expected, buf, "mock write mismatch");

        self.read_call_count.store(0, Ordering::SeqCst);
        self.write_call_count.fetch_add(1, Ordering::SeqCst);
        self.responses_len.store(exchange.responses.len(), Ordering::SeqCst);

        Ok(())
    }
}

#[derive(Debug)]
struct Exchange {
    request: Vec<u8>,
    responses: VecDeque<ResponseMessage>,
    /// True for handshake exchanges where responses are pure text (no binary msg_id prefix).
    is_handshake: bool,
}

impl Exchange {
    fn new(request: Vec<u8>, responses: Vec<ResponseMessage>) -> Self {
        Self {
            request,
            responses: VecDeque::from(responses),
            is_handshake: false,
        }
    }
    fn simple(request: &str, responses: &[&str]) -> Self {
        let responses = responses
            .iter()
            .map(|s| ResponseMessage::from_simple(s))
            .collect::<Vec<ResponseMessage>>();
        // Convert pipe-delimited text to NUL-delimited, then extract msg_id for binary encoding
        let nul_delimited = request.replace('|', "\0");
        let mut exchange = Self::new(nul_delimited.into_bytes(), responses);
        exchange.is_handshake = true;
        exchange
    }
    fn request(request: Vec<u8>, responses: &[&str]) -> Self {
        let responses = responses
            .iter()
            .map(|s| ResponseMessage::from_simple(s))
            .collect::<Vec<ResponseMessage>>();
        Self::new(request, responses)
    }
}

/// Builds the handshake request string the client sends, derived from the
/// handler's advertised version range (mirrors `format_handshake`). Deriving it
/// from the constants keeps these fixtures correct across floor/max ratchets
/// (docs/rules/testing/derive-from-constants.md).
fn handshake_request(handler: &ConnectionHandler) -> String {
    format!("v{}..{}", handler.min_version, handler.max_version)
}

fn managed_accounts_response(accounts: &str) -> ResponseMessage {
    use prost::Message;
    let bytes = crate::proto::ManagedAccounts {
        accounts_list: Some(accounts.to_string()),
    }
    .encode_to_vec();
    proto_response(crate::messages::IncomingMessages::ManagedAccounts, bytes)
}

fn next_valid_id_response(order_id: i32) -> ResponseMessage {
    use prost::Message;
    let bytes = crate::proto::NextValidId { order_id: Some(order_id) }.encode_to_vec();
    proto_response(crate::messages::IncomingMessages::NextValidId, bytes)
}

#[test]
fn test_bus_send_order_request() -> Result<(), Error> {
    use prost::Message;
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let order = order_builder::market_order(Action::Buy, 100.0);
    let contract = &Contract::stock("AAPL").build();
    let request = encode_place_order(5, contract, &order)?;

    let open_order_proto = |status: &str| {
        proto_response(
            crate::messages::IncomingMessages::OpenOrder,
            crate::proto::OpenOrder {
                order_id: Some(5),
                order_state: Some(crate::proto::OrderState {
                    status: Some(status.into()),
                    ..Default::default()
                }),
                ..Default::default()
            }
            .encode_to_vec(),
        )
    };
    let order_status_proto = |status: &str, filled: i64| {
        proto_response(
            crate::messages::IncomingMessages::OrderStatus,
            crate::proto::OrderStatus {
                order_id: Some(5),
                status: Some(status.into()),
                filled: Some(filled.to_string()),
                ..Default::default()
            }
            .encode_to_vec(),
        )
    };
    let execution_data_proto = proto_response(
        crate::messages::IncomingMessages::ExecutionData,
        crate::proto::ExecutionDetails {
            req_id: Some(-1),
            contract: None,
            execution: Some(crate::proto::Execution {
                order_id: Some(5),
                exec_id: Some("0000e0d5.67fe667b.01.01".into()),
                ..Default::default()
            }),
        }
        .encode_to_vec(),
    );

    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250415 19:38:30 British Summer Time|")]),
        Exchange::new(start_api_bytes, vec![managed_accounts_response("DU1234567"), next_valid_id_response(5)]),
        Exchange::new(
            request.clone(),
            vec![
                open_order_proto("PreSubmitted"),
                order_status_proto("PreSubmitted", 0),
                execution_data_proto,
                open_order_proto("Filled"),
                order_status_proto("Filled", 100),
            ],
        ),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::with_socket(stream, 28, None, std::sync::Arc::new(crate::transport::sync::NoticeBroadcaster::new()));
    connection.establish_connection()?;
    let bus = Arc::new(TcpMessageBus::new(connection)?);

    let subscription = bus.send_order_request(OrderId::from(5), &request)?;

    bus.dispatch()?;
    bus.dispatch()?;
    bus.dispatch()?;
    bus.dispatch()?;
    bus.dispatch()?;

    subscription.next().unwrap()?;

    Ok(())
}

#[test]
fn test_connection_establish_connection() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes,
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(1),
                ResponseMessage::from_simple("4|2|-1|2104|Market data farm connection is OK:usfarm||"),
            ],
        ),
    ];
    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;

    Ok(())
}

#[test]
fn test_reconnect_failed() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes,
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(1),
                ResponseMessage::from_simple("\0"),
            ],
        ),
    ];
    let socket = MockSocket::new(events, MAX_RECONNECT_ATTEMPTS as usize + 1);

    let connection = Connection::stubbed(socket, 28);
    connection.establish_connection()?;

    let _ = connection.read_message();

    // Exhausted attempts surface the last attempt's error, not a generic
    // `Error::ConnectionFailed`.
    match connection.reconnect() {
        Err(Error::Io(e)) if e.kind() == ErrorKind::ConnectionRefused => Ok(()),
        other => panic!("expected the last reconnect error, got {other:?}"),
    }
}

#[test]
fn test_reconnect_success() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(1),
                ResponseMessage::from_simple("\0"),
            ],
        ),
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(start_api_bytes, vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)]),
    ];
    let socket = MockSocket::new(events, MAX_RECONNECT_ATTEMPTS as usize - 1);

    let connection = Connection::stubbed(socket, 28);
    connection.establish_connection()?;

    let _ = connection.read_message();

    connection.reconnect()
}

#[test]
fn test_client_reconnect() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let managed_req = crate::accounts::common::encoders::encode_request_managed_accounts().unwrap();
    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)],
        ),
        Exchange::new(managed_req.clone(), vec![ResponseMessage::from_simple("\0")]),
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(start_api_bytes, vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)]),
        Exchange::new(managed_req, vec![managed_accounts_response("DU1234567")]),
    ];
    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
    let server_version = connection.server_version();
    let bus = Arc::new(TcpMessageBus::new(connection)?);
    bus.process_messages(server_version)?;
    let client = Client::stubbed(bus.clone(), server_version);

    client.managed_accounts()?;

    Ok(())
}

/// Regression: a previous version of `reset()` cleared `connected` *after* the
/// dispatcher had restored it to true on successful reconnect, so
/// `is_connected()` was permanently false after the first network blip.
#[test]
fn test_is_connected_stays_true_after_reconnect() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(1),
                ResponseMessage::from_simple("\0"),
            ],
        ), // RESTART
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(start_api_bytes, vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)]),
    ];
    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
    let bus = TcpMessageBus::new(connection)?;

    assert!(bus.is_connected(), "bus should be connected after initial handshake");

    bus.dispatch()?; // reads "\0", reconnects, restores connected=true

    assert!(bus.is_connected(), "bus should still be connected after reconnect");

    Ok(())
}

/// A successful automatic reconnect replays the handshake, whose
/// `NextValidId` is a fresh server floor for order IDs. The bus must raise
/// the client's generator from it: before this, only the initial connection
/// seeded the generator and every reconnect silently discarded the value,
/// leaving allocation stale against the server.
#[test]
fn test_reconnect_raises_order_ids_from_handshake() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(100),
                ResponseMessage::from_simple("\0"),
            ],
        ), // RESTART
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes,
            vec![managed_accounts_response("DU1234567"), next_valid_id_response(5000)],
        ),
    ];
    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
    let bus = TcpMessageBus::new(connection)?;

    let order_ids = Arc::new(ClientIdManager::new(100)?);
    bus.set_order_ids(order_ids.clone());

    bus.dispatch()?; // reads "\0", reconnects, handshake re-receives NextValidId(5000)

    assert_eq!(
        order_ids.current_order_id(),
        5000,
        "order-id generator should be raised from the reconnect handshake"
    );

    Ok(())
}

const AAPL_CONTRACT_RESPONSE: &str  = "AAPL|STK||0||SMART|USD|AAPL|NMS|NMS|265598|0.01||ACTIVETIM,AD,ADDONT,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SMARTSTG,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,NASDAQ,DRCTEDGE,BEX,BATS,EDGEA,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,IBEOS,OVERNIGHT,TPLUS0,PSX|1|0|APPLE INC|NASDAQ||Technology|Computers|Computers|US/Eastern|20250324:0400-20250324:2000;20250325:0400-20250325:2000;20250326:0400-20250326:2000;20250327:0400-20250327:2000;20250328:0400-20250328:2000|20250324:0930-20250324:1600;20250325:0930-20250325:1600;20250326:0930-20250326:1600;20250327:0930-20250327:1600;20250328:0930-20250328:1600|||1|ISIN|US0378331005|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|0.0001|0.0001|100|";

#[test]
fn test_send_request_after_disconnect() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let request_id = RequestId::nth(0);
    let packet = encode_request_contract_data(sv, request_id.raw(), &Contract::stock("AAPL").build())?;

    let expected_response = &format!("10|{request_id}|{AAPL_CONTRACT_RESPONSE}");

    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(1),
                ResponseMessage::from_simple("\0"),
            ],
        ), // RESTART
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(start_api_bytes, vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)]),
        Exchange::request(packet.clone(), &[expected_response, &format!("52|1|{}|", RequestId::nth(1))]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
    let bus = TcpMessageBus::new(connection)?;

    bus.dispatch()?;

    let subscription = bus.send_request(request_id, &packet)?;

    bus.dispatch()?;
    bus.dispatch()?;

    let result = subscription.next().unwrap()?;

    assert_eq!(result.encode_simple(), *expected_response);

    Ok(())
}

// If a request is sent before a restart
// the waiter should receive Error::ConnectionReset
#[test]
fn test_request_before_disconnect_raises_error() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let request_id = RequestId::nth(0);
    let packet = encode_request_contract_data(sv, request_id.raw(), &Contract::stock("AAPL").build())?;

    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)],
        ),
        Exchange::request(packet.clone(), &["\0"]), // RESTART
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(start_api_bytes, vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
    let bus = TcpMessageBus::new(connection)?;

    let notices = bus.connection.notice_broadcaster.subscribe();

    let subscription = bus.send_request(request_id, &packet)?;

    bus.dispatch()?;

    match subscription.next() {
        Some(Err(Error::ConnectionReset)) => {}
        _ => panic!(),
    }

    // The restart also publishes the synthesized reconnect notice to the
    // notice fan-out (see TRANSPORT_RECONNECT_CODE): the notice channel is
    // how a connection-state consumer without live subscriptions learns the
    // socket generation changed.
    loop {
        let notice = notices
            .recv_timeout(Duration::from_millis(100))
            .expect("no reconnect notice on the notice fan-out");
        if notice.code == TRANSPORT_RECONNECT_CODE {
            break;
        }
    }

    Ok(())
}

// If a request is sent during a restart
// the waiter should receive Error::ConnectionReset
#[test]
fn test_request_during_disconnect_raises_error() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let request_id = RequestId::nth(0);
    let packet = encode_request_contract_data(sv, request_id.raw(), &Contract::stock("AAPL").build())?;

    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(1),
                ResponseMessage::from_simple("\0"),
            ],
        ), // RESTART
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::request(packet.clone(), &[]),
        Exchange::new(start_api_bytes, vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;

    match connection.read_message() {
        Ok(_) => panic!(""),
        Err(_) => {
            connection.socket.reconnect()?;
            connection.handshake()?;
            connection.write_message(&packet)?;
            connection.start_api()?;
            connection.receive_account_info()?;
        }
    };

    Ok(())
}

#[test]
fn test_contract_details_disconnect_raises_error() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let contract = &Contract::stock("AAPL").build();

    // The stubbed client allocates the first request id.
    let packet = encode_request_contract_data(sv, RequestId::nth(0).raw(), contract)?;

    let events = vec![
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)],
        ),
        Exchange::request(packet.clone(), &["\0"]),
        Exchange::simple(&handshake_request(&handler), &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(start_api_bytes, vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
    let server_version = connection.server_version();
    let bus = Arc::new(TcpMessageBus::new(connection)?);
    bus.process_messages(server_version)?;
    let client = Client::stubbed(bus.clone(), server_version);

    match client.contract_details(contract) {
        Err(Error::ConnectionReset) => {}
        _ => panic!(),
    }

    Ok(())
}

#[test]
fn test_request_simple_encoding_roundtrip() {
    let expected = "17|1|";
    let req = RequestMessage::from_simple(expected);
    assert_eq!(req.fields, vec!["17", "1"]);
    let simple_encoded = req.encode_simple();
    assert_eq!(simple_encoded, expected);
}

#[test]
fn test_request_encoding_roundtrip() {
    let expected = "17\01\0";
    let req = RequestMessage::from(expected);
    assert_eq!(req.fields, vec!["17", "1"]);
    let encoded = req.encode();
    assert_eq!(encoded, expected);
}

// ---- routing tests using MemoryStream ----
//
// `MockSocket` pairs each write with a scripted response and can't easily
// express scenarios like interleaved responses or shared-channel fan-out.
// `MemoryStream` lets tests push response frames freely and drive
// `bus.dispatch()` directly.

/// Wrap a fresh `MemoryStream` in a stubbed `TcpMessageBus`. Pins
/// `server_version` to the current floor so `parse_raw_message` produces
/// binary-text-payload frames from `body()` inputs.
fn make_bus() -> (MemoryStream, Arc<TcpMessageBus<MemoryStream>>) {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), 28);
    connection.set_server_version_for_test(crate::server_versions::PROTOBUF_REST_MESSAGES_3);
    let bus = Arc::new(TcpMessageBus::new(connection).unwrap());
    (stream, bus)
}

const TICK: Duration = Duration::from_millis(100);

/// Two in-flight `send_request` subscriptions: responses arrive in reverse order
/// and each subscription receives only its own message. Validates `requests`
/// `SenderHash` lookup by request_id.
#[test]
fn test_request_id_correlation_with_interleaved_responses() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let (id_a, id_b) = (RequestId::nth(100), RequestId::nth(200));
    let sub_a = bus.send_request(id_a, &[])?;
    let sub_b = bus.send_request(id_b, &[])?;

    // HistogramData (msg_id 89): request_id at field index 1.
    stream.push_inbound(body(&format!("89|{id_b}|payload-b|")));
    stream.push_inbound(body(&format!("89|{id_a}|payload-a|")));

    bus.dispatch()?;
    bus.dispatch()?;

    let msg_a = sub_a.next_timeout(TICK).expect("sub_a got no message")?;
    let msg_b = sub_b.next_timeout(TICK).expect("sub_b got no message")?;
    assert_eq!(msg_a.peek_int(1)?, id_a.raw());
    assert_eq!(msg_b.peek_int(1)?, id_b.raw());

    // No cross-talk.
    assert!(sub_a.try_next().is_none(), "sub_a received an extra message");
    assert!(sub_b.try_next().is_none(), "sub_b received an extra message");
    Ok(())
}

/// Same shape as the request_id test but on the orders channel: two in-flight
/// `send_order_request` subscriptions, OrderStatus responses interleaved.
#[test]
fn test_order_id_correlation_with_interleaved_responses() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let sub_a = bus.send_order_request(OrderId::from(11), &[])?;
    let sub_b = bus.send_order_request(OrderId::from(22), &[])?;

    // OrderStatus carries `order_id` at proto tag 1.
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OrderStatus as i32,
        &crate::proto::OrderStatus {
            order_id: Some(22),
            status: Some("Filled".into()),
            ..Default::default()
        },
    ));
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OrderStatus as i32,
        &crate::proto::OrderStatus {
            order_id: Some(11),
            status: Some("Submitted".into()),
            ..Default::default()
        },
    ));

    bus.dispatch()?;
    bus.dispatch()?;

    let msg_a = sub_a.next_timeout(TICK).expect("sub_a got no message")?;
    let msg_b = sub_b.next_timeout(TICK).expect("sub_b got no message")?;
    assert_eq!(msg_a.order_id(), Some(11));
    assert_eq!(msg_b.order_id(), Some(22));

    // No cross-talk.
    assert!(sub_a.try_next().is_none(), "sub_a received an extra message");
    assert!(sub_b.try_next().is_none(), "sub_b received an extra message");
    Ok(())
}

/// Shared-channel fan-out: `RequestOpenOrders`, `RequestAllOpenOrders`, and
/// `RequestAutoOpenOrders` all map to `[OpenOrder, OrderStatus, OpenOrderEnd]`
/// in `CHANNEL_MAPPINGS`. With no `send_order_request` subscriber for the
/// incoming order_id, the `OrderOrShared` strategy in `process_orders` fans
/// the message out to every shared subscriber registered for `OpenOrder`.
#[test]
fn test_shared_channel_fan_out_for_open_orders() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let sub_open = bus.send_shared_request(OutgoingMessages::RequestOpenOrders, &[])?;
    let sub_all = bus.send_shared_request(OutgoingMessages::RequestAllOpenOrders, &[])?;
    let sub_auto = bus.send_shared_request(OutgoingMessages::RequestAutoOpenOrders, &[])?;

    // OpenOrder carries `order_id` at proto tag 1; no matching order subscription
    // means the OrderOrShared strategy falls back to fan-out across shared subs.
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OpenOrder as i32,
        &crate::proto::OpenOrder {
            order_id: Some(42),
            ..Default::default()
        },
    ));
    bus.dispatch()?;

    for (name, sub) in [("open", &sub_open), ("all", &sub_all), ("auto", &sub_auto)] {
        let msg = sub.next_timeout(TICK).unwrap_or_else(|| panic!("sub_{name} got no message"))?;
        assert_eq!(msg.message_type(), crate::messages::IncomingMessages::OpenOrder);
        assert_eq!(msg.order_id(), Some(42));
    }
    Ok(())
}

/// Shared-channel routing: `send_shared_request` for `RequestCurrentTime` should
/// receive the `CurrentTime` response via the channel mapping in
/// `shared_channel_configuration::CHANNEL_MAPPINGS`.
#[test]
fn test_shared_channel_routing_current_time() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let sub = bus.send_shared_request(OutgoingMessages::RequestCurrentTime, &[])?;

    // CurrentTime (msg_id 49): "49|version|epoch_seconds|"
    stream.push_inbound(body("49|1|1700000000|"));
    bus.dispatch()?;

    let msg = sub.next_timeout(TICK).expect("shared subscription got no message")?;
    assert_eq!(msg.peek_int(0)?, 49);
    assert_eq!(msg.peek_int(2)?, 1_700_000_000);
    Ok(())
}

/// EOF on the stream classifies as a connection error in `dispatch`, which
/// triggers reconnect; the stub's reconnect "succeeds" but the subsequent
/// handshake also reads EOF, so `dispatch` ultimately returns `ConnectionFailed`
/// rather than hanging or silently dropping the error. In-flight subscriptions
/// are notified of `Error::ConnectionReset` before the reconnect is attempted,
/// and the `Error::Shutdown` the failed reconnect then requests finds the
/// channel already cleared - so the reset is the only notification.
#[test]
fn test_dispatch_surfaces_connection_failure_after_eof() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_request(RequestId::nth(100), &[])?;

    stream.close();
    let err = bus.dispatch().expect_err("dispatch should surface an error");
    assert!(matches!(err, Error::ConnectionFailed), "unexpected error: {err:?}");

    let resp = sub.next_timeout(TICK).expect("subscription got no notification");
    assert!(matches!(resp, Err(Error::ConnectionReset)), "got: {resp:?}");
    assert!(sub.try_next().is_none(), "subscription received a second notification");
    Ok(())
}

/// Cleanup thread observes shutdown immediately via `crossbeam::select!`
/// over the signal channel + shutdown-notify channel, instead of polling
/// with `recv_timeout(1s)`. Regression guard for issue #523.
#[test]
fn test_cleanup_thread_exits_promptly_on_shutdown() {
    let (_stream, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let start = Instant::now();
    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    let elapsed = start.elapsed();

    // 500ms is 2x headroom over the 1s bug being guarded; comfortable
    // margin for slow CI runners while still failing loudly on regression.
    assert!(
        elapsed < Duration::from_millis(500),
        "cleanup-thread join took {elapsed:?}, expected <500ms"
    );
}

/// Dispatcher thread's blocked socket read is interrupted by
/// `Reconnect::shutdown_read` from `request_shutdown`, instead of waiting
/// up to the 1s `TWS_READ_TIMEOUT`. Companion to the cleanup-thread test;
/// together they cover both threads `Client::drop` joins. Issue #523.
#[test]
fn test_dispatcher_thread_exits_promptly_on_shutdown() {
    let (_stream, bus) = make_bus();
    bus.process_messages(0).expect("process_messages");

    let start = Instant::now();
    bus.ensure_shutdown();
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_millis(500), "ensure_shutdown took {elapsed:?}, expected <500ms");
}

/// `cancel()` notifies the subscription's own queue with `Error::Cancelled`
/// and unregisters its route through the cleanup thread.
#[test]
fn test_cancel_notifies_and_unregisters() -> Result<(), Error> {
    let (_, bus) = make_bus();
    let handle = bus.start_cleanup_thread();
    let (request_id, order_id) = (RequestId::nth(100), OrderId::from(42));
    let request = bus.send_request(request_id, &[])?;
    let order = bus.send_order_request(order_id, &[])?;

    request.cancel();
    order.cancel();

    for sub in [&request, &order] {
        let resp = sub.next_timeout(TICK).expect("subscription got no notification");
        assert!(matches!(resp, Err(Error::Cancelled)), "got: {resp:?}");
    }
    drain_cleanup_signals(&bus);
    assert!(!bus.requests.contains(&request_id), "request route outlived its cancel");
    assert!(!bus.orders.contains(&order_id), "order route outlived its cancel");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// Regression test for #894: cancelling an old subscription must neither
/// notify nor unregister a newer one under the same id (cancel used to send
/// `Cancelled` to, then remove, whatever was registered under the key).
#[test]
fn test_cancel_preserves_newer_subscription_under_same_id() -> Result<(), Error> {
    let (_, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let (request_id, order_id) = (RequestId::nth(100), OrderId::from(42));
    let request_a = bus.send_request(request_id, &[])?;
    let request_b = bus.send_request(request_id, &[])?;
    let order_a = bus.send_order_request(order_id, &[])?;
    let order_b = bus.send_order_request(order_id, &[])?;

    request_a.cancel();
    order_a.cancel();
    drain_cleanup_signals(&bus);

    assert!(bus.requests.contains(&request_id), "cancel removed the newer request registration");
    assert!(bus.orders.contains(&order_id), "cancel removed the newer order registration");
    assert!(request_b.try_next_routed().is_none(), "cancel notified the newer request subscription");
    assert!(order_b.try_next_routed().is_none(), "cancel notified the newer order subscription");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// `MessageBus::cancel_shared_subscription` with no counted subscription of
/// the type writes the cancel bytes through to the connection.
#[test]
fn test_cancel_shared_subscription_writes_through() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let mb: &dyn MessageBus = bus.as_ref();

    let ticket = SharedTicket {
        message_type: OutgoingMessages::RequestCurrentTime,
        generation: 0,
    };
    mb.cancel_shared_subscription(ticket, Some(b"cancel-bytes"))?;

    let captured = stream.captured();
    assert!(captured.windows(b"cancel-bytes".len()).any(|w| w == b"cancel-bytes"));
    Ok(())
}

/// Number of length-prefixed frames in `captured` whose payload is `payload`.
fn count_frames(captured: &[u8], payload: &[u8]) -> usize {
    let mut rest = captured;
    let mut count = 0;
    while rest.len() >= 4 {
        let len = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        let (frame, tail) = rest[4..].split_at(len);
        count += usize::from(frame == payload);
        rest = tail;
    }
    count
}

/// TWS keeps one positions stream per client (`CancelPositions` carries no id),
/// so with two live `RequestPositions` subscriptions the first drop must not
/// write the cancel, the survivor must keep receiving, and the last drop writes
/// exactly one cancel.
#[test]
fn test_shared_subscription_cancel_waits_for_last_subscriber() -> Result<(), Error> {
    use crate::accounts::PositionUpdate;
    use crate::subscriptions::{StreamDecoder, SubscriptionItem};

    let (stream, bus) = make_bus();
    let cancel = <PositionUpdate as StreamDecoder<PositionUpdate>>::cancel_message(0, None, None)?;
    let cancels_written = || count_frames(&stream.captured(), &cancel);

    let first: crate::subscriptions::sync::Subscription<PositionUpdate> =
        wrap_subscription(bus.clone(), bus.send_shared_request(OutgoingMessages::RequestPositions, b"positions")?);
    let second: crate::subscriptions::sync::Subscription<PositionUpdate> =
        wrap_subscription(bus.clone(), bus.send_shared_request(OutgoingMessages::RequestPositions, b"positions")?);

    drop(first);
    assert_eq!(cancels_written(), 0, "cancel written while a subscription is still live");

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::PositionEnd as i32,
        &crate::proto::PositionEnd {},
    ));
    bus.dispatch()?;
    let item = second.next_timeout(TICK).expect("survivor received nothing")?;
    assert!(matches!(item, SubscriptionItem::Data(PositionUpdate::PositionEnd)), "got: {item:?}");

    drop(second);
    assert_eq!(cancels_written(), 1, "last drop must write exactly one cancel");
    Ok(())
}

/// `Orders` has no cancel message, so its drop writes nothing; the count for
/// its request type must still return to zero, or a later cancel for the
/// type would be withheld.
#[test]
fn test_shared_subscription_without_cancel_message_releases_count() -> Result<(), Error> {
    use crate::orders::Orders;

    let (stream, bus) = make_bus();
    let count = || bus.shared_channels.counts.lock().unwrap().live(OutgoingMessages::RequestOpenOrders);

    let sub: crate::subscriptions::sync::Subscription<Orders> =
        wrap_subscription(bus.clone(), bus.send_shared_request(OutgoingMessages::RequestOpenOrders, b"open-orders")?);
    assert_eq!(count(), 1);

    drop(sub);
    assert_eq!(count(), 0, "count leaked for a type with no cancel message");
    assert_eq!(count_frames(&stream.captured(), b"open-orders"), 1, "only the request was written");
    Ok(())
}

type PositionsSubscription = crate::subscriptions::sync::Subscription<crate::accounts::PositionUpdate>;

fn positions_subscription(bus: &Arc<TcpMessageBus<MemoryStream>>) -> Result<PositionsSubscription, Error> {
    Ok(wrap_subscription(
        bus.clone(),
        bus.send_shared_request(OutgoingMessages::RequestPositions, b"positions")?,
    ))
}

fn positions_cancel() -> Vec<u8> {
    use crate::subscriptions::StreamDecoder;
    <crate::accounts::PositionUpdate as StreamDecoder<crate::accounts::PositionUpdate>>::cancel_message(0, None, None).unwrap()
}

/// A reset ends every shared subscription, so the count restarts at zero for
/// the new session: a resubscription made after the reset is the only live one
/// and its drop writes the cancel even while the dead handle is still held.
#[test]
fn test_shared_count_restarts_after_reset() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let cancel = positions_cancel();

    let old = positions_subscription(&bus)?;
    bus.reset();
    assert_eq!(bus.shared_channels.counts.lock().unwrap().live(OutgoingMessages::RequestPositions), 0);

    let new = positions_subscription(&bus)?;
    drop(new);
    assert_eq!(count_frames(&stream.captured(), &cancel), 1, "cancel withheld by a dead handle");

    drop(old);
    assert_eq!(count_frames(&stream.captured(), &cancel), 1, "dead handle wrote a cancel");
    Ok(())
}

/// A handle from before the reset is dead: its drop must neither cancel on the
/// new session, which has no such stream, nor touch the new session's count.
#[test]
fn test_stale_shared_handle_neither_cancels_nor_decrements() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let cancel = positions_cancel();

    let old = positions_subscription(&bus)?;
    bus.reset();
    let new = positions_subscription(&bus)?;

    drop(old);
    assert_eq!(count_frames(&stream.captured(), &cancel), 0, "dead handle wrote a cancel");
    assert_eq!(bus.shared_channels.counts.lock().unwrap().live(OutgoingMessages::RequestPositions), 1);

    drop(new);
    assert_eq!(count_frames(&stream.captured(), &cancel), 1);
    assert_eq!(bus.shared_channels.counts.lock().unwrap().live(OutgoingMessages::RequestPositions), 0);
    Ok(())
}

type AccountUpdatesSubscription = crate::subscriptions::sync::Subscription<crate::accounts::AccountUpdate>;

fn account(name: &str) -> crate::accounts::types::AccountId {
    crate::accounts::types::AccountId(name.to_string())
}

// The request bytes name the account so each account's writes can be counted.
fn account_updates_request(name: &str) -> Vec<u8> {
    format!("account-updates {name}").into_bytes()
}

fn account_updates_subscription(bus: &Arc<TcpMessageBus<MemoryStream>>, name: &str) -> Result<AccountUpdatesSubscription, Error> {
    let internal = bus.send_account_updates_request(&account(name), &account_updates_request(name))?;
    Ok(wrap_subscription(bus.clone(), internal))
}

fn account_updates_cancel() -> Vec<u8> {
    use crate::subscriptions::StreamDecoder;
    <crate::accounts::AccountUpdate as StreamDecoder<crate::accounts::AccountUpdate>>::cancel_message(0, None, None).unwrap()
}

fn account_updates_slot(bus: &TcpMessageBus<MemoryStream>) -> Option<crate::accounts::types::AccountId> {
    bus.shared_channels.counts.lock().unwrap().account_updates().cloned()
}

/// The same account again shares the one TWS stream: both requests go out,
/// and only the last drop cancels and frees the slot.
#[test]
fn test_account_updates_same_account_shares() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let cancel = account_updates_cancel();

    let first = account_updates_subscription(&bus, "DU1")?;
    let second = account_updates_subscription(&bus, "DU1")?;
    assert_eq!(count_frames(&stream.captured(), &account_updates_request("DU1")), 2);
    assert_eq!(bus.shared_channels.counts.lock().unwrap().live(OutgoingMessages::RequestAccountData), 2);

    drop(first);
    assert_eq!(
        count_frames(&stream.captured(), &cancel),
        0,
        "cancel written while a subscription is still live"
    );
    assert_eq!(account_updates_slot(&bus), Some(account("DU1")));

    drop(second);
    assert_eq!(count_frames(&stream.captured(), &cancel), 1);
    assert_eq!(account_updates_slot(&bus), None);
    Ok(())
}

/// TWS has one account-updates slot: a second account would switch the live
/// subscription to its data. It is refused before anything is written, and
/// the live subscription keeps its registration and its data.
#[test]
fn test_account_updates_other_account_refused() -> Result<(), Error> {
    use crate::accounts::AccountUpdate;
    use crate::subscriptions::SubscriptionItem;

    let (stream, bus) = make_bus();
    let first = account_updates_subscription(&bus, "DU1")?;
    let subscribers = bus.shared_channels.subscribers().len();

    let refused = account_updates_subscription(&bus, "DU2").map(|_| ());
    assert!(
        matches!(&refused, Err(Error::AccountUpdatesInUse { active, requested }) if *active == account("DU1") && *requested == account("DU2")),
        "got: {refused:?}"
    );
    assert_eq!(
        count_frames(&stream.captured(), &account_updates_request("DU2")),
        0,
        "refused request was written"
    );
    assert_eq!(bus.shared_channels.counts.lock().unwrap().live(OutgoingMessages::RequestAccountData), 1);
    assert_eq!(bus.shared_channels.subscribers().len(), subscribers, "refused registration left behind");

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::AccountDownloadEnd as i32,
        &crate::proto::AccountDataEnd {
            account_name: Some("DU1".to_string()),
        },
    ));
    bus.dispatch()?;
    let item = first.next_timeout(TICK).expect("live subscription received nothing")?;
    assert!(matches!(item, SubscriptionItem::Data(AccountUpdate::End)), "got: {item:?}");
    Ok(())
}

/// Once the last subscription of an account ends, another account is accepted.
#[test]
fn test_account_updates_other_account_after_cancel() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let first = account_updates_subscription(&bus, "DU1")?;
    drop(first);
    assert_eq!(count_frames(&stream.captured(), &account_updates_cancel()), 1);

    let _second = account_updates_subscription(&bus, "DU2")?;
    assert_eq!(count_frames(&stream.captured(), &account_updates_request("DU2")), 1);
    assert_eq!(account_updates_slot(&bus), Some(account("DU2")));
    Ok(())
}

/// A reset ends every subscription, so the slot is free on the new session,
/// and a dead handle's drop must not free the new account's slot.
#[test]
fn test_account_updates_slot_restarts_after_reset() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let old = account_updates_subscription(&bus, "DU1")?;
    bus.reset();
    let _new = account_updates_subscription(&bus, "DU2")?;

    drop(old);
    assert_eq!(
        count_frames(&stream.captured(), &account_updates_cancel()),
        0,
        "dead handle wrote a cancel"
    );
    assert_eq!(account_updates_slot(&bus), Some(account("DU2")));
    Ok(())
}

/// `MessageBus::send_message` writes through to the connection.
#[test]
fn test_send_message_writes_through() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let mb: &dyn MessageBus = bus.as_ref();

    mb.send_message(b"global-cancel-bytes")?;

    let captured = stream.captured();
    assert!(captured.windows(b"global-cancel-bytes".len()).any(|w| w == b"global-cancel-bytes"));
    Ok(())
}

/// `MessageBus::create_order_update_subscription` returns `AlreadySubscribed`
/// on duplicate calls while the first is held; dropping it frees the slot at
/// once (see `test_drop_then_recreate_order_update_stream`).
#[test]
fn test_create_order_update_subscription_is_unique() -> Result<(), Error> {
    let (_, bus) = make_bus();
    let mb: &dyn MessageBus = bus.as_ref();

    let _first = mb.create_order_update_subscription()?;
    let err = mb.create_order_update_subscription().expect_err("duplicate fails");
    assert!(matches!(err, Error::AlreadySubscribed), "got: {err:?}");
    Ok(())
}

/// `Subscription::cancel` frees the order-update slot while the handle is
/// still held, once the cleanup thread processes the signal (#911).
#[test]
fn test_order_update_stream_cancel_frees_slot() -> Result<(), Error> {
    let (_, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let first = wrap_subscription::<crate::orders::OrderUpdate>(bus.clone(), bus.create_order_update_subscription()?);
    first.cancel();
    drain_cleanup_signals(&bus);
    let _second = bus.create_order_update_subscription().expect("slot still taken after cancel");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// Shutdown ends a live order-update stream with `Error::Shutdown` and frees
/// the slot. Before #871 the stream got nothing and blocked forever.
#[test]
fn test_order_update_stream_ends_on_shutdown() -> Result<(), Error> {
    let (_, bus) = make_bus();
    let updates = bus.create_order_update_subscription()?;

    bus.ensure_shutdown();

    let item = updates.next_timeout_routed(TICK);
    assert!(matches!(item, Some(RoutedItem::Error(Error::Shutdown))), "got: {item:?}");
    assert!(bus.order_update_stream.lock().unwrap().is_none(), "slot should be released");
    Ok(())
}

/// After shutdown, a new order-update stream is refused rather than
/// returned as a stream nothing will ever end (#871).
#[test]
fn test_create_order_update_subscription_after_shutdown_fails() {
    let (_, bus) = make_bus();
    bus.ensure_shutdown();

    let err = bus.create_order_update_subscription().expect_err("subscribe after shutdown");
    assert!(matches!(err, Error::Shutdown), "got: {err:?}");
}

/// A notice stream opened after shutdown is already at end-of-stream, like
/// the streams shutdown closed (#871).
#[test]
fn test_notice_subscribe_after_shutdown_is_closed() {
    let (_, bus) = make_bus();
    bus.ensure_shutdown();

    let notices = bus.connection.notice_broadcaster.subscribe();
    bus.connection.notice_broadcaster.broadcast(Notice::synthesized(-1, "late".into()));
    let got = notices.recv_timeout(TICK);
    assert!(matches!(got, Err(crossbeam::channel::RecvTimeoutError::Disconnected)), "{got:?}");
}

/// Warning code (2104) bound to a real request_id is delivered as a
/// `RoutedItem::Notice` to the owning subscription — stream stays open.
#[test]
fn test_warning_with_request_id_delivers_notice() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(42);
    let sub = bus.send_request(request_id, &[])?;

    stream.push_inbound(error_frame(request_id.raw(), 2104, FARM_OK_MSG));
    bus.dispatch()?;

    let item = sub.next_timeout_routed(TICK).expect("notice not delivered");
    match item {
        RoutedItem::Notice(notice) => {
            assert_eq!(notice.request_id, Some(request_id.raw()));
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, "Market data farm connection is OK:usfarm");
        }
        other => panic!("expected RoutedItem::Notice, got {other:?}"),
    }

    // Stream stays open: a follow-up send delivers normally.
    stream.push_inbound(body(&format!("89|{request_id}|payload|")));
    bus.dispatch()?;
    let item = sub.next_timeout_routed(TICK).expect("follow-up message lost");
    assert!(matches!(item, RoutedItem::Response(_)), "got: {item:?}");
    Ok(())
}

/// Data advisory (code 10167) bound to a real request_id is informational:
/// TWS proceeds with delayed data, so it is delivered as a `RoutedItem::Notice`
/// and the stream stays open for the follow-up data — not routed as an error
/// that would terminate the subscription.
#[test]
fn test_data_advisory_with_request_id_keeps_stream_open() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(42);
    let sub = bus.send_request(request_id, &[])?;

    let code = 10167; // data advisory: "Displaying delayed market data."
    stream.push_inbound(error_frame(request_id.raw(), code, "Displaying delayed market data."));
    bus.dispatch()?;

    let item = sub.next_timeout_routed(TICK).expect("notice not delivered");
    match item {
        RoutedItem::Notice(notice) => {
            assert_eq!(notice.code, code);
            assert!(notice.is_data_advisory());
        }
        other => panic!("expected RoutedItem::Notice, got {other:?}"),
    }

    // Stream stays open: the delayed data the advisory promised arrives.
    stream.push_inbound(body(&format!("89|{request_id}|payload|")));
    bus.dispatch()?;
    let item = sub.next_timeout_routed(TICK).expect("delayed data lost");
    assert!(matches!(item, RoutedItem::Response(_)), "got: {item:?}");
    Ok(())
}

/// Hard error (code 200) bound to a real request_id is delivered as a
/// `RoutedItem::Error` to the owning subscription. The subscription
/// terminates: subsequent reads return `None`.
#[test]
fn test_hard_error_with_request_id_terminates_subscription() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(42);
    let sub = bus.send_request(request_id, &[])?;

    stream.push_inbound(error_frame(request_id.raw(), 200, "No security definition found"));
    bus.dispatch()?;

    let item = sub.next_timeout_routed(TICK).expect("error not delivered");
    match item {
        RoutedItem::Error(Error::Notice(notice)) => {
            assert_eq!(notice.request_id, Some(request_id.raw()));
            assert_eq!(notice.code, 200);
            assert_eq!(notice.message, "No security definition found");
        }
        other => panic!("expected RoutedItem::Error(Notice), got {other:?}"),
    }
    Ok(())
}

/// Warning with `UNSPECIFIED_REQUEST_ID` has no owner — log only, no channel
/// write. An in-flight subscription should not see anything.
#[test]
fn test_warning_with_unspecified_id_is_log_only() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_request(RequestId::nth(42), &[])?;

    stream.push_inbound(error_frame(-1, 2104, FARM_OK_MSG));
    bus.dispatch()?;

    assert!(sub.try_next_routed().is_none(), "unrouted notice must not be delivered to a subscription");
    Ok(())
}

/// Request-less hard error (id = -1) is uncorrelatable, so it fails every
/// in-flight *one-shot* shared request fast (`RequestIds` here) while leaving
/// *streaming* shared requests (`RequestPositions`) untouched — and still fans
/// out to the global notice stream. Regression for #694 (callers hung forever).
#[test]
fn test_request_less_hard_error_fails_one_shot_and_spares_stream() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let notice_stream = bus.notice_subscribe();
    let one_shot = bus.send_shared_request(OutgoingMessages::RequestIds, &[])?;
    let streaming = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;

    // 321 "read-only mode" is the live-reproduced case; non-warning, id = -1.
    stream.push_inbound(error_frame(-1, 321, READ_ONLY_MSG));
    bus.dispatch()?;

    // One-shot caller fails fast with the real error instead of hanging. Read via
    // the legacy `next()` projection — the same path `next_valid_order_id` and the
    // `one_shot_shared` helper consume — so a `Some(Err(..))` surfaces to callers.
    match one_shot.next_timeout(TICK).expect("one-shot got no error") {
        Err(Error::Notice(notice)) => {
            assert_eq!(notice.code, 321);
            assert_eq!(notice.message, READ_ONLY_MSG);
        }
        other => panic!("expected Err(Notice), got {other:?}"),
    }
    // Streaming shared subscription is not terminated by the unrelated error.
    assert!(
        streaming.try_next_routed().is_none(),
        "streaming shared sub must not receive the request-less error"
    );
    // Global notice stream still observes it.
    let notice = notice_stream.next_timeout(TICK).expect("notice stream missed hard error");
    assert_eq!(notice.request_id, None);
    assert_eq!(notice.code, 321);
    Ok(())
}

/// A request-less *warning* stays notice-only: it must not fail an in-flight
/// one-shot shared request (only non-warning hard errors trip fail-fast).
#[test]
fn test_request_less_warning_does_not_fail_one_shot() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let one_shot = bus.send_shared_request(OutgoingMessages::RequestIds, &[])?;

    stream.push_inbound(error_frame(-1, 2104, "Market data farm connection is OK:usfarm"));
    bus.dispatch()?;

    assert!(one_shot.try_next_routed().is_none(), "warning must not fail a one-shot shared request");
    Ok(())
}

/// A request-less *system message* (1102, connectivity restored with data
/// maintained) reports a connection-wide state change, not a failed request.
/// It must reach the notice stream without failing in-flight one-shot shared
/// requests - `managed_accounts`, `server_time`, `next_valid_order_id`.
#[test]
fn test_request_less_system_message_does_not_fail_one_shot() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let notice_stream = bus.notice_subscribe();
    let one_shot = bus.send_shared_request(OutgoingMessages::RequestIds, &[])?;

    let code = crate::messages::CONNECTIVITY_RESTORED_DATA_MAINTAINED_CODE;
    stream.push_inbound(error_frame(-1, code, CONNECTIVITY_RESTORED_MSG));
    bus.dispatch()?;

    assert!(
        one_shot.try_next_routed().is_none(),
        "system message must not fail a one-shot shared request"
    );

    let notice = notice_stream.next_timeout(TICK).expect("notice stream missed system message");
    assert_eq!(notice.code, code);
    assert!(notice.is_system_message());
    Ok(())
}

/// A request-less error fanned out while no one-shot request was in flight
/// reaches nobody: a one-shot request made afterwards has a queue of its own
/// and reads only its own response.
#[test]
fn test_one_shot_shared_request_does_not_see_earlier_error() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    stream.push_inbound(error_frame(-1, 321, READ_ONLY_MSG));
    bus.dispatch()?;

    let one_shot = bus.send_shared_request(OutgoingMessages::RequestIds, &[])?;
    assert!(
        one_shot.try_next_routed().is_none(),
        "error from before the request must not be delivered"
    );

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::NextValidId as i32,
        &crate::proto::NextValidId { order_id: Some(90) },
    ));
    bus.dispatch()?;

    let message = one_shot.next_timeout(TICK).expect("one-shot response missing")?;
    assert_eq!(message.message_type(), crate::messages::IncomingMessages::NextValidId);
    Ok(())
}

/// Two one-shot requests of the same type in flight at once each read the
/// reply: neither takes it from the other.
#[test]
fn test_concurrent_one_shot_shared_requests_each_receive_reply() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let first = bus.send_shared_request(OutgoingMessages::RequestCurrentTime, &[])?;
    let second = bus.send_shared_request(OutgoingMessages::RequestCurrentTime, &[])?;

    stream.push_inbound(body("49|1|1700000000|"));
    bus.dispatch()?;

    for (name, sub) in [("first", &first), ("second", &second)] {
        let message = sub.next_timeout(TICK).unwrap_or_else(|| panic!("{name} one-shot got no reply"))?;
        assert_eq!(message.peek_int(2)?, 1_700_000_000, "{name}");
        assert!(sub.try_next_routed().is_none(), "{name} one-shot received more than the reply");
    }
    Ok(())
}

/// Every live subscription of a shared streaming type receives every frame,
/// as with the async broadcast: two `RequestPositions` subscriptions both see
/// each `Position` and `PositionEnd`.
#[test]
fn test_live_shared_subscriptions_each_receive_every_frame() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let first = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;
    let second = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::Position as i32,
        &crate::proto::Position {
            account: Some("DU123".to_string()),
            ..Default::default()
        },
    ));
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::PositionEnd as i32,
        &crate::proto::PositionEnd {},
    ));
    bus.dispatch()?;
    bus.dispatch()?;

    for (name, sub) in [("first", &first), ("second", &second)] {
        let position = sub.next_timeout(TICK).unwrap_or_else(|| panic!("{name} got no Position"))?;
        assert_eq!(position.message_type(), crate::messages::IncomingMessages::Position, "{name}");
        let end = sub.next_timeout(TICK).unwrap_or_else(|| panic!("{name} got no PositionEnd"))?;
        assert_eq!(end.message_type(), crate::messages::IncomingMessages::PositionEnd, "{name}");
        assert!(sub.try_next_routed().is_none(), "{name} received a frame twice");
    }
    Ok(())
}

/// Frames dispatched before a subscription existed never reach it: a
/// `RequestPositions` subscription made after a `PositionEnd` was delivered to
/// an earlier one starts empty, and the earlier one keeps its frame.
#[test]
fn test_later_shared_subscription_does_not_see_earlier_frames() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let earlier = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::PositionEnd as i32,
        &crate::proto::PositionEnd {},
    ));
    bus.dispatch()?;

    let later = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;
    assert!(later.try_next_routed().is_none(), "frame from before the request was delivered");

    let message = earlier.next_timeout(TICK).expect("earlier subscription lost its frame")?;
    assert_eq!(message.message_type(), crate::messages::IncomingMessages::PositionEnd);
    Ok(())
}

/// A reset with no subscription in flight leaves nothing behind: a streaming
/// shared request made after it reads its own responses, not a stale
/// `ConnectionReset`.
#[test]
fn test_later_shared_subscription_does_not_see_earlier_reset() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    bus.reset();

    let sub = bus.send_shared_request(OutgoingMessages::RequestOpenOrders, &[])?;
    assert!(sub.try_next_routed().is_none(), "reset from before the request was delivered");

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OpenOrderEnd as i32,
        &crate::proto::OpenOrdersEnd {},
    ));
    bus.dispatch()?;

    let message = sub.next_timeout(TICK).expect("response missing")?;
    assert_eq!(message.message_type(), crate::messages::IncomingMessages::OpenOrderEnd);
    Ok(())
}

/// The registered shared subscriptions matching `predicate`.
fn shared_subscriber_count<S: Stream>(bus: &TcpMessageBus<S>, predicate: impl Fn(&SharedSubscriber) -> bool) -> usize {
    bus.shared_channels.subscribers.lock().unwrap().iter().filter(|s| predicate(s)).count()
}

/// Dropping a shared subscription removes its registration, so no response
/// type it was registered under delivers to it any more. `RequestOpenOrders`
/// maps to three response types; one drop signal clears all three.
#[test]
fn test_dropped_shared_subscription_is_unregistered() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let sub = bus.send_shared_request(OutgoingMessages::RequestOpenOrders, &[])?;
    let survivor = bus.send_shared_request(OutgoingMessages::RequestOpenOrders, &[])?;
    let registered = |bus: &TcpMessageBus<MemoryStream>| shared_subscriber_count(bus, |s| s.request == OutgoingMessages::RequestOpenOrders);
    assert_eq!(registered(&bus), 2);

    drop(sub);
    drain_cleanup_signals(&bus);
    assert_eq!(registered(&bus), 1, "dropped subscription still registered");

    // The survivor is the one still registered: every response type of the
    // mapping still delivers to it, once.
    for message_type in shared_channel_configuration::response_types(OutgoingMessages::RequestOpenOrders).unwrap() {
        stream.push_inbound(match message_type {
            crate::messages::IncomingMessages::OpenOrder => binary_proto(
                *message_type as i32,
                &crate::proto::OpenOrder {
                    order_id: Some(7),
                    ..Default::default()
                },
            ),
            crate::messages::IncomingMessages::OrderStatus => binary_proto(
                *message_type as i32,
                &crate::proto::OrderStatus {
                    order_id: Some(7),
                    ..Default::default()
                },
            ),
            crate::messages::IncomingMessages::OpenOrderEnd => binary_proto(*message_type as i32, &crate::proto::OpenOrdersEnd {}),
            other => panic!("unexpected response type {other:?} in the RequestOpenOrders mapping"),
        });
        bus.dispatch()?;
        let message = survivor
            .next_timeout(TICK)
            .unwrap_or_else(|| panic!("survivor got no {message_type:?}"))?;
        assert_eq!(message.message_type(), *message_type);
    }
    assert!(survivor.try_next_routed().is_none(), "survivor received a frame twice");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// Until the cleanup thread processes the drop signal, a send to the dropped
/// subscription's queue fails; the dispatcher removes the registration then,
/// so nothing accumulates and a queue with no reader is not left registered.
#[test]
fn test_send_to_dropped_shared_subscription_unregisters_it() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let sub = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;
    drop(sub);
    assert_eq!(
        shared_subscriber_count(&bus, |_| true),
        1,
        "no cleanup thread: the drop signal is still queued"
    );

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::PositionEnd as i32,
        &crate::proto::PositionEnd {},
    ));
    bus.dispatch()?;

    assert_eq!(shared_subscriber_count(&bus, |_| true), 0, "send to a dropped queue must unregister it");
    Ok(())
}

/// `cancel()` on a shared subscription delivers `Error::Cancelled` to its own
/// queue, so a thread blocked in `next()` on the same handle returns, and
/// unregisters it: a frame of its type dispatched afterwards is not queued on
/// the cancelled handle, while a subscription made afterwards receives it.
#[test]
fn test_shared_subscription_cancel_notifies_and_unregisters() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let sub = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;
    sub.cancel();
    drain_cleanup_signals(&bus);
    assert_eq!(shared_subscriber_count(&bus, |_| true), 0, "cancelled subscription still registered");

    let later = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::PositionEnd as i32,
        &crate::proto::PositionEnd {},
    ));
    bus.dispatch()?;

    let resp = sub.next_timeout(TICK).expect("cancelled shared subscription got no notification");
    assert!(matches!(resp, Err(Error::Cancelled)), "{resp:?}");
    assert!(sub.try_next_routed().is_none(), "frame queued on a cancelled handle");
    let message = later.next_timeout(TICK).expect("later subscription got nothing")?;
    assert_eq!(message.message_type(), crate::messages::IncomingMessages::PositionEnd);

    drop(sub);
    drain_cleanup_signals(&bus);
    assert_eq!(
        shared_subscriber_count(&bus, |_| true),
        1,
        "drop after cancel removed the wrong registration"
    );

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// `Subscription::cancel()` on a shared stream unregisters the handle through
/// the same path, so a cancelled `positions()` handle kept in a struct stops
/// collecting frames while the count is released for the cancel.
#[test]
fn test_subscription_cancel_unregisters_shared_handle() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let handle = bus.start_cleanup_thread();
    let cancel = positions_cancel();

    let sub = positions_subscription(&bus)?;
    sub.cancel();
    drain_cleanup_signals(&bus);
    assert_eq!(count_frames(&stream.captured(), &cancel), 1);
    assert_eq!(shared_subscriber_count(&bus, |_| true), 0, "cancelled subscription still registered");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// `reset` unregisters every shared subscription after failing it once (not
/// once per response type): a frame of the type dispatched afterwards is not
/// queued on the dead handle, and a resubscription made after the reset
/// receives it.
#[test]
fn test_reset_unregisters_shared_subscriptions() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    let old = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;
    bus.reset();
    assert_eq!(shared_subscriber_count(&bus, |_| true), 0, "reset left a registration behind");

    let new = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::PositionEnd as i32,
        &crate::proto::PositionEnd {},
    ));
    bus.dispatch()?;

    let resp = old.next_timeout(TICK).expect("dead handle got no reset");
    assert!(matches!(resp, Err(Error::ConnectionReset)), "{resp:?}");
    assert!(
        old.try_next_routed().is_none(),
        "dead handle got a second reset or a frame; a reset is delivered once, not once per response type"
    );
    let message = new.next_timeout(TICK).expect("resubscription got nothing")?;
    assert_eq!(message.message_type(), crate::messages::IncomingMessages::PositionEnd);
    Ok(())
}

/// A notice bound to an id below the request floor is an order's: it goes to
/// the order subscription for that id.
#[test]
fn test_warning_with_order_id_routes_to_order_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_order_request(OrderId::from(7), &[])?;

    stream.push_inbound(error_frame(7, 2104, "Order warning"));
    bus.dispatch()?;

    let item = sub.next_timeout_routed(TICK).expect("order notice not delivered");
    match item {
        RoutedItem::Notice(notice) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, "Order warning");
        }
        other => panic!("expected RoutedItem::Notice, got {other:?}"),
    }
    Ok(())
}

// ---- end-to-end Subscription consumer tests for Notice delivery ----
//
// Mirror the dispatcher routing tests above, one layer up: drive bytes through
// the production dispatcher and assert via the public `Subscription<T>::next()`
// / `iter_data()` API that the consumer sees `SubscriptionItem::Notice` /
// `Err(_)` / `None` as expected.

const CONNECTIVITY_RESTORED_MSG: &str = "Connectivity between IB and TWS has been restored - data maintained.";
const READ_ONLY_MSG: &str = "The API interface is currently in Read-Only mode.";

fn wrap_subscription<T: crate::subscriptions::StreamDecoder<T>>(
    bus: Arc<TcpMessageBus<MemoryStream>>,
    internal: InternalSubscription,
) -> crate::subscriptions::sync::Subscription<T> {
    crate::subscriptions::sync::Subscription::new(bus, internal, crate::subscriptions::DecoderContext::default())
}

type NoticeFixture = (
    MemoryStream,
    Arc<TcpMessageBus<MemoryStream>>,
    crate::subscriptions::sync::Subscription<NoticeTestData>,
);

fn make_request_subscription(request_id: RequestId) -> Result<NoticeFixture, Error> {
    let (stream, bus) = make_bus();
    let internal = bus.send_request(request_id, &[])?;
    let sub = wrap_subscription(bus.clone(), internal);
    Ok((stream, bus, sub))
}

fn make_order_subscription(order_id: OrderId) -> Result<NoticeFixture, Error> {
    let (stream, bus) = make_bus();
    let internal = bus.send_order_request(order_id, &[])?;
    let sub = wrap_subscription(bus.clone(), internal);
    Ok((stream, bus, sub))
}

/// Code 2104 on a request id surfaces as `SubscriptionItem::Notice` without
/// terminating; a follow-up data message arrives normally on the same stream.
#[test]
fn test_subscription_notice_delivery_request_keyed() -> Result<(), Error> {
    use crate::subscriptions::SubscriptionItem;

    let (stream, bus, subscription) = make_request_subscription(RequestId::nth(42))?;

    stream.push_inbound(farm_ok_frame_42());
    bus.dispatch()?;

    match subscription.next_timeout(TICK) {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, FARM_OK_MSG);
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }

    stream.push_inbound(body(&format!("89|{}|payload|", RequestId::nth(42))));
    bus.dispatch()?;
    match subscription.next_timeout(TICK) {
        Some(Ok(SubscriptionItem::Data(_))) => {}
        other => panic!("expected SubscriptionItem::Data, got {other:?}"),
    }
    Ok(())
}

/// Partial entitlement can precede delayed Greeks on the same request.
#[test]
fn test_subscription_10091_preserves_later_option_computation() -> Result<(), Error> {
    use crate::contracts::tick_types::TickType;
    use crate::market_data::realtime::TickTypes;
    use crate::messages::IncomingMessages;
    use crate::subscriptions::SubscriptionItem;
    use crate::testdata::builders::{market_data::tick_option_computation, ResponseProtoEncoder};

    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(42);
    let internal = bus.send_request(request_id, &[])?;
    let subscription = wrap_subscription::<TickTypes>(bus.clone(), internal);
    let computation = tick_option_computation()
        .request_id(request_id.raw())
        .tick_type(TickType::DelayedModelOption as i32)
        .tick_attrib(0)
        .delta(0.5)
        .to_proto();

    // Both frames are dispatched before polling: the error must not hide
    // an already-queued computation on the same request.
    stream.push_inbound(error_frame(request_id.raw(), 10091, "Synthetic partial-entitlement advisory"));
    stream.push_inbound(binary_proto(IncomingMessages::TickOptionComputation as i32, &computation));
    bus.dispatch()?;
    bus.dispatch()?;

    match subscription.next_timeout(TICK) {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert_eq!(notice.request_id, Some(request_id.raw()));
            assert_eq!(notice.code, 10091);
            assert_eq!(notice.message, "Synthetic partial-entitlement advisory");
            assert!(notice.is_data_advisory());
        }
        other => panic!("expected nonterminal 10091 notice, got {other:?}"),
    }
    match subscription.next_timeout(TICK) {
        Some(Ok(SubscriptionItem::Data(TickTypes::OptionComputation(greeks)))) => {
            assert_eq!(greeks.field, TickType::DelayedModelOption);
            assert_eq!(greeks.tick_attribute, Some(0));
            assert_eq!(greeks.delta, Some(0.5));
            assert_eq!(greeks.implied_volatility, None);
        }
        other => panic!("option computation after 10091 lost: {other:?}"),
    }
    Ok(())
}

/// A depth-book reset (317) precedes the rows that rebuild it on the same
/// request. It must not end the depth stream (#806), and arrives as
/// `MarketDepths::Reset` so `iter_data()` keeps it (#899).
#[test]
fn test_subscription_317_yields_reset_then_rows() -> Result<(), Error> {
    use crate::market_data::realtime::MarketDepths;
    use crate::messages::IncomingMessages;
    use crate::testdata::builders::{market_data::market_depth_response, ResponseProtoEncoder};

    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(42);
    let internal = bus.send_request(request_id, &[])?;
    let subscription = wrap_subscription::<MarketDepths>(bus.clone(), internal);
    let row = market_depth_response()
        .request_id(request_id.raw())
        .position(0)
        .operation(0)
        .side(1)
        .price(101.5)
        .size(3.0)
        .to_proto();

    // Both frames are dispatched before polling: the reset must not hide the
    // first row of the rebuilt book.
    stream.push_inbound(error_frame(
        request_id.raw(),
        317,
        "Market depth data has been RESET. Please empty deep book contents before applying any new entries.",
    ));
    stream.push_inbound(binary_proto(IncomingMessages::MarketDepth as i32, &row));
    bus.dispatch()?;
    bus.dispatch()?;

    let mut depths = subscription.timeout_iter_data(TICK);
    match depths.next() {
        Some(Ok(MarketDepths::Reset)) => {}
        other => panic!("expected MarketDepths::Reset, got {other:?}"),
    }
    match depths.next() {
        Some(Ok(MarketDepths::MarketDepth(depth))) => {
            assert_eq!(depth.position, 0);
            assert_eq!(depth.operation, 0);
            assert_eq!(depth.side, 1);
            assert_eq!(depth.price, 101.5);
            assert_eq!(depth.size, 3.0);
        }
        other => panic!("market depth row after 317 lost: {other:?}"),
    }
    Ok(())
}

/// Hard error (code 200) surfaces as `Some(Err(_))`; subsequent reads return `None`.
#[test]
fn test_subscription_hard_error_terminates_stream() -> Result<(), Error> {
    let (stream, bus, subscription) = make_request_subscription(RequestId::nth(42))?;

    let request_id = RequestId::nth(42);
    stream.push_inbound(error_frame(request_id.raw(), 200, "No security definition found"));
    stream.push_inbound(body(&format!("89|{request_id}|payload|")));
    bus.dispatch()?;
    bus.dispatch()?;

    match subscription.next_timeout(TICK) {
        Some(Err(Error::Notice(notice))) => {
            assert_eq!(notice.code, 200);
            assert_eq!(notice.message, "No security definition found");
        }
        other => panic!("expected Some(Err(Error::Notice)), got {other:?}"),
    }

    assert!(subscription.next_timeout(TICK).is_none(), "terminal error must hide even queued data");
    Ok(())
}

/// Order-keyed notice: an id below the request floor reaches the order subscription.
#[test]
fn test_subscription_notice_delivery_order_keyed() -> Result<(), Error> {
    use crate::subscriptions::SubscriptionItem;

    let (stream, bus, subscription) = make_order_subscription(OrderId::from(7))?;

    stream.push_inbound(error_frame(7, 2109, "Outside RTH order warning"));
    bus.dispatch()?;

    match subscription.next_timeout(TICK) {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert_eq!(notice.code, 2109);
            assert_eq!(notice.message, "Outside RTH order warning");
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }
    Ok(())
}

/// Unrouted notice (UNSPECIFIED request_id) is log-only; no channel write.
#[test]
fn test_subscription_unspecified_notice_not_delivered() -> Result<(), Error> {
    let (stream, bus, subscription) = make_request_subscription(RequestId::nth(42))?;

    stream.push_inbound(farm_ok_frame_unrouted());
    bus.dispatch()?;

    assert!(
        subscription.try_next().is_none(),
        "unrouted notice must not be delivered to a subscription"
    );
    Ok(())
}

/// `iter_data()` filters `SubscriptionItem::Notice` and yields only data.
#[test]
fn test_subscription_iter_data_filters_notices() -> Result<(), Error> {
    let (stream, bus, subscription) = make_request_subscription(RequestId::nth(42))?;

    let request_id = RequestId::nth(42);
    stream.push_inbound(body(&format!("89|{request_id}|first|")));
    stream.push_inbound(farm_ok_frame_42());
    stream.push_inbound(body(&format!("89|{request_id}|second|")));
    for _ in 0..3 {
        bus.dispatch()?;
    }

    let mut iter = subscription.timeout_iter_data(TICK);
    assert!(matches!(iter.next(), Some(Ok(NoticeTestData))), "first data missing");
    assert!(matches!(iter.next(), Some(Ok(NoticeTestData))), "second data missing");
    assert!(iter.next().is_none(), "iterator should drain after both data items");
    Ok(())
}

// ---- end-to-end NoticeStream tests (PR 5) ----
//
// Drive bytes through the production dispatcher and assert that unrouted
// notices reach `MessageBus::notice_subscribe()` consumers, while routed
// notices stay with their owning subscription.

/// An unrouted warning (`request_id == -1`, code 2104) is delivered to a
/// `notice_stream` subscriber.
#[test]
fn test_notice_stream_receives_unrouted_warning() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let notice_stream = bus.notice_subscribe();

    stream.push_inbound(farm_ok_frame_unrouted());
    bus.dispatch()?;

    let notice = notice_stream.next_timeout(TICK).expect("notice not delivered");
    assert_eq!(notice.code, 2104);
    assert_eq!(notice.message, FARM_OK_MSG);
    Ok(())
}

/// Two `notice_subscribe` calls each receive every unrouted notice.
#[test]
fn test_notice_stream_fans_out_to_multiple_subscribers() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let s1 = bus.notice_subscribe();
    let s2 = bus.notice_subscribe();

    stream.push_inbound(farm_ok_frame_unrouted());
    bus.dispatch()?;

    let n1 = s1.next_timeout(TICK).expect("subscriber 1 missed notice");
    let n2 = s2.next_timeout(TICK).expect("subscriber 2 missed notice");
    assert_eq!(n1.code, 2104);
    assert_eq!(n2.code, 2104);
    Ok(())
}

/// Severity-agnostic: an unrouted hard error (e.g. code 504) also fans out.
#[test]
fn test_notice_stream_receives_unrouted_hard_error() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let notice_stream = bus.notice_subscribe();

    // code 504 — "Not connected" — is non-warning.
    stream.push_inbound(error_frame(-1, 504, "Not connected"));
    bus.dispatch()?;

    let notice = notice_stream.next_timeout(TICK).expect("hard-error notice missed");
    assert_eq!(notice.code, 504);
    Ok(())
}

/// A routed notice (real `request_id`) goes to the owning subscription, NOT
/// to the global notice stream.
#[test]
fn test_notice_stream_skips_routed_notices() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let notice_stream = bus.notice_subscribe();
    let request_sub = bus.send_request(RequestId::nth(42), &[])?;

    stream.push_inbound(farm_ok_frame_42());
    bus.dispatch()?;

    // Routed to the owner.
    assert!(request_sub.try_next_routed().is_some(), "owner subscription missed notice");
    // NOT delivered to the global stream.
    assert!(notice_stream.try_next().is_none(), "routed notice leaked to global stream");
    Ok(())
}

/// Late subscribers don't see prior notices (no replay buffer).
#[test]
fn test_notice_stream_late_subscriber_misses_prior() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    stream.push_inbound(farm_ok_frame_unrouted());
    bus.dispatch()?;

    // Subscribe AFTER the notice was broadcast.
    let late = bus.notice_subscribe();
    assert!(late.try_next().is_none(), "late subscriber should not see prior notices");
    Ok(())
}

/// Shutdown closes the broadcaster; receivers see channel-closed via `next() == None`.
#[test]
fn test_notice_stream_closes_on_shutdown() -> Result<(), Error> {
    let (_stream, bus) = make_bus();
    let notice_stream = bus.notice_subscribe();

    bus.ensure_shutdown();
    assert!(notice_stream.next().is_none(), "stream should close on shutdown");
    Ok(())
}

// ---- order-routing strategy tests ----
//
// `process_orders` dispatches by `order_routing_strategy(message_type)`. Each
// strategy has a different fallback order (order_id → request_id, by execution_id,
// shared-only). For each strategy we cover the positive route, every fallback,
// and the orphan-fallthrough.

#[test]
fn test_execution_data_routes_to_order_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_order_request(OrderId::from(7), &[])?;

    stream.push_inbound(execution_data_frame(RequestId::nth(99).raw(), 7, "exec-1"));
    bus.dispatch()?;

    let msg = sub.next_timeout(TICK).expect("order sub got no message")?;
    assert_eq!(msg.message_type(), crate::messages::IncomingMessages::ExecutionData);
    assert_eq!(msg.order_id(), Some(7));
    Ok(())
}

#[test]
fn test_execution_data_falls_back_to_request_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(99);
    let sub = bus.send_request(request_id, &[])?;

    stream.push_inbound(execution_data_frame(request_id.raw(), 7, "exec-1"));
    bus.dispatch()?;

    let msg = sub.next_timeout(TICK).expect("request sub got no message")?;
    assert_eq!(msg.request_id(), Some(request_id.raw()));
    Ok(())
}

#[test]
fn test_execution_data_end_routes_to_order_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_order_request(OrderId::from(7), &[])?;

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::ExecutionDataEnd as i32,
        &crate::proto::ExecutionDetailsEnd { req_id: Some(7) },
    ));
    bus.dispatch()?;

    let msg = sub.next_timeout(TICK).expect("order sub got no end")?;
    assert_eq!(msg.message_type(), crate::messages::IncomingMessages::ExecutionDataEnd);
    Ok(())
}

/// ExecutionDataEnd's `req_id` doubles as the order_id key for the router; a
/// request-range id misses the order channel and falls back to the request
/// channel.
#[test]
fn test_execution_data_end_falls_back_to_request_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let request_id = RequestId::nth(7);
    let sub = bus.send_request(request_id, &[])?;

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::ExecutionDataEnd as i32,
        &crate::proto::ExecutionDetailsEnd {
            req_id: Some(request_id.raw()),
        },
    ));
    bus.dispatch()?;

    let msg = sub.next_timeout(TICK).expect("request sub got no end")?;
    assert_eq!(msg.message_type(), crate::messages::IncomingMessages::ExecutionDataEnd);
    Ok(())
}

/// `ByExecutionId`: the prior ExecutionData stores `exec-abc → order_id 7`'s
/// sender, and the CommissionsReport rides that mapping back to the same sub.
#[test]
fn test_commission_report_routes_via_execution_id_mapping() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_order_request(OrderId::from(7), &[])?;

    stream.push_inbound(execution_data_frame(RequestId::nth(99).raw(), 7, "exec-abc"));
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::CommissionsReport as i32,
        &crate::proto::CommissionAndFeesReport {
            exec_id: Some("exec-abc".into()),
            ..Default::default()
        },
    ));

    bus.dispatch()?;
    bus.dispatch()?;

    let exec_msg = sub.next_timeout(TICK).expect("exec data missing")?;
    assert_eq!(exec_msg.message_type(), crate::messages::IncomingMessages::ExecutionData);

    let commission = sub.next_timeout(TICK).expect("commission report missing")?;
    assert_eq!(commission.message_type(), crate::messages::IncomingMessages::CommissionsReport);
    Ok(())
}

#[test]
fn test_completed_order_routes_to_shared_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_shared_request(OutgoingMessages::RequestCompletedOrders, &[])?;

    stream.push_inbound(body("101|265598|AAPL|STK|"));
    bus.dispatch()?;

    let msg = sub.next_timeout(TICK).expect("completed orders got no message")?;
    assert_eq!(msg.peek_int(0)?, 101);
    Ok(())
}

#[test]
fn test_completed_orders_end_routes_to_shared_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_shared_request(OutgoingMessages::RequestCompletedOrders, &[])?;

    stream.push_inbound(body("102|"));
    bus.dispatch()?;

    let msg = sub.next_timeout(TICK).expect("completed orders end got no message")?;
    assert_eq!(msg.peek_int(0)?, 102);
    Ok(())
}

/// `send_order_update` fan-out: an OpenOrder reaches both an order subscription
/// and the order-update stream when both are registered for the same order.
#[test]
fn test_order_update_stream_receives_open_order() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let order_sub = bus.send_order_request(OrderId::from(42), &[])?;
    let stream_sub = bus.create_order_update_subscription()?;

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OpenOrder as i32,
        &crate::proto::OpenOrder {
            order_id: Some(42),
            ..Default::default()
        },
    ));
    bus.dispatch()?;

    assert!(order_sub.next_timeout(TICK).is_some(), "order sub missed open order");
    assert!(stream_sub.next_timeout(TICK).is_some(), "update stream missed open order");
    Ok(())
}

/// A targeted hard error reaches the order-update stream as a notice so the
/// stream can continue with later order updates.
#[test]
fn test_order_update_stream_receives_order_error_as_notice() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let stream_sub = bus.create_order_update_subscription()?;

    stream.push_inbound(error_frame(42, 201, "Order rejected"));
    bus.dispatch()?;

    match stream_sub.next_timeout_routed(TICK).expect("update stream missed order error") {
        RoutedItem::Notice(notice) => {
            assert_eq!(notice.request_id, Some(42));
            assert_eq!(notice.code, 201);
            assert_eq!(notice.message, "Order rejected");
        }
        other => panic!("expected RoutedItem::Notice, received {other:?}"),
    }

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OpenOrder as i32,
        &crate::proto::OpenOrder {
            order_id: Some(42),
            ..Default::default()
        },
    ));
    bus.dispatch()?;

    assert!(
        matches!(stream_sub.next_timeout_routed(TICK), Some(RoutedItem::Response(_))),
        "update stream did not remain open"
    );
    Ok(())
}

/// An error with a request-range id stays on that request's subscription:
/// the order-update stream must not receive a copy.
#[test]
fn test_order_update_stream_skips_data_request_error() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let stream_sub = bus.create_order_update_subscription()?;
    let request_id = RequestId::nth(42);
    let sub = bus.send_request(request_id, &[])?;

    stream.push_inbound(error_frame(request_id.raw(), 200, "No security definition found"));
    bus.dispatch()?;

    let item = sub.next_timeout_routed(TICK).expect("error not delivered");
    assert!(matches!(item, RoutedItem::Error(Error::Notice(_))), "got: {item:?}");
    assert!(
        stream_sub.next_timeout_routed(TICK).is_none(),
        "order-update stream must not receive a data-request error"
    );
    Ok(())
}

/// Drop signals exercise `clean_request` / `clean_order` / `clear_order_update_stream`.
/// The cleanup thread is signal-driven; `drain_cleanup_signals` bounds the
/// wait without adding an ack channel to production code.
#[test]
fn test_cleanup_thread_processes_drop_signals() -> Result<(), Error> {
    let (_, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let (request_id, order_id) = (RequestId::nth(42), OrderId::from(99));
    let req = bus.send_request(request_id, &[])?;
    let order = bus.send_order_request(order_id, &[])?;
    let stream_sub = bus.create_order_update_subscription()?;

    drop(req);
    drop(order);
    drop(stream_sub);

    drain_cleanup_signals(&bus);

    assert!(!bus.requests.contains(&request_id), "request not cleaned");
    assert!(!bus.orders.contains(&order_id), "order not cleaned");
    assert!(bus.order_update_stream.lock().unwrap().is_none(), "order update stream not cleared");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// One warning per watermark multiple — not one per message, not never.
#[test]
fn backlog_watermark_fires_on_multiples_only() {
    assert!(!backlog_watermark_crossed(0));
    assert!(!backlog_watermark_crossed(1));
    assert!(!backlog_watermark_crossed(BACKLOG_WATERMARK - 1));
    assert!(backlog_watermark_crossed(BACKLOG_WATERMARK));
    assert!(!backlog_watermark_crossed(BACKLOG_WATERMARK + 1));
    assert!(backlog_watermark_crossed(2 * BACKLOG_WATERMARK));
}

/// Queue a marker drop signal behind everything already queued and wait until
/// the cleanup thread has processed it. Signals are handled FIFO by a single
/// thread, so once the marker's registration is gone, every signal sent
/// before it has been handled too.
fn drain_cleanup_signals(bus: &Arc<TcpMessageBus<MemoryStream>>) {
    const MARKER_REQUEST_ID: RequestId = RequestId::nth(987_654);
    let marker = bus.send_request(MARKER_REQUEST_ID, &[]).expect("marker request failed");
    drop(marker);

    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if !bus.requests.contains(&MARKER_REQUEST_ID) {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("cleanup thread did not process the marker signal");
}

/// #893: drop then immediately recreate the order update stream. The dead
/// registration is replaced without waiting for the cleanup thread (which
/// used to return `AlreadySubscribed` here), and the old stream's stale signal
/// must not clear the replacement.
#[test]
fn test_drop_then_recreate_order_update_stream() -> Result<(), Error> {
    let (_, bus) = make_bus();

    // Cleanup thread not running yet: the old stream's signal stays queued.
    drop(bus.create_order_update_subscription()?);
    let replacement = bus.create_order_update_subscription().expect("immediate recreation failed");

    let handle = bus.start_cleanup_thread();
    drain_cleanup_signals(&bus);
    assert!(
        bus.send_order_update_item(Error::Cancelled.into()),
        "stale cleanup cleared the replacement"
    );
    let item = replacement.next_timeout_routed(TICK);
    assert!(matches!(item, Some(RoutedItem::Error(Error::Cancelled))), "{item:?}");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// #932: cancel then recreate the order update stream, the old handle still
/// held. `cancel()` releases the lease, so the registration is replaced
/// without waiting for the cleanup thread; the old stream sends one signal,
/// none at drop, and that stale signal must not clear the replacement.
#[test]
fn test_cancel_then_recreate_order_update_stream() -> Result<(), Error> {
    let (_, bus) = make_bus();

    // Cleanup thread not running yet: the old stream's signal stays queued.
    let cancelled = bus.create_order_update_subscription()?;
    cancelled.cancel();
    let replacement = bus.create_order_update_subscription().expect("recreation after cancel failed");
    drop(cancelled);
    assert_eq!(bus.signals_recv.len(), 1, "cancel then drop should send one signal");

    let handle = bus.start_cleanup_thread();
    drain_cleanup_signals(&bus);
    assert!(
        bus.send_order_update_item(Error::Cancelled.into()),
        "stale cleanup cleared the replacement"
    );
    let item = replacement.next_timeout_routed(TICK);
    assert!(matches!(item, Some(RoutedItem::Error(Error::Cancelled))), "{item:?}");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// Regression test for #773: dropping an old order subscription must not
/// unregister a newer subscription under the same order id (place then cancel
/// on one id). The stale signal carries the old subscription's lease and skips
/// the replacement.
#[test]
fn test_stale_order_cleanup_preserves_newer_subscription() -> Result<(), Error> {
    let (_, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let order_id = OrderId::from(42);
    let sub_a = bus.send_order_request(order_id, &[])?;
    let sub_b = bus.send_order_request(order_id, &[])?;
    drop(sub_a);

    drain_cleanup_signals(&bus);

    let sender = bus.orders.copy_sender(order_id).expect("stale cleanup removed the newer subscription");
    sender
        .send(RoutedItem::Error(Error::Cancelled))
        .expect("send to registered channel failed");
    let item = sub_b.next_timeout_routed(TICK);
    assert!(matches!(item, Some(RoutedItem::Error(Error::Cancelled))), "{item:?}");
    drop(sender);

    // The replacement's own drop still cleans up.
    drop(sub_b);
    drain_cleanup_signals(&bus);
    assert!(!bus.orders.contains(&order_id), "order channel leaked");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// The identity guards directly: a cleanup carrying a foreign lease must not
/// remove a live registration, and one carrying the registered lease must.
#[test]
fn test_cleanup_identity_guards() -> Result<(), Error> {
    let (_, bus) = make_bus();

    let order_id = OrderId::from(7);
    let _order_sub = bus.send_order_request(order_id, &[])?;
    let foreign = Lease::new().downgrade();
    bus.clean_order(order_id, &foreign);
    assert!(bus.orders.contains(&order_id), "foreign lease removed a live order registration");
    let registered = bus.orders.lease(order_id).unwrap();
    bus.clean_order(order_id, &registered);
    assert!(!bus.orders.contains(&order_id), "matching lease failed to remove the registration");

    let _stream_sub = bus.create_order_update_subscription()?;
    bus.clear_order_update_stream(&foreign);
    assert!(
        bus.order_update_stream.lock().unwrap().is_some(),
        "foreign lease cleared a live order update stream"
    );
    let registered = bus.order_update_stream.lock().unwrap().as_ref().unwrap().lease.clone();
    bus.clear_order_update_stream(&registered);
    assert!(bus.order_update_stream.lock().unwrap().is_none(), "matching lease failed to clear");

    Ok(())
}

/// #789: an order-range error id reaches the order subscription and the
/// order-update stream, never a request. A range-routing guard: the request
/// below sits at the same small number plus the floor, so the ids cannot
/// collide; the collision itself is what the floor rules out.
#[test]
fn test_issue_789_order_error_reaches_order_side() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let order_id = OrderId::from(7);
    let request = bus.send_request(RequestId::nth(order_id.value()), &[])?;
    let order = bus.send_order_request(order_id, &[])?;
    let updates = bus.create_order_update_subscription()?;

    stream.push_inbound(error_frame(order_id.value(), 202, "Order Canceled"));
    stream.push_inbound(error_frame(order_id.value(), 201, "Order rejected"));
    bus.dispatch()?;
    bus.dispatch()?;

    match order.next_timeout_routed(TICK) {
        Some(RoutedItem::Notice(notice)) => assert_eq!(notice.code, 202),
        other => panic!("expected the 202 notice, got {other:?}"),
    }
    match order.next_timeout_routed(TICK) {
        Some(RoutedItem::Error(Error::Notice(notice))) => assert_eq!(notice.code, 201),
        other => panic!("expected the 201 error, got {other:?}"),
    }
    for code in [202, 201] {
        match updates.next_timeout_routed(TICK) {
            Some(RoutedItem::Notice(notice)) => {
                assert_eq!(notice.request_id, Some(order_id.value()));
                assert_eq!(notice.code, code);
            }
            other => panic!("expected the {code} notice on the order-update stream, got {other:?}"),
        }
    }
    assert!(request.try_next_routed().is_none(), "request received an order's error");
    Ok(())
}

/// #789: an error for a request that is gone is not published as order-bound.
#[test]
fn test_issue_789_late_request_error_stays_off_order_stream() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let handle = bus.start_cleanup_thread();
    let request_id = RequestId::nth(5);
    drop(bus.send_request(request_id, &[])?);
    drain_cleanup_signals(&bus);
    assert!(!bus.requests.contains(&request_id), "request still registered");
    let updates = bus.create_order_update_subscription()?;

    stream.push_inbound(error_frame(request_id.raw(), 200, "No security definition found"));
    bus.dispatch()?;

    assert!(
        updates.next_timeout_routed(TICK).is_none(),
        "late request error reached the order-update stream"
    );

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// Routed-but-orphan notice (real request_id, no matching sub) takes the
/// `log_orphan` path, NOT the global notice stream.
#[test]
fn test_warning_with_orphan_request_id_logs() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let unrelated = bus.send_request(RequestId::nth(42), &[])?;
    let notice_stream = bus.notice_subscribe();

    stream.push_inbound(error_frame(RequestId::nth(99).raw(), 2104, "orphan warning"));
    bus.dispatch()?;

    assert!(unrelated.try_next_routed().is_none(), "unrelated sub got the notice");
    assert!(notice_stream.try_next().is_none(), "global notice stream got a routed-but-orphan notice");
    Ok(())
}

#[test]
fn test_is_connected_reflects_shutdown() {
    let (_, bus) = make_bus();

    assert!(bus.is_connected());
    bus.request_shutdown();
    assert!(!bus.is_connected());
}

/// `ExecutionData` with no matching order or request subscription falls
/// through both branches; an unrelated subscription must not see it.
#[test]
fn test_execution_data_orphan_dropped() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let unrelated = bus.send_request(RequestId::nth(42), &[])?;

    stream.push_inbound(execution_data_frame(RequestId::nth(99).raw(), 7, "exec-1"));
    bus.dispatch()?;

    assert!(unrelated.try_next().is_none(), "unrelated sub got an orphan message");
    Ok(())
}

#[test]
fn test_execution_data_end_orphan_dropped() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let unrelated = bus.send_request(RequestId::nth(42), &[])?;

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::ExecutionDataEnd as i32,
        &crate::proto::ExecutionDetailsEnd {
            req_id: Some(RequestId::nth(999).raw()),
        },
    ));
    bus.dispatch()?;

    assert!(unrelated.try_next().is_none(), "unrelated sub got an orphan end");
    Ok(())
}

#[test]
fn test_commission_report_without_mapping_dropped() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let unrelated = bus.send_order_request(OrderId::from(7), &[])?;

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::CommissionsReport as i32,
        &crate::proto::CommissionAndFeesReport {
            exec_id: Some("exec-not-mapped".into()),
            ..Default::default()
        },
    ));
    bus.dispatch()?;

    assert!(unrelated.try_next().is_none(), "unrelated sub got an unmapped commission");
    Ok(())
}

/// #880: an execution-id alias holds a sender clone, so it must go when the
/// order or request subscription that owns it is dropped.
#[test]
fn test_execution_aliases_pruned_when_subscriptions_drop() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let order = bus.send_order_request(OrderId::from(7), &[])?;
    let executions = bus.send_request(RequestId::nth(99), &[])?;
    stream.push_inbound(execution_data_frame(0, 7, "exec-order"));
    stream.push_inbound(execution_data_frame(RequestId::nth(99).raw(), 0, "exec-request"));
    bus.dispatch()?;
    bus.dispatch()?;
    assert_eq!(bus.executions.len(), 2, "both executions mapped");

    drop(order);
    drain_cleanup_signals(&bus);
    assert!(!bus.executions.contains(&"exec-order".to_string()), "order alias leaked");
    assert!(bus.executions.contains(&"exec-request".to_string()), "live request alias pruned");

    drop(executions);
    drain_cleanup_signals(&bus);
    assert_eq!(bus.executions.len(), 0, "request alias leaked");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// `cancel()` releases the registration and its aliases; the drop that
/// follows finds nothing left to release.
#[test]
fn test_execution_aliases_pruned_on_cancel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let order_id = OrderId::from(7);
    let order = bus.send_order_request(order_id, &[])?;
    stream.push_inbound(execution_data_frame(0, 7, "exec-order"));
    bus.dispatch()?;
    order.cancel();
    drain_cleanup_signals(&bus);
    assert!(!bus.orders.contains(&order_id), "order route outlived its cancel");
    assert_eq!(bus.executions.len(), 0, "alias leaked after cancel");

    drop(order);
    drain_cleanup_signals(&bus);
    assert_eq!(bus.executions.len(), 0);

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// A stale drop signal releases the old subscription's aliases and keeps those
/// of a newer registration under the same order id.
#[test]
fn test_stale_cleanup_keeps_newer_execution_aliases() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let sub_a = bus.send_order_request(OrderId::from(42), &[])?;
    stream.push_inbound(execution_data_frame(0, 42, "exec-a"));
    bus.dispatch()?;
    let sub_b = bus.send_order_request(OrderId::from(42), &[])?;
    stream.push_inbound(execution_data_frame(0, 42, "exec-b"));
    bus.dispatch()?;

    drop(sub_a);
    drain_cleanup_signals(&bus);
    assert!(!bus.executions.contains(&"exec-a".to_string()), "stale subscription's alias leaked");
    assert!(bus.executions.contains(&"exec-b".to_string()), "newer subscription's alias pruned");

    drop(sub_b);
    drain_cleanup_signals(&bus);
    assert_eq!(bus.executions.len(), 0);

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// `process_response_with_id` routes by range: a non-order message
/// (HistogramData) whose id is below the request floor goes to the order
/// subscription for that id.
#[test]
fn test_response_with_order_range_id_routes_to_order_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let order_sub = bus.send_order_request(OrderId::from(7), &[])?;

    stream.push_inbound(body("89|7|payload|"));
    bus.dispatch()?;

    order_sub.next_timeout(TICK).expect("order sub got no message")?;
    Ok(())
}

#[test]
fn test_response_with_no_recipient_dropped() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let unrelated = bus.send_request(RequestId::nth(42), &[])?;

    stream.push_inbound(body(&format!("89|{}|payload|", RequestId::nth(999))));
    bus.dispatch()?;

    assert!(unrelated.try_next().is_none(), "unrelated sub got a stray message");
    Ok(())
}

/// `reset` notifies every channel category — requests, orders, shared — and
/// clears the channel maps. All three categories must be live before the call
/// to exercise each `fail_all` branch. A shared subscription receives the
/// reset once, however many response types its request maps to
/// (`RequestAccountData` maps to four).
#[test]
fn test_reset_notifies_all_channel_categories() -> Result<(), Error> {
    let (_, bus) = make_bus();

    let (request_id, order_id) = (RequestId::nth(100), OrderId::from(200));
    let req = bus.send_request(request_id, &[])?;
    let order = bus.send_order_request(order_id, &[])?;
    let shared = bus.send_shared_request(OutgoingMessages::RequestAccountData, &[])?;

    bus.reset();

    for (name, sub) in [("request", &req), ("order", &order), ("shared", &shared)] {
        let resp = sub.next_timeout(TICK).unwrap_or_else(|| panic!("{name} sub got no notification"));
        assert!(matches!(resp, Err(Error::ConnectionReset)), "{name}: {resp:?}");
    }
    assert!(
        shared.try_next_routed().is_none(),
        "shared subscription received the reset more than once"
    );

    assert!(!bus.requests.contains(&request_id));
    assert!(!bus.orders.contains(&order_id));
    Ok(())
}

/// A garbage length prefix must fail the read rather than size an allocation
/// from it. Before this guard the oversized case reached `vec![0u8; 4 GiB]`,
/// and on a live socket the subsequent `read_exact` would consume every real
/// message until it was satisfied — permanently desynchronizing the stream.
#[test]
fn test_read_message_rejects_out_of_range_length_prefix() {
    use crate::transport::common::{MAX_FRAME_LENGTH, MIN_FRAME_LENGTH};

    for prefix in [0, MIN_FRAME_LENGTH as u32 - 1, MAX_FRAME_LENGTH as u32 + 1, u32::MAX] {
        // Prefix only: a valid frame's body never arrives, so if the length were
        // accepted this would block or allocate before noticing anything is wrong.
        let framed = prefix.to_be_bytes().to_vec();

        let err = read_message(&mut framed.as_slice(), &RawFrameTap::disabled()).expect_err("an out-of-range length prefix must be rejected");
        assert!(
            matches!(err, Error::InvalidFrame(_)),
            "prefix {prefix} must raise InvalidFrame, got {err:?}"
        );
    }

    // The smallest legal frame still reads: a bare message id with no payload.
    let framed = encode_raw_length(&9_i32.to_be_bytes());
    assert_eq!(
        read_message(&mut framed.as_slice(), &RawFrameTap::disabled()).unwrap(),
        9_i32.to_be_bytes()
    );
}

/// The blocking frame reader taps the wire, and taps it *below* validation.
///
/// Both halves matter. A capture that only holds frames the reader accepted
/// would be missing the one byte sequence worth capturing: the desync in #891
/// announces itself as a length prefix that cannot describe a frame, and that
/// prefix is rejected before any caller sees it.
#[test]
fn test_read_message_taps_raw_bytes_including_rejected_prefixes() {
    let dir = tempfile::TempDir::new().unwrap();
    let tap = RawFrameTap::capturing_to(dir.path());

    let good = encode_raw_length(&[0, 0, 0, 9, 42]);
    let bad_prefix = u32::MAX.to_be_bytes();

    assert_eq!(read_message(&mut good.as_slice(), &tap).unwrap(), [0, 0, 0, 9, 42]);
    let err = read_message(&mut bad_prefix.as_slice(), &tap).expect_err("prefix must be rejected");
    assert!(matches!(err, Error::InvalidFrame(_)), "got {err:?}");

    let mut expected = good.clone();
    expected.extend_from_slice(&bad_prefix);
    assert_eq!(test_support::frames(dir.path()), expected);
}

// ---- frame reads across socket read timeouts (#892) ----

/// A `Read` that replays one scripted result per `read` call. Each `Ok` chunk
/// must fit the caller's buffer; a test that splits differently has a bug.
struct ScriptedReader(VecDeque<std::io::Result<Vec<u8>>>);

impl ScriptedReader {
    fn new(script: impl IntoIterator<Item = std::io::Result<Vec<u8>>>) -> Self {
        Self(script.into_iter().collect())
    }
}

impl std::io::Read for ScriptedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.0.pop_front() {
            Some(Ok(chunk)) => {
                assert!(
                    chunk.len() <= buf.len(),
                    "scripted chunk of {} exceeds the {}-byte read",
                    chunk.len(),
                    buf.len()
                );
                buf[..chunk.len()].copy_from_slice(&chunk);
                Ok(chunk.len())
            }
            Some(Err(e)) => Err(e),
            None => Ok(0),
        }
    }
}

fn timed_out() -> std::io::Result<Vec<u8>> {
    Err(std::io::ErrorKind::WouldBlock.into())
}

const FRAME_BODY: [u8; 5] = [0, 0, 0, 9, 42];

/// The split frame also lands in the raw capture byte-exact.
#[test]
fn test_read_message_keeps_prefix_bytes_across_a_timeout() {
    let dir = tempfile::TempDir::new().unwrap();
    let tap = RawFrameTap::capturing_to(dir.path());
    let framed = encode_raw_length(&FRAME_BODY);
    let mut reader = ScriptedReader::new([Ok(framed[..2].to_vec()), timed_out(), Ok(framed[2..4].to_vec()), Ok(framed[4..].to_vec())]);

    assert_eq!(read_message(&mut reader, &tap).unwrap(), FRAME_BODY);
    assert_eq!(test_support::frames(dir.path()), framed);
}

/// The frame after a split one still decodes: framing stayed aligned.
#[test]
fn test_read_message_keeps_body_bytes_across_a_timeout() {
    let first = encode_raw_length(&FRAME_BODY);
    let second = encode_raw_length(&[0, 0, 0, 7]);
    let mut reader = ScriptedReader::new([
        Ok(first[..4].to_vec()),
        Ok(first[4..6].to_vec()),
        Err(std::io::ErrorKind::TimedOut.into()),
        Ok(first[6..].to_vec()),
        Ok(second[..4].to_vec()),
        Ok(second[4..].to_vec()),
    ]);

    assert_eq!(read_message(&mut reader, &RawFrameTap::disabled()).unwrap(), FRAME_BODY);
    assert_eq!(read_message(&mut reader, &RawFrameTap::disabled()).unwrap(), [0, 0, 0, 7]);
}

/// A timeout before any byte of the frame is still idle: the dispatcher polls
/// its shutdown flag and the handshake fails fast.
/// Both kinds: Unix reports an `SO_RCVTIMEO` expiry as `WouldBlock`, Windows
/// as `TimedOut`.
#[test]
fn test_read_message_returns_timeout_before_the_first_byte() {
    for kind in [std::io::ErrorKind::WouldBlock, std::io::ErrorKind::TimedOut] {
        let mut reader = ScriptedReader::new([Err(kind.into())]);

        let err = read_message(&mut reader, &RawFrameTap::disabled()).expect_err("idle read must time out");
        assert!(err.is_read_timeout(), "{kind:?}: got {err:?}");
    }
}

#[test]
fn test_read_message_eof_mid_frame_is_connection_lost() {
    let framed = encode_raw_length(&FRAME_BODY);
    let mut reader = ScriptedReader::new([Ok(framed[..4].to_vec()), Ok(framed[4..6].to_vec())]);

    let err = read_message(&mut reader, &RawFrameTap::disabled()).expect_err("truncated frame must fail");
    assert!(err.is_connection_lost(), "got {err:?}");
}

#[test]
fn test_read_message_retries_interrupted_reads() {
    let framed = encode_raw_length(&FRAME_BODY);
    let mut reader = ScriptedReader::new([
        Ok(framed[..3].to_vec()),
        Err(std::io::ErrorKind::Interrupted.into()),
        Ok(framed[3..4].to_vec()),
        Ok(framed[4..].to_vec()),
    ]);

    assert_eq!(read_message(&mut reader, &RawFrameTap::disabled()).unwrap(), FRAME_BODY);
}

/// The stall budget belongs to the frame: timeouts in the prefix and the body
/// count together, and only progress resets them.
#[test]
fn test_read_message_mid_frame_stall_limit() {
    let framed = encode_raw_length(&FRAME_BODY);
    let stalls = |n: u32| (0..n).map(|_| timed_out());
    let limit = MID_FRAME_TIMEOUT_LIMIT;

    // At the limit, split across prefix and body around a progress reset: reads.
    let script = std::iter::once(Ok(framed[..2].to_vec()))
        .chain(stalls(limit))
        .chain([Ok(framed[2..4].to_vec())])
        .chain(stalls(limit))
        .chain([Ok(framed[4..].to_vec())]);
    let mut reader = ScriptedReader::new(script);
    assert_eq!(read_message(&mut reader, &RawFrameTap::disabled()).unwrap(), FRAME_BODY);

    // One past the limit without progress: the frame is abandoned.
    let script = std::iter::once(Ok(framed[..2].to_vec())).chain(stalls(limit + 1));
    let mut reader = ScriptedReader::new(script);
    let err = read_message(&mut reader, &RawFrameTap::disabled()).expect_err("stalled frame must fail");
    assert!(matches!(err, Error::InvalidFrame(_)), "got {err:?}");
    assert!(err.is_connection_lost(), "a stalled frame must reconnect");
}

/// Connect to a peer thread that runs `peer` on the accepted socket. The client
/// gets `read_timeout`; `ready` fires once the peer has sent its first bytes, so
/// the client's first read can't time out before anything was sent.
fn socket_pair(
    read_timeout: Duration,
    peer: impl FnOnce(std::net::TcpStream, &std::sync::mpsc::Sender<()>) + Send + 'static,
) -> (std::net::TcpStream, std::sync::mpsc::Receiver<()>, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (ready_send, ready) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        peer(socket, &ready_send);
    });
    let client = std::net::TcpStream::connect(address).unwrap();
    client.set_read_timeout(Some(read_timeout)).unwrap();
    (client, ready, handle)
}

/// The #892 repro on a real socket: the peer sends half a length prefix, stalls
/// past the read timeout, then sends the rest and a second frame. Both frames
/// must read intact.
#[test]
fn test_read_message_survives_a_socket_read_timeout_mid_frame() {
    let read_timeout = Duration::from_millis(50);
    let first = encode_raw_length(&FRAME_BODY);
    let second = encode_raw_length(&[0, 0, 0, 7]);
    let (head, rest) = first.split_at(2);
    let (head, mut rest) = (head.to_vec(), rest.to_vec());
    rest.extend_from_slice(&second);

    let (mut client, ready, peer) = socket_pair(read_timeout, move |mut socket, ready| {
        socket.write_all(&head).unwrap();
        ready.send(()).unwrap();
        std::thread::sleep(read_timeout * 3);
        socket.write_all(&rest).unwrap();
    });
    ready.recv().unwrap();

    assert_eq!(read_message(&mut client, &RawFrameTap::disabled()).unwrap(), FRAME_BODY);
    assert_eq!(read_message(&mut client, &RawFrameTap::disabled()).unwrap(), [0, 0, 0, 7]);
    peer.join().unwrap();
}

/// `shutdown_read` still breaks a read that is waiting out a stalled frame: the
/// read ends as a lost connection right away, not after the stall limit.
#[test]
fn test_shutdown_read_breaks_a_mid_frame_wait() {
    let read_timeout = Duration::from_millis(50);
    let (release_send, release) = std::sync::mpsc::channel::<()>();
    let (mut client, ready, peer) = socket_pair(read_timeout, move |mut socket, ready| {
        socket.write_all(&[0, 0]).unwrap();
        ready.send(()).unwrap();
        // Hold the connection open, mid-frame, until the test is done.
        let _ = release.recv();
    });
    ready.recv().unwrap();

    let shutdown_handle = client.try_clone().unwrap();
    let shutdown = std::thread::spawn(move || {
        std::thread::sleep(read_timeout * 3);
        shutdown_handle.shutdown(std::net::Shutdown::Read).unwrap();
    });

    let started = Instant::now();
    let err = read_message(&mut client, &RawFrameTap::disabled()).expect_err("shutdown must end the read");
    let elapsed = started.elapsed();
    assert!(err.is_connection_lost(), "got {err:?}");
    let stall_limit = read_timeout * MID_FRAME_TIMEOUT_LIMIT;
    assert!(
        elapsed < stall_limit,
        "read ended after {elapsed:?}, not before the {stall_limit:?} stall limit"
    );

    shutdown.join().unwrap();
    drop(release_send);
    peer.join().unwrap();
}

/// Blocking twin of the async unknown-message-id test. This path already had an
/// `info!` log, which is exactly why the 2026-07-07 desync went unnoticed —
/// nothing a consumer could act on. It now raises a notice.
#[test]
fn test_unknown_message_id_reaches_the_notice_stream() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let notice_stream = bus.notice_subscribe();

    stream.push_inbound(helpers::unknown_message_frame());

    bus.dispatch()?;

    let notice = notice_stream.next_timeout(TICK).expect("unknown frame must raise a notice");
    assert_eq!(notice.code, crate::messages::UNKNOWN_MESSAGE_TYPE_CODE);
    // The id survives the protobuf path, where `kind` alone would have lost it.
    assert!(
        notice.message.contains(&helpers::UNKNOWN_MESSAGE_ID.to_string()),
        "notice must name the offending id, got {:?}",
        notice.message
    );
    Ok(())
}

/// A frame body too short to hold the message id must send the dispatcher down
/// its reconnect branch, not kill it. `parse_raw_message` rejects the body as
/// `InvalidFrame`, which counts as connection lost; before that guard it
/// indexed past the end and panicked the dispatcher thread (#891).
#[test]
fn test_short_frame_body_reconnects() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let notices = bus.connection.notice_broadcaster.subscribe();

    stream.push_inbound(b"xx".to_vec());
    for frame in helpers::handshake_frames(crate::server_versions::PROTOBUF_REST_MESSAGES_3, "EST", 5000) {
        stream.push_inbound(frame);
    }

    bus.dispatch()?;

    let notice = notices.recv_timeout(TICK).expect("no reconnect notice on the notice fan-out");
    assert_eq!(notice.code, TRANSPORT_RECONNECT_CODE, "{notice:?}");
    assert!(bus.is_connected());
    Ok(())
}

/// Every send is refused while the session is down: nothing reaches the socket
/// the reconnect is replacing, and nothing is registered on a channel no reset
/// would clear again.
#[test]
fn test_sends_are_refused_while_disconnected() {
    let (stream, bus) = make_bus();
    let mb: &dyn MessageBus = bus.as_ref();

    bus.connection_state.set_disconnected();

    assert!(matches!(mb.send_request(RequestId::nth(100), b"req-bytes"), Err(Error::ConnectionReset)));
    assert!(matches!(
        mb.send_order_request(OrderId::from(42), b"order-bytes"),
        Err(Error::ConnectionReset)
    ));
    assert!(matches!(
        mb.send_shared_request(OutgoingMessages::RequestManagedAccounts, b"shared-bytes"),
        Err(Error::ConnectionReset)
    ));
    assert!(matches!(mb.send_message(b"message-bytes"), Err(Error::ConnectionReset)));

    assert_eq!(bus.requests.len(), 0, "a refused request must register nothing");
    assert_eq!(bus.orders.len(), 0, "a refused order request must register nothing");
    assert_eq!(
        shared_subscriber_count(&bus, |_| true),
        0,
        "a refused shared request must register nothing"
    );
    assert!(stream.captured().is_empty(), "a refused send must not reach the socket");

    // The same send goes through once the handshake has put the session back.
    bus.connection_state.set_connected();
    assert!(mb.send_request(RequestId::nth(100), b"req-bytes").is_ok());
    assert!(!stream.captured().is_empty());
}

/// A cancel that cannot be sent is not an error the caller has to handle:
/// the session that held the subscription is gone. The local registration goes
/// either way, so nothing is left behind.
#[test]
fn test_cancel_while_disconnected_clears_the_registration() -> Result<(), Error> {
    use crate::contracts::ContractDetails;
    use crate::subscriptions::sync::Subscription;
    use crate::subscriptions::DecoderContext;

    let (_stream, bus) = make_bus();
    let handle = bus.start_cleanup_thread();
    let request_id = RequestId::nth(100);
    let internal = bus.send_request(request_id, &[])?;
    let subscription: Subscription<ContractDetails> =
        Subscription::new(bus.clone(), internal, DecoderContext::new(crate::server_versions::CANCEL_CONTRACT_DATA));

    bus.connection_state.set_disconnected();
    subscription.cancel();

    // The marker request `drain_cleanup_signals` sends needs a connection.
    bus.connection_state.set_connected();
    drain_cleanup_signals(&bus);
    assert!(!bus.requests.contains(&request_id), "cancel must clear the registration anyway");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// `wait_connected` blocks the one-shot retry until the session is back, and a
/// shutdown releases it rather than leaving the caller there for good.
#[test]
fn test_wait_connected_returns_shutdown_when_the_bus_shuts_down() {
    let (_stream, bus) = make_bus();
    bus.connection_state.set_disconnected();

    let closer = Arc::clone(&bus);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        closer.request_shutdown();
    });

    let start = Instant::now();
    let result = MessageBus::wait_connected(&*bus);
    assert!(matches!(result, Err(Error::Shutdown)), "got: {result:?}");
    assert!(start.elapsed() < Duration::from_secs(5), "wait_connected did not return on shutdown");
}

/// Writes fail, reads delegate to a `MemoryStream`. Stands in for the window
/// the send gate cannot close: the write is refused after the registration
/// went in.
#[derive(Clone, Debug)]
struct FailingWriteStream(MemoryStream);

impl Io for FailingWriteStream {
    fn read_message(&self) -> Result<Vec<u8>, Error> {
        self.0.read_message()
    }

    fn write_all(&self, _buf: &[u8]) -> Result<(), Error> {
        Err(Error::ConnectionReset)
    }
}

impl Reconnect for FailingWriteStream {
    fn reconnect(&self) -> Result<(), Error> {
        self.0.reconnect()
    }

    fn sleep(&self, duration: Duration, shutdown: &ShutdownSignal) {
        self.0.sleep(duration, shutdown)
    }

    fn shutdown_read(&self) -> Result<(), Error> {
        self.0.shutdown_read()
    }
}

impl Stream for FailingWriteStream {}

/// A send whose write fails takes its registration with it, so no channel is
/// left waiting on a response that was never requested. Covers the window the
/// `ensure_connected` check cannot close, where the session goes down between
/// the check and the write.
#[test]
fn test_failed_write_leaves_no_registration() {
    let connection = Connection::stubbed(FailingWriteStream(MemoryStream::default()), 28);
    connection.set_server_version_for_test(crate::server_versions::PROTOBUF_REST_MESSAGES_3);
    let bus = Arc::new(TcpMessageBus::new(connection).unwrap());
    let mb: &dyn MessageBus = bus.as_ref();

    assert!(mb.send_request(RequestId::nth(100), b"req-bytes").is_err());
    assert_eq!(bus.requests.len(), 0, "a failed write must leave no request registered");

    assert!(mb.send_order_request(OrderId::from(42), b"order-bytes").is_err());
    assert_eq!(bus.orders.len(), 0, "a failed write must leave no order registered");

    assert!(mb.send_shared_request(OutgoingMessages::RequestPositions, b"positions").is_err());
    assert_eq!(
        shared_subscriber_count(&bus, |_| true),
        0,
        "a failed write must leave no shared subscription registered"
    );
}

#[test]
fn order_binding_reaches_updates_without_using_raw_order_id() {
    let (stream, bus) = make_bus();
    let order_sub = bus.send_order_request(OrderId::from(42), &[]).unwrap();
    let update_sub = bus.create_order_update_subscription().unwrap();
    stream.push_inbound(binary_proto(IncomingMessages::OrderBound as i32, &order_bound().client_id(73).to_proto()));
    bus.dispatch().unwrap();
    let message = update_sub.next_timeout(TICK).expect("update stream got no message").unwrap();
    assert_eq!(message.message_type(), IncomingMessages::OrderBound);
    assert!(order_sub.next_timeout(TICK).is_none());
}

// ---- buffer_limit: bounded request routes ------------------------------------

/// A cap of `limit`, ending on `ContractDataEnd`.
fn bound(limit: usize) -> BufferBound {
    BufferBound {
        limit,
        end: IncomingMessages::ContractDataEnd,
    }
}

/// The request id `contract_row` and `contract_end` frames carry.
const CONTRACT_REQUEST_ID: RequestId = RequestId::nth(0);

fn contract_row(contract_id: i32) -> Vec<u8> {
    binary_proto(
        IncomingMessages::ContractData as i32,
        &contract_data().request_id(CONTRACT_REQUEST_ID.raw()).contract_id(contract_id).to_proto(),
    )
}

fn contract_end() -> Vec<u8> {
    binary_proto(
        IncomingMessages::ContractDataEnd as i32,
        &crate::proto::ContractDataEnd {
            req_id: Some(CONTRACT_REQUEST_ID.raw()),
        },
    )
}

fn histogram(request_id: RequestId) -> Vec<u8> {
    // HistogramData (msg_id 89): request_id at field index 1.
    body(&format!("89|{request_id}|payload|"))
}

fn route(stream: &MemoryStream, bus: &TcpMessageBus<MemoryStream>, frames: usize, request_id: RequestId) -> Result<(), Error> {
    for _ in 0..frames {
        stream.push_inbound(histogram(request_id));
        bus.dispatch()?;
    }
    Ok(())
}

#[test]
fn test_bounded_request_fails_after_limit_unread() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_request_bounded(RequestId::nth(100), &[], bound(2))?;

    route(&stream, &bus, 4, RequestId::nth(100))?;

    assert!(sub.next_timeout(TICK).expect("first row")?.peek_int(1)? == RequestId::nth(100).raw());
    assert!(sub.next_timeout(TICK).expect("second row").is_ok());
    assert!(matches!(sub.next_timeout(TICK), Some(Err(Error::BufferLimitExceeded { limit: 2 }))));
    assert!(sub.try_next().is_none(), "frames after the overflow are discarded");
    Ok(())
}

#[test]
fn test_bounded_request_counts_unread_not_total() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_request_bounded(RequestId::nth(100), &[], bound(2))?;

    for _ in 0..6 {
        route(&stream, &bus, 1, RequestId::nth(100))?;
        assert!(sub.next_timeout(TICK).expect("row").is_ok(), "a reader that keeps up never overflows");
    }
    Ok(())
}

#[test]
fn test_reset_skips_overflowed_route() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let overflowed = bus.send_request_bounded(RequestId::nth(100), &[], bound(1))?;
    let at_limit = bus.send_request_bounded(RequestId::nth(200), &[], bound(1))?;

    route(&stream, &bus, 2, RequestId::nth(100))?;
    route(&stream, &bus, 1, RequestId::nth(200))?;
    bus.reset();

    assert!(overflowed.next_timeout(TICK).expect("row").is_ok());
    assert!(matches!(overflowed.next_timeout(TICK), Some(Err(Error::BufferLimitExceeded { .. }))));
    assert!(overflowed.try_next().is_none(), "no second terminal error after reset");

    assert!(at_limit.next_timeout(TICK).expect("row").is_ok());
    assert!(matches!(at_limit.next_timeout(TICK), Some(Err(Error::ConnectionReset))));
    Ok(())
}

#[test]
fn test_overflowed_subscription_cancels_on_drop() -> Result<(), Error> {
    use crate::contracts::ContractDetails;
    use crate::subscriptions::sync::Subscription;
    use crate::subscriptions::DecoderContext;

    let (stream, bus) = make_bus();
    let internal = bus.send_request_bounded(CONTRACT_REQUEST_ID, &[], bound(1))?;
    let subscription: Subscription<ContractDetails> =
        Subscription::new(bus.clone(), internal, DecoderContext::new(crate::server_versions::CANCEL_CONTRACT_DATA));

    for contract_id in [1, 2] {
        stream.push_inbound(contract_row(contract_id));
        bus.dispatch()?;
    }

    assert!(matches!(subscription.next(), Some(Ok(_))));
    assert!(matches!(subscription.next(), Some(Err(Error::BufferLimitExceeded { limit: 1 }))));
    drop(subscription);

    let cancel = <ContractDetails as crate::subscriptions::StreamDecoder<ContractDetails>>::cancel_message(
        crate::server_versions::CANCEL_CONTRACT_DATA,
        Some(CONTRACT_REQUEST_ID.raw()),
        None,
    )?;
    assert_eq!(count_frames(&stream.captured(), &cancel), 1, "overflow leaves the cancel to drop");
    Ok(())
}

/// `cancel_and_drain` writes the cancel without unregistering the route, so
/// TWS's end marker, dispatched after the cancel, still reaches the drain.
/// (`cancel()` unregisters the route.)
#[test]
fn test_drain_route_survives_its_cancel() -> Result<(), Error> {
    use crate::contracts::ContractDetails;
    use crate::subscriptions::sync::Subscription;
    use crate::subscriptions::{DecoderContext, Drained};

    let (stream, bus) = make_bus();
    let internal = bus.send_request(CONTRACT_REQUEST_ID, &[])?;
    let subscription: Subscription<ContractDetails> =
        Subscription::new(bus.clone(), internal, DecoderContext::new(crate::server_versions::CANCEL_CONTRACT_DATA));
    let cancel = <ContractDetails as crate::subscriptions::StreamDecoder<ContractDetails>>::cancel_message(
        crate::server_versions::CANCEL_CONTRACT_DATA,
        Some(CONTRACT_REQUEST_ID.raw()),
        None,
    )?;

    let dispatcher = {
        let (stream, bus, cancel) = (stream.clone(), bus.clone(), cancel.clone());
        std::thread::spawn(move || -> Result<(), Error> {
            // Dispatch the end only once the drain's cancel is on the wire.
            let deadline = Instant::now() + Duration::from_secs(2);
            while count_frames(&stream.captured(), &cancel) == 0 && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(1));
            }
            stream.push_inbound(contract_end());
            bus.dispatch()
        })
    };

    let outcome = subscription.cancel_and_drain(Instant::now() + Duration::from_secs(2))?;
    dispatcher.join().expect("dispatcher panicked")?;

    assert_eq!(outcome, Drained::Ended);
    assert_eq!(count_frames(&stream.captured(), &cancel), 1);
    Ok(())
}

#[test]
fn test_bounded_request_end_marker_at_limit_still_ends() -> Result<(), Error> {
    // A result exactly `limit` rows long, read late: the end marker gets
    // through past the cap, so the stream ends normally.
    let (stream, bus) = make_bus();
    let sub = bus.send_request_bounded(CONTRACT_REQUEST_ID, &[], bound(1))?;

    for frame in [contract_row(1), contract_end(), contract_row(2)] {
        stream.push_inbound(frame);
        bus.dispatch()?;
    }

    assert_eq!(sub.next_timeout(TICK).expect("row")?.message_type(), IncomingMessages::ContractData);
    assert_eq!(sub.next_timeout(TICK).expect("end")?.message_type(), IncomingMessages::ContractDataEnd);
    assert!(sub.try_next().is_none(), "frames after the end marker are discarded");
    Ok(())
}

/// A request route, its receiver, and the lease that keeps it live.
fn sender_hash_route() -> (SenderHash<RequestId, RoutedItem>, Receiver<RoutedItem>, Lease) {
    let routes = SenderHash::new();
    let (sender, receiver) = channel::unbounded();
    let lease = Lease::new();
    routes.insert(RequestId::nth(1), sender, lease.downgrade());
    (routes, receiver, lease)
}

#[test]
fn sender_hash_deliver_hands_the_item_back_when_unrouted() {
    let routes = SenderHash::<RequestId, RoutedItem>::new();
    let item = routes.deliver(&RequestId::nth(1), RoutedItem::Error(Error::Cancelled));
    assert!(matches!(item, Err(RoutedItem::Error(Error::Cancelled))));
}

#[test]
fn sender_hash_deliver_aliased_aliases_the_route() {
    let (routes, receiver, lease) = sender_hash_route();
    let aliases = SenderHash::<String, RoutedItem>::new();
    let alias = "exec-1".to_string();

    routes
        .deliver_aliased(&RequestId::nth(1), RoutedItem::Error(Error::Cancelled), Some(&alias), &aliases)
        .unwrap();
    aliases.deliver(&alias, Error::ConnectionReset.into()).unwrap();

    let items: Vec<_> = receiver.try_iter().collect();
    assert!(
        items.len() == 2 && matches!(items[1], RoutedItem::Error(Error::ConnectionReset)),
        "{items:?}"
    );
    assert!(aliases.lease(alias).unwrap().is(&lease.downgrade()), "alias holds another lease");
}

#[test]
fn sender_hash_deliver_aliased_without_alias_registers_none() {
    let (routes, receiver, _lease) = sender_hash_route();
    let aliases = SenderHash::<String, RoutedItem>::new();

    routes
        .deliver_aliased(&RequestId::nth(1), RoutedItem::Error(Error::Cancelled), None, &aliases)
        .unwrap();

    assert_eq!(receiver.try_iter().count(), 1);
    assert_eq!(aliases.len(), 0);
}

#[test]
fn sender_hash_deliver_aliased_hands_the_item_back_when_unrouted() {
    let routes = SenderHash::<RequestId, RoutedItem>::new();
    let aliases = SenderHash::<String, RoutedItem>::new();

    let item = routes.deliver_aliased(
        &RequestId::nth(1),
        RoutedItem::Error(Error::Cancelled),
        Some(&"exec-1".to_string()),
        &aliases,
    );

    assert!(matches!(item, Err(RoutedItem::Error(Error::Cancelled))));
    assert_eq!(aliases.len(), 0, "unrouted item was aliased");
}

/// A panic under the route lock must not take every later route and
/// teardown down with it.
#[test]
fn sender_hash_recovers_from_a_poisoned_lock() {
    let (routes, receiver, _lease) = sender_hash_route();
    std::thread::scope(|scope| {
        let _ = scope
            .spawn(|| {
                let _guard = routes.senders.write().unwrap();
                panic!("poison the route lock");
            })
            .join();
    });
    assert!(routes.senders.is_poisoned());

    routes.deliver(&RequestId::nth(1), RoutedItem::Error(Error::Cancelled)).unwrap();
    routes.clear();

    assert_eq!(receiver.try_iter().count(), 1);
    assert_eq!(routes.len(), 0);
}

/// A panic under the order-update slot's lock must not silently drop later
/// order updates or leave the slot uncleanable.
#[test]
fn test_order_update_stream_survives_a_poisoned_lock() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let stream_sub = bus.create_order_update_subscription()?;
    std::thread::scope(|scope| {
        let _ = scope
            .spawn(|| {
                let _guard = bus.order_update_stream.lock().unwrap();
                panic!("poison the order-update slot");
            })
            .join();
    });
    assert!(bus.order_update_stream.is_poisoned());

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::OpenOrder as i32,
        &crate::proto::OpenOrder {
            order_id: Some(42),
            ..Default::default()
        },
    ));
    bus.dispatch()?;
    assert!(stream_sub.next_timeout(TICK).is_some(), "update stream missed open order");

    let registered = lock_slot(&bus.order_update_stream).as_ref().unwrap().lease.clone();
    bus.clear_order_update_stream(&registered);
    assert!(lock_slot(&bus.order_update_stream).is_none(), "poisoned slot not cleared");
    Ok(())
}
