use super::*;
use crate::connection::common::{ConnectionHandler, ConnectionProtocol};
use crate::connection::sync::Connection;
use crate::tests::assert_send_and_sync;
use crate::transport::common::MAX_RECONNECT_ATTEMPTS;

// Additional imports for connection tests
use crate::client::sync::Client;
use crate::common::test_utils::helpers::{binary_proto, error_frame, proto_response};
use crate::contracts::Contract;
use crate::messages::{encode_length, OutgoingMessages, RequestMessage};
use crate::orders::common::encoders::encode_place_order;
use crate::orders::{order_builder, Action};
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
    fn sleep(&self, _duration: std::time::Duration) {}
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

        // disconnect if a null byte response is encountered
        if response.fields[0] == "\0" {
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
            read_message(&mut expected.as_slice())
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
        Exchange::simple("v213..221", &[&format!("{sv}|20250415 19:38:30 British Summer Time|")]),
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

    let subscription = bus.send_order_request(5, &request)?;

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
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
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
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
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

    match connection.reconnect() {
        Err(Error::ConnectionFailed) => Ok(()),
        _ => panic!(""),
    }
}

#[test]
fn test_reconnect_success() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let events = vec![
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(1),
                ResponseMessage::from_simple("\0"),
            ],
        ),
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
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
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)],
        ),
        Exchange::new(managed_req.clone(), vec![ResponseMessage::from_simple("\0")]),
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
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
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(1),
                ResponseMessage::from_simple("\0"),
            ],
        ), // RESTART
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
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

const AAPL_CONTRACT_RESPONSE: &str  = "AAPL|STK||0||SMART|USD|AAPL|NMS|NMS|265598|0.01||ACTIVETIM,AD,ADDONT,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SMARTSTG,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,NASDAQ,DRCTEDGE,BEX,BATS,EDGEA,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,IBEOS,OVERNIGHT,TPLUS0,PSX|1|0|APPLE INC|NASDAQ||Technology|Computers|Computers|US/Eastern|20250324:0400-20250324:2000;20250325:0400-20250325:2000;20250326:0400-20250326:2000;20250327:0400-20250327:2000;20250328:0400-20250328:2000|20250324:0930-20250324:1600;20250325:0930-20250325:1600;20250326:0930-20250326:1600;20250327:0930-20250327:1600;20250328:0930-20250328:1600|||1|ISIN|US0378331005|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|0.0001|0.0001|100|";

