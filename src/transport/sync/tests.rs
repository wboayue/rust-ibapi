use super::*;
use crate::connection::common::{ConnectionHandler, ConnectionProtocol};
use crate::connection::sync::Connection;
use crate::tests::assert_send_and_sync;
use crate::transport::common::MAX_RECONNECT_ATTEMPTS;

// Additional imports for connection tests
use crate::client::sync::Client;
use crate::contracts::Contract;
use crate::messages::{encode_length, RequestMessage};
use crate::orders::common::encoders::encode_place_order;
use crate::orders::{order_builder, Action};
use log::{debug, trace};
use std::collections::VecDeque;
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

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
