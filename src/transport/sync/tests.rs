use super::*;
use crate::connection::common::{ConnectionHandler, ConnectionProtocol};
use crate::connection::sync::Connection;
use crate::tests::assert_send_and_sync;
use crate::transport::common::MAX_RECONNECT_ATTEMPTS;

// Additional imports for connection tests
use crate::client::sync::Client;
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
use std::time::Duration;

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

#[test]
fn test_error_event_warning_handling() {
    // Test that warning error codes (2100-2169) are handled correctly
    let server_version = 100;

    // Create a warning message (error code 2104 is a common warning)
    // Format: "4|2|123|2104|Market data farm connection is OK:usfarm.nj"
    let warning_message = ResponseMessage::from_simple("4|2|123|2104|Market data farm connection is OK:usfarm.nj");

    // This should not panic and should handle as a warning
    let result = error_event(server_version, warning_message);
    assert!(result.is_ok());

    // Test actual error (non-warning code)
    // Format: "4|2|456|200|No security definition has been found"
    let error_message = ResponseMessage::from_simple("4|2|456|200|No security definition has been found");

    // This should also not panic and should handle as an error
    let result = error_event(server_version, error_message);
    assert!(result.is_ok());
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
        // All other responses use binary-text format (4-byte BE msg_id + text payload).
        if exchange.is_handshake {
            let expected = encode_length(&encoded);
            read_message(&mut expected.as_slice())
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

#[test]
fn test_bus_send_order_request() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let order = order_builder::market_order(Action::Buy, 100.0);
    let contract = &Contract::stock("AAPL").build();
    let request = encode_place_order(5, contract, &order)?;

    let events = vec![
        Exchange::simple("v201..221", &[&format!("{sv}|20250415 19:38:30 British Summer Time|")]),
        Exchange::new(start_api_bytes, vec![
            ResponseMessage::from_simple("15|1|DU1234567|"),
            ResponseMessage::from_simple("9|1|5|"),
        ]),
        Exchange::request(request.clone(),
            &[
                "5|5|265598|AAPL|STK||0|?||SMART|USD|AAPL|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1234567||0||100|600745656|0|0|0||600745656.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|PreSubmitted|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||100|0.02|||",
                "3|5|PreSubmitted|0|100|0|600745656|0|0|100||0|",
                "11|-1|5|265598|AAPL|STK||0.0|||IEX|USD|AAPL|NMS|0000e0d5.67fe667b.01.01|20250415  19:38:31|DU1234567|IEX|BOT|100|201.94|600745656|100|0|100|201.94|||||2|",
                "5|5|265598|AAPL|STK||0|?||SMART|USD|AAPL|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1234567||0||100|600745656|0|0|0||600745656.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|Filled|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||100|0.02|||",
                "3|5|Filled|100|0|201.94|600745656|0|201.94|100||0|"
            ]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::connect(stream, 28)?;
    let server_version = connection.server_version();
    let bus = Arc::new(TcpMessageBus::new(connection)?);

    let subscription = bus.send_order_request(5, &request)?;

    bus.dispatch(server_version)?;
    bus.dispatch(server_version)?;
    bus.dispatch(server_version)?;
    bus.dispatch(server_version)?;
    bus.dispatch(server_version)?;

    subscription.next().unwrap()?;

    Ok(())
}

#[test]
fn test_connection_establish_connection() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let events = vec![
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes,
            vec![
                ResponseMessage::from_simple("15|1|DU1234567|"),
                ResponseMessage::from_simple("9|1|1|"),
                ResponseMessage::from_simple("4|2|-1|2104|Market data farm connection is OK:usfarm||"),
            ],
        ),
    ];
    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection(None)?;

    Ok(())
}

#[test]
fn test_reconnect_failed() -> Result<(), Error> {
    let handler = ConnectionHandler::default();
    let sv = handler.min_version;

    let start_api_bytes = handler.format_start_api(28, sv);
    let events = vec![
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes,
            vec![
                ResponseMessage::from_simple("15|1|DU1234567|"),
                ResponseMessage::from_simple("9|1|1|"),
                ResponseMessage::from_simple("\0"),
            ],
        ),
    ];
    let socket = MockSocket::new(events, MAX_RECONNECT_ATTEMPTS as usize + 1);

    let connection = Connection::stubbed(socket, 28);
    connection.establish_connection(None)?;

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
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                ResponseMessage::from_simple("15|1|DU1234567|"),
                ResponseMessage::from_simple("9|1|1|"),
                ResponseMessage::from_simple("\0"),
            ],
        ),
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes,
            vec![ResponseMessage::from_simple("15|1|DU1234567|"), ResponseMessage::from_simple("9|1|1|")],
        ),
    ];
    let socket = MockSocket::new(events, MAX_RECONNECT_ATTEMPTS as usize - 1);

    let connection = Connection::stubbed(socket, 28);
    connection.establish_connection(None)?;

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
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![ResponseMessage::from_simple("15|1|DU1234567|"), ResponseMessage::from_simple("9|1|1|")],
        ),
        Exchange::new(managed_req.clone(), vec![ResponseMessage::from_simple("\0")]),
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes,
            vec![ResponseMessage::from_simple("15|1|DU1234567|"), ResponseMessage::from_simple("9|1|1|")],
        ),
        Exchange::new(managed_req, vec![ResponseMessage::from_simple("15|1|DU1234567|")]),
    ];
    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection(None)?;
    let server_version = connection.server_version();
    let bus = Arc::new(TcpMessageBus::new(connection)?);
    bus.process_messages(server_version, std::time::Duration::from_secs(0))?;
    let client = Client::stubbed(bus.clone(), server_version);

    client.managed_accounts()?;

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
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                ResponseMessage::from_simple("15|1|DU1234567|"),
                ResponseMessage::from_simple("9|1|1|"),
                ResponseMessage::from_simple("\0"),
            ],
        ), // RESTART
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes,
            vec![ResponseMessage::from_simple("15|1|DU1234567|"), ResponseMessage::from_simple("9|1|1|")],
        ),
        Exchange::request(packet.clone(), &[expected_response, "52|1|9001|"]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection(None)?;
    let server_version = connection.server_version();
    let bus = TcpMessageBus::new(connection)?;

    bus.dispatch(server_version)?;

    let subscription = bus.send_request(9000, &packet)?;

    bus.dispatch(server_version)?;
    bus.dispatch(server_version)?;

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
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![ResponseMessage::from_simple("15|1|DU1234567|"), ResponseMessage::from_simple("9|1|1|")],
        ),
        Exchange::request(packet.clone(), &["\0"]), // RESTART
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes,
            vec![ResponseMessage::from_simple("15|1|DU1234567|"), ResponseMessage::from_simple("9|1|1|")],
        ),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection(None)?;
    let server_version = connection.server_version();
    let bus = TcpMessageBus::new(connection)?;

    let subscription = bus.send_request(9000, &packet)?;

    bus.dispatch(server_version)?;

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
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![
                ResponseMessage::from_simple("15|1|DU1234567|"),
                ResponseMessage::from_simple("9|1|1|"),
                ResponseMessage::from_simple("\0"),
            ],
        ), // RESTART
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::request(packet.clone(), &[]),
        Exchange::new(
            start_api_bytes,
            vec![ResponseMessage::from_simple("15|1|DU1234567|"), ResponseMessage::from_simple("9|1|1|")],
        ),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection(None)?;

    match connection.read_message() {
        Ok(_) => panic!(""),
        Err(_) => {
            connection.socket.reconnect()?;
            connection.handshake()?;
            connection.write_message(&packet)?;
            connection.start_api()?;
            connection.receive_account_info(None)?;
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
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes.clone(),
            vec![ResponseMessage::from_simple("15|1|DU1234567|"), ResponseMessage::from_simple("9|1|1|")],
        ),
        Exchange::request(packet.clone(), &["\0"]),
        Exchange::simple("v201..221", &[&format!("{sv}|20250323 22:21:01 Greenwich Mean Time|")]),
        Exchange::new(
            start_api_bytes,
            vec![ResponseMessage::from_simple("15|1|DU1234567|"), ResponseMessage::from_simple("9|1|1|")],
        ),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection(None)?;
    let server_version = connection.server_version();
    let bus = Arc::new(TcpMessageBus::new(connection)?);
    bus.process_messages(server_version, std::time::Duration::from_secs(0))?;
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

/// Build a text-format response body: `"msg_id|f1|f2|..."` → `b"msg_id\0f1\0f2\0..."`.
/// Pipes are stand-ins for NULs so test inputs stay readable.
fn body(text: &str) -> Vec<u8> {
    text.replace('|', "\0").into_bytes()
}

/// Wrap a fresh `MemoryStream` in a stubbed `TcpMessageBus`. server_version=0
/// keeps `parse_raw_message` on the text path.
fn make_bus() -> (MemoryStream, Arc<TcpMessageBus<MemoryStream>>) {
    let stream = MemoryStream::default();
    let connection = Connection::stubbed(stream.clone(), 28);
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

    bus.dispatch(0)?;
    bus.dispatch(0)?;

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

    // OrderStatus (msg_id 3): order_id at field index 1.
    stream.push_inbound(body("3|22|Filled|0|100|0|0|0|0|0||0|"));
    stream.push_inbound(body("3|11|Submitted|0|0|0|0|0|0|0||0|"));

    bus.dispatch(0)?;
    bus.dispatch(0)?;

    let msg_a = sub_a.next_timeout(TICK).expect("sub_a got no message")?;
    let msg_b = sub_b.next_timeout(TICK).expect("sub_b got no message")?;
    assert_eq!(msg_a.peek_int(1)?, 11);
    assert_eq!(msg_b.peek_int(1)?, 22);

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

    // OpenOrder (msg_id 5): order_id at index 1. No matching order subscription,
    // so the OrderOrShared strategy falls back to fan-out.
    stream.push_inbound(body("5|42|265598|AAPL|STK||0|||SMART|USD|AAPL|NMS|"));
    bus.dispatch(0)?;

    for (name, sub) in [("open", &sub_open), ("all", &sub_all), ("auto", &sub_auto)] {
        let msg = sub.next_timeout(TICK).unwrap_or_else(|| panic!("sub_{name} got no message"))?;
        assert_eq!(msg.peek_int(0)?, 5);
        assert_eq!(msg.peek_int(1)?, 42);
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
    bus.dispatch(0)?;

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
    let err = bus.dispatch(0).expect_err("dispatch should surface an error");
    assert!(matches!(err, Error::ConnectionFailed), "unexpected error: {err:?}");

    let resp = sub.next_timeout(TICK).expect("subscription got no notification");
    assert!(matches!(resp, Err(Error::Shutdown)), "got: {resp:?}");
    Ok(())
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