#[test]
fn test_send_request_after_disconnect() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let packet = encode_request_contract_data(sv, 9000, &Contract::stock("AAPL").build())?;

    let expected_response = &format!("10|9000|{AAPL_CONTRACT_RESPONSE}");

    let events = vec![
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(1),
                ResponseMessage::from_simple("\0"),
            ],
        ), // RESTART
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(start_api_bytes, vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)]),
        Exchange::request(packet.clone(), &[expected_response, "52|1|9001|"]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
    let bus = TcpMessageBus::new(connection)?;

    bus.dispatch()?;

    let subscription = bus.send_request(9000, &packet)?;

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
    let packet = encode_request_contract_data(sv, 9000, &Contract::stock("AAPL").build())?;

    let events = vec![
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)],
        ),
        Exchange::request(packet.clone(), &["\0"]), // RESTART
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(start_api_bytes, vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
    let bus = TcpMessageBus::new(connection)?;

    let subscription = bus.send_request(9000, &packet)?;

    bus.dispatch()?;

    match subscription.next() {
        Some(Err(Error::ConnectionReset)) => {}
        _ => panic!(),
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
    let packet = encode_request_contract_data(sv, 9000, &Contract::stock("AAPL").build())?;

    let events = vec![
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                managed_accounts_response("DU1234567"),
                next_valid_id_response(1),
                ResponseMessage::from_simple("\0"),
            ],
        ), // RESTART
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
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

    let packet = encode_request_contract_data(sv, 9000, contract)?;

    let events = vec![
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![managed_accounts_response("DU1234567"), next_valid_id_response(1)],
        ),
        Exchange::request(packet.clone(), &["\0"]),
        Exchange::simple("v213..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
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

/// Build a binary-text-payload response body from a pipe-delimited test input.
/// `"msg_id|f1|f2|..."` → `[4-byte BE msg_id][f1\0f2\0...]`. Pipes are
/// stand-ins for NULs so test inputs stay readable. For `Error` frames,
/// use [`crate::common::test_utils::helpers::error_frame`] — they ship as
/// protobuf post-floor-213 and the binary-text-payload path defaults to an
/// empty Notice.
fn body(text: &str) -> Vec<u8> {
    let fields: Vec<&str> = text.split_terminator('|').collect();
    let msg_id: i32 = fields[0].parse().expect("body() fixture must start with a numeric msg_id");
    debug_assert_ne!(
        msg_id,
        crate::messages::IncomingMessages::Error as i32,
        "Error frames must use error_frame() — protobuf-framed since PR-D1"
    );
    let payload: String = fields[1..].iter().map(|f| format!("{f}\0")).collect();
    let mut data = msg_id.to_be_bytes().to_vec();
    data.extend_from_slice(payload.as_bytes());
    data
}

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

    let sub_a = bus.send_request(100, &[])?;
    let sub_b = bus.send_request(200, &[])?;

    // HistogramData (msg_id 89): request_id at field index 1.
    stream.push_inbound(body("89|200|payload-b|"));
    stream.push_inbound(body("89|100|payload-a|"));

    bus.dispatch()?;
    bus.dispatch()?;

    let msg_a = sub_a.next_timeout(TICK).expect("sub_a got no message")?;
    let msg_b = sub_b.next_timeout(TICK).expect("sub_b got no message")?;
    assert_eq!(msg_a.peek_int(1)?, 100);
    assert_eq!(msg_b.peek_int(1)?, 200);

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

    let sub_a = bus.send_order_request(11, &[])?;
    let sub_b = bus.send_order_request(22, &[])?;

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
/// are notified of `Error::Shutdown`.
#[test]
fn test_dispatch_surfaces_connection_failure_after_eof() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_request(100, &[])?;

    stream.close();
    let err = bus.dispatch().expect_err("dispatch should surface an error");
    assert!(matches!(err, Error::ConnectionFailed), "unexpected error: {err:?}");

    let resp = sub.next_timeout(TICK).expect("subscription got no notification");
    assert!(matches!(resp, Err(Error::Shutdown)), "got: {resp:?}");
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

/// `MessageBus::cancel_subscription` writes the cancel bytes to the stream and
/// notifies the in-flight subscription with `Error::Cancelled`.
#[test]
fn test_cancel_subscription_notifies_in_flight() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let mb: &dyn MessageBus = bus.as_ref();
    let sub = mb.send_request(100, b"req-bytes")?;

    mb.cancel_subscription(100, b"cancel-bytes")?;

    let resp = sub.next_timeout(TICK).expect("subscription got no notification");
    assert!(matches!(resp, Err(Error::Cancelled)), "got: {resp:?}");

    let captured = stream.captured();
    assert!(captured.windows(b"req-bytes".len()).any(|w| w == b"req-bytes"), "request not written");
    assert!(
        captured.windows(b"cancel-bytes".len()).any(|w| w == b"cancel-bytes"),
        "cancel not written"
    );
    Ok(())
}

/// `MessageBus::cancel_order_subscription` mirrors cancel_subscription but on
/// the orders channel.
#[test]
fn test_cancel_order_subscription_notifies_in_flight() -> Result<(), Error> {
    let (_, bus) = make_bus();
    let mb: &dyn MessageBus = bus.as_ref();
    let sub = mb.send_order_request(42, b"order-bytes")?;

    mb.cancel_order_subscription(42, b"cancel-bytes")?;

    let resp = sub.next_timeout(TICK).expect("subscription got no notification");
    assert!(matches!(resp, Err(Error::Cancelled)), "got: {resp:?}");
    Ok(())
}

/// `MessageBus::cancel_shared_subscription` writes the cancel bytes through
/// to the connection. (No notify path — shared channels are persistent.)
#[test]
fn test_cancel_shared_subscription_writes_through() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let mb: &dyn MessageBus = bus.as_ref();

    mb.cancel_shared_subscription(OutgoingMessages::RequestCurrentTime, b"cancel-bytes")?;

    let captured = stream.captured();
    assert!(captured.windows(b"cancel-bytes".len()).any(|w| w == b"cancel-bytes"));
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
/// on duplicate calls; explicit drop of the first subscription releases the
/// slot via the cleanup thread, but here we just assert duplicate-rejection.
#[test]
fn test_create_order_update_subscription_is_unique() -> Result<(), Error> {
    let (_, bus) = make_bus();
    let mb: &dyn MessageBus = bus.as_ref();

    let _first = mb.create_order_update_subscription()?;
    let err = mb.create_order_update_subscription().expect_err("duplicate fails");
    assert!(matches!(err, Error::AlreadySubscribed), "got: {err:?}");
    Ok(())
}

/// Warning code (2104) bound to a real request_id is delivered as a
/// `RoutedItem::Notice` to the owning subscription — stream stays open.
#[test]
fn test_warning_with_request_id_delivers_notice() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_request(42, &[])?;

    // Old-format Error: msg_id=4, version=2, request_id=42, code=2104, message=...
    stream.push_inbound(error_frame(42, 2104, FARM_OK_MSG));
    bus.dispatch()?;

    let item = sub.next_timeout_routed(TICK).expect("notice not delivered");
    match item {
        RoutedItem::Notice(notice) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, "Market data farm connection is OK:usfarm");
        }
        other => panic!("expected RoutedItem::Notice, got {other:?}"),
    }

    // Stream stays open: a follow-up send delivers normally.
    stream.push_inbound(body("89|42|payload|"));
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
    let sub = bus.send_request(42, &[])?;

    let code = crate::messages::DATA_ADVISORY_CODES[1]; // 10167
    stream.push_inbound(error_frame(42, code, "Displaying delayed market data."));
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
    stream.push_inbound(body("89|42|payload|"));
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
    let sub = bus.send_request(42, &[])?;

    stream.push_inbound(error_frame(42, 200, "No security definition found"));
    bus.dispatch()?;

    let item = sub.next_timeout_routed(TICK).expect("error not delivered");
    match item {
        RoutedItem::Error(Error::Notice(notice)) => {
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
    let sub = bus.send_request(42, &[])?;

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
    // `one_shot_request` helper consume — so a `Some(Err(..))` surfaces to callers.
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

/// A one-shot `send_shared_request` drains the shared queue before writing:
/// a request-less error buffered while no request was in flight must not
/// poison the next call, which reads only its own response.
#[test]
fn test_one_shot_shared_request_drains_stale_buffered_error() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    // Hard error arrives with no request in flight; fanned to the persistent
    // one-shot senders, it buffers in the RequestIds shared queue.
    stream.push_inbound(error_frame(-1, 321, READ_ONLY_MSG));
    bus.dispatch()?;

    // The next one-shot request drains the stale error and reads only its own response.
    let one_shot = bus.send_shared_request(OutgoingMessages::RequestIds, &[])?;
    assert!(one_shot.try_next_routed().is_none(), "stale buffered error must be drained");

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::NextValidId as i32,
        &crate::proto::NextValidId { order_id: Some(90) },
    ));
    bus.dispatch()?;

    let message = one_shot.next_timeout(TICK).expect("one-shot response missing")?;
    assert_eq!(message.message_type(), crate::messages::IncomingMessages::NextValidId);
    Ok(())
}

/// Streaming `send_shared_request` must NOT drain the shared queue: sync
/// shares one crossbeam queue per request type, so draining could discard
/// messages buffered for a concurrent live subscription of the same type.
#[test]
fn test_streaming_shared_request_does_not_drain_buffered_items() -> Result<(), Error> {
    let (stream, bus) = make_bus();

    // A message buffers in the RequestPositions shared queue (e.g. delivered for a
    // concurrent subscription of the same type that hasn't consumed it yet).
    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::PositionEnd as i32,
        &crate::proto::PositionEnd {},
    ));
    bus.dispatch()?;

    // A new streaming request of the same type must leave the buffered item intact.
    let streaming = bus.send_shared_request(OutgoingMessages::RequestPositions, &[])?;
    let message = streaming.next_timeout(TICK).expect("buffered streaming message was drained")?;
    assert_eq!(message.message_type(), crate::messages::IncomingMessages::PositionEnd);
    Ok(())
}

/// Order-channel fallback: a notice arrives bound to an `order_id` that
/// matches an order subscription (not a request subscription). The
/// `deliver_to_request_id` helper should fall back to the order channel.
#[test]
fn test_warning_with_order_id_falls_back_to_order_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_order_request(7, &[])?;

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

const FARM_OK_MSG: &str = "Market data farm connection is OK:usfarm";
const READ_ONLY_MSG: &str = "The API interface is currently in Read-Only mode.";

fn farm_ok_frame_42() -> Vec<u8> {
    error_frame(42, 2104, FARM_OK_MSG)
}

fn farm_ok_frame_unrouted() -> Vec<u8> {
    error_frame(-1, 2104, FARM_OK_MSG)
}

#[derive(Debug)]
struct NoticeTestData;

impl crate::subscriptions::StreamDecoder<NoticeTestData> for NoticeTestData {
    fn decode(_context: &crate::subscriptions::DecoderContext, _msg: &mut ResponseMessage) -> Result<NoticeTestData, Error> {
        Ok(NoticeTestData)
    }
}

fn wrap_subscription(
    bus: Arc<TcpMessageBus<MemoryStream>>,
    internal: InternalSubscription,
) -> crate::subscriptions::sync::Subscription<NoticeTestData> {
    crate::subscriptions::sync::Subscription::new(bus, internal, crate::subscriptions::DecoderContext::default())
}

type NoticeFixture = (
    MemoryStream,
    Arc<TcpMessageBus<MemoryStream>>,
    crate::subscriptions::sync::Subscription<NoticeTestData>,
);

fn make_request_subscription(request_id: i32) -> Result<NoticeFixture, Error> {
    let (stream, bus) = make_bus();
    let internal = bus.send_request(request_id, &[])?;
    let sub = wrap_subscription(bus.clone(), internal);
    Ok((stream, bus, sub))
}

fn make_order_subscription(order_id: i32) -> Result<NoticeFixture, Error> {
    let (stream, bus) = make_bus();
    let internal = bus.send_order_request(order_id, &[])?;
    let sub = wrap_subscription(bus.clone(), internal);
    Ok((stream, bus, sub))
}

/// Code 2104 + request_id=42 surfaces as `SubscriptionItem::Notice` without
/// terminating; a follow-up data message arrives normally on the same stream.
#[test]
fn test_subscription_notice_delivery_request_keyed() -> Result<(), Error> {
    use crate::subscriptions::SubscriptionItem;

    let (stream, bus, subscription) = make_request_subscription(42)?;

    stream.push_inbound(farm_ok_frame_42());
    bus.dispatch()?;

    match subscription.next_timeout(TICK) {
        Some(Ok(SubscriptionItem::Notice(notice))) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, FARM_OK_MSG);
        }
        other => panic!("expected SubscriptionItem::Notice, got {other:?}"),
    }

    stream.push_inbound(body("89|42|payload|"));
    bus.dispatch()?;
    match subscription.next_timeout(TICK) {
        Some(Ok(SubscriptionItem::Data(_))) => {}
        other => panic!("expected SubscriptionItem::Data, got {other:?}"),
    }
    Ok(())
}

/// Hard error (code 200) surfaces as `Some(Err(_))`; subsequent reads return `None`.
#[test]
fn test_subscription_hard_error_terminates_stream() -> Result<(), Error> {
    let (stream, bus, subscription) = make_request_subscription(42)?;

    stream.push_inbound(error_frame(42, 200, "No security definition found"));
    bus.dispatch()?;

    match subscription.next_timeout(TICK) {
        Some(Err(Error::Notice(notice))) => {
            assert_eq!(notice.code, 200);
            assert_eq!(notice.message, "No security definition found");
        }
        other => panic!("expected Some(Err(Error::Notice)), got {other:?}"),
    }

    assert!(subscription.next_timeout(TICK).is_none(), "stream must end after terminal error");
    Ok(())
}

/// Order-keyed notice via `deliver_to_request_id`'s order-channel fallback.
#[test]
fn test_subscription_notice_delivery_order_keyed() -> Result<(), Error> {
    use crate::subscriptions::SubscriptionItem;

    let (stream, bus, subscription) = make_order_subscription(7)?;

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
    let (stream, bus, subscription) = make_request_subscription(42)?;

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
    let (stream, bus, subscription) = make_request_subscription(42)?;

    stream.push_inbound(body("89|42|first|"));
    stream.push_inbound(farm_ok_frame_42());
    stream.push_inbound(body("89|42|second|"));
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
    let request_sub = bus.send_request(42, &[])?;

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

/// Proto-framed ExecutionData fixture. `request_id` is at proto tag 1; the
/// dispatcher's `order_id` / `execution_id` accessors read the nested
/// `execution.{order_id, exec_id}` sub-message via `ExecutionDetailsMinimal`.
fn execution_data_body(request_id: i32, order_id: i32, execution_id: &str) -> Vec<u8> {
    binary_proto(
        crate::messages::IncomingMessages::ExecutionData as i32,
        &crate::proto::ExecutionDetails {
            req_id: Some(request_id),
            contract: None,
            execution: Some(crate::proto::Execution {
                order_id: Some(order_id),
                exec_id: Some(execution_id.to_string()),
                ..Default::default()
            }),
        },
    )
}

#[test]
fn test_execution_data_routes_to_order_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_order_request(7, &[])?;

    stream.push_inbound(execution_data_body(99, 7, "exec-1"));
    bus.dispatch()?;

    let msg = sub.next_timeout(TICK).expect("order sub got no message")?;
    assert_eq!(msg.message_type(), crate::messages::IncomingMessages::ExecutionData);
    assert_eq!(msg.order_id(), Some(7));
    Ok(())
}

#[test]
fn test_execution_data_falls_back_to_request_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_request(99, &[])?;

    stream.push_inbound(execution_data_body(99, 7, "exec-1"));
    bus.dispatch()?;

    let msg = sub.next_timeout(TICK).expect("request sub got no message")?;
    assert_eq!(msg.request_id(), Some(99));
    Ok(())
}

#[test]
fn test_execution_data_end_routes_to_order_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_order_request(7, &[])?;

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
/// request subscription on the same id catches it via the order-channel-miss
/// fallback to the request channel.
#[test]
fn test_execution_data_end_falls_back_to_request_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let sub = bus.send_request(7, &[])?;

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::ExecutionDataEnd as i32,
        &crate::proto::ExecutionDetailsEnd { req_id: Some(7) },
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
    let sub = bus.send_order_request(7, &[])?;

    stream.push_inbound(execution_data_body(99, 7, "exec-abc"));
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
    let order_sub = bus.send_order_request(42, &[])?;
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

/// Drop signals exercise `clean_request` / `clean_order` / `clear_order_update_stream`.
/// The cleanup thread is signal-driven; we poll with a deadline rather than
/// adding an ack channel to production code.
#[test]
fn test_cleanup_thread_processes_drop_signals() -> Result<(), Error> {
    let (_, bus) = make_bus();
    let handle = bus.start_cleanup_thread();

    let req = bus.send_request(42, &[])?;
    let order = bus.send_order_request(99, &[])?;
    let stream_sub = bus.create_order_update_subscription()?;

    drop(req);
    drop(order);
    drop(stream_sub);

    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline && (bus.requests.contains(&42) || bus.orders.contains(&99) || bus.order_update_stream.lock().unwrap().is_some()) {
        std::thread::sleep(Duration::from_millis(2));
    }

    assert!(!bus.requests.contains(&42), "request 42 not cleaned");
    assert!(!bus.orders.contains(&99), "order 99 not cleaned");
    assert!(bus.order_update_stream.lock().unwrap().is_none(), "order update stream not cleared");

    bus.request_shutdown();
    handle.join().expect("cleanup thread join");
    Ok(())
}

/// Routed-but-orphan notice (real request_id, no matching sub) takes the
/// `log_orphan` path, NOT the global notice stream.
#[test]
fn test_warning_with_orphan_request_id_logs() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let unrelated = bus.send_request(42, &[])?;
    let notice_stream = bus.notice_subscribe();

    stream.push_inbound(error_frame(99, 2104, "orphan warning"));
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

/// Cancel for an unknown id still writes the cancel bytes through (no-op
/// otherwise — there's no in-flight subscription to notify).
#[test]
fn test_cancel_unknown_subscription_writes_through() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let mb: &dyn MessageBus = bus.as_ref();

    mb.cancel_subscription(7777, b"cancel-bytes")?;

    let captured = stream.captured();
    assert!(captured.windows(b"cancel-bytes".len()).any(|w| w == b"cancel-bytes"));
    Ok(())
}

/// `ExecutionData` with no matching order or request subscription falls
/// through both branches; an unrelated subscription must not see it.
#[test]
fn test_execution_data_orphan_dropped() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let unrelated = bus.send_request(42, &[])?;

    stream.push_inbound(execution_data_body(99, 7, "exec-1"));
    bus.dispatch()?;

    assert!(unrelated.try_next().is_none(), "unrelated sub got an orphan message");
    Ok(())
}

#[test]
fn test_execution_data_end_orphan_dropped() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let unrelated = bus.send_request(42, &[])?;

    stream.push_inbound(binary_proto(
        crate::messages::IncomingMessages::ExecutionDataEnd as i32,
        &crate::proto::ExecutionDetailsEnd { req_id: Some(999) },
    ));
    bus.dispatch()?;

    assert!(unrelated.try_next().is_none(), "unrelated sub got an orphan end");
    Ok(())
}

#[test]
fn test_commission_report_without_mapping_dropped() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let unrelated = bus.send_order_request(7, &[])?;

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

/// `process_response_with_id` orders-fallback: a non-order message
/// (HistogramData) whose request_id collides with an order subscription's id
/// still gets routed to the order channel.
#[test]
fn test_response_falls_back_to_order_channel() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let order_sub = bus.send_order_request(7, &[])?;

    stream.push_inbound(body("89|7|payload|"));
    bus.dispatch()?;

    order_sub.next_timeout(TICK).expect("order sub got no message")?;
    Ok(())
}

#[test]
fn test_response_with_no_recipient_dropped() -> Result<(), Error> {
    let (stream, bus) = make_bus();
    let unrelated = bus.send_request(42, &[])?;

    stream.push_inbound(body("89|999|payload|"));
    bus.dispatch()?;

    assert!(unrelated.try_next().is_none(), "unrelated sub got a stray message");
    Ok(())
}

/// `reset` notifies every channel category — requests, orders, shared — and
/// clears the channel maps. All three categories must be live before the call
/// to exercise each `notify_all` branch.
#[test]
fn test_reset_notifies_all_channel_categories() -> Result<(), Error> {
    let (_, bus) = make_bus();

    let req = bus.send_request(100, &[])?;
    let order = bus.send_order_request(200, &[])?;
    let shared = bus.send_shared_request(OutgoingMessages::RequestCurrentTime, &[])?;

    bus.reset();

    for (name, sub) in [("request", &req), ("order", &order), ("shared", &shared)] {
        let resp = sub.next_timeout(TICK).unwrap_or_else(|| panic!("{name} sub got no notification"));
        assert!(matches!(resp, Err(Error::ConnectionReset)), "{name}: {resp:?}");
    }

    assert!(!bus.requests.contains(&100));
    assert!(!bus.orders.contains(&200));
    Ok(())
}
