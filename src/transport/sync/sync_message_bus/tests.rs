use crate::transport::TcpSocket;
use time::macros::datetime;
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt};

use crate::messages::ResponseMessage;
use crate::tests::assert_send_and_sync;

use super::*;

// Additional imports for connection tests
use crate::client::Client;
use crate::contracts::encoders::encode_request_contract_data;
use crate::contracts::Contract;
use crate::messages::{encode_length, RequestMessage};
use crate::orders::encoders::encode_place_order;
use crate::orders::{order_builder, Action};
use log::{debug, trace};
use std::collections::VecDeque;
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_thread_safe() {
    assert_send_and_sync::<Connection<TcpSocket>>();
    assert_send_and_sync::<TcpMessageBus<TcpSocket>>();
}

#[test]
fn test_parse_connection_time() {
    let example = "20230405 22:20:39 PST";
    let (connection_time, _) = parse_connection_time(example);

    let la = timezones::db::america::LOS_ANGELES;
    if let OffsetResult::Some(other) = datetime!(2023-04-05 22:20:39).assume_timezone(la) {
        assert_eq!(connection_time, Some(other));
    }
}

#[test]
fn test_fibonacci_backoff() {
    let mut backoff = FibonacciBackoff::new(10);

    assert_eq!(backoff.next_delay(), Duration::from_secs(1));
    assert_eq!(backoff.next_delay(), Duration::from_secs(2));
    assert_eq!(backoff.next_delay(), Duration::from_secs(3));
    assert_eq!(backoff.next_delay(), Duration::from_secs(5));
    assert_eq!(backoff.next_delay(), Duration::from_secs(8));
    assert_eq!(backoff.next_delay(), Duration::from_secs(10));
    assert_eq!(backoff.next_delay(), Duration::from_secs(10));
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
    Error::Io(Arc::new(io_error))
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
        return Err(mock_socket_error(ErrorKind::ConnectionRefused));
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

        // if there are no more remaining exchanges or responses
        // set keep_alive - so the client can gracefully disconnect
        if write_call_count >= self.exchanges.len() && read_call_count >= responses.len() - 1 {
            self.keep_alive.store(true, Ordering::SeqCst);
        }

        self.read_call_count.fetch_add(1, Ordering::SeqCst);

        // process the declared response in the test with transport read_message()
        // to force any errors
        let encoded = response.encode();
        debug!("mock read {:?}", &encoded);
        let expected = encode_length(&encoded);
        Ok(read_message(&mut expected.as_slice())?)
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
            &buf
        };

        // the handshake does not include the trailing null byte
        // Message encode() cannot be used to encode the handshake
        let expected = if is_handshake {
            assert_eq!(request.len(), 1);
            &encode_length(&request.fields[0])
        } else {
            &encode_length(&request.encode())
        };

        let raw_string = std::str::from_utf8(&buf[4..]).unwrap(); // strip length
        debug!("mock write {:?}", raw_string);

        assert_eq!(
            expected,
            buf,
            "assertion left == right failed\nexpected: {:?}\nbuf: {:?}\n",
            std::str::from_utf8(expected).unwrap(),
            std::str::from_utf8(buf).unwrap()
        );

        self.read_call_count.store(0, Ordering::SeqCst);
        self.write_call_count.fetch_add(1, Ordering::SeqCst);
        self.responses_len.store(exchange.responses.len(), Ordering::SeqCst);

        Ok(())
    }
}

#[derive(Debug)]
struct Exchange {
    request: RequestMessage,
    responses: VecDeque<ResponseMessage>,
}

impl Exchange {
    fn new(request: RequestMessage, responses: Vec<ResponseMessage>) -> Self {
        Self {
            request,
            responses: VecDeque::from(responses),
        }
    }
    fn simple(request: &str, responses: &[&str]) -> Self {
        let responses = responses
            .into_iter()
            .map(|s| ResponseMessage::from_simple(s))
            .collect::<Vec<ResponseMessage>>();
        Self::new(RequestMessage::from_simple(request), responses)
    }
    fn request(request: RequestMessage, responses: &[&str]) -> Self {
        let responses = responses
            .into_iter()
            .map(|s| ResponseMessage::from_simple(s))
            .collect::<Vec<ResponseMessage>>();
        Self::new(request, responses)
    }
}

#[test]
fn test_bus_send_order_request() -> Result<(), Error> {
    let order = order_builder::market_order(Action::Buy, 100.0);
    let contract = &Contract::stock("AAPL");
    let request = encode_place_order(176, 5, contract, &order)?;

    let events = vec![
        Exchange::simple("v100..173", &["173|20250415 19:38:30 British Summer Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|5|"]),
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
    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple(
            "71|2|28||",
            &[
                "15|1|DU1234567|",
                "9|1|1|",
                "4|2|-1|2104|Market data farm connection is OK:usfarm||",
                "4|2|-1|2107|HMDS data farm connection is inactive but should be available upon demand.ushmds||",
                "4|2|-1|2158|Sec-def data farm connection is OK:secdefil||",
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
    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
    ];
    let socket = MockSocket::new(events, MAX_RETRIES as usize + 1);

    let connection = Connection::stubbed(socket, 28);
    connection.establish_connection()?;

    // simulated dispatcher thread read to trigger disconnection
    let _ = connection.read_message();

    match connection.reconnect() {
        Err(Error::ConnectionFailed) => return Ok(()),
        _ => panic!(""),
    }
}

#[test]
fn test_reconnect_success() -> Result<(), Error> {
    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
    ];
    let socket = MockSocket::new(events, MAX_RETRIES as usize - 1);

    let connection = Connection::stubbed(socket, 28);
    connection.establish_connection()?;

    // simulated dispatcher thread read to trigger disconnection
    let _ = connection.read_message();

    Ok(connection.reconnect()?)
}

#[test]
fn test_client_reconnect() -> Result<(), Error> {
    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
        Exchange::simple("17|1|", &["\0"]), // ManagedAccounts RESTART
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
        Exchange::simple("17|1|", &["15|1|DU1234567|"]), // ManagedAccounts
    ];
    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
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
    let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;

    let expected_response = &format!("10|9000|{}", AAPL_CONTRACT_RESPONSE);

    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
        Exchange::request(packet.clone(), &[expected_response, "52|1|9001|"]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
    let server_version = connection.server_version();
    let bus = TcpMessageBus::new(connection)?;

    bus.dispatch(server_version)?;

    let subscription = bus.send_request(9000, &packet)?;

    bus.dispatch(server_version)?;
    bus.dispatch(server_version)?;

    let result = subscription.next().unwrap()?;

    assert_eq!(&result.encode_simple(), expected_response);

    Ok(())
}

// If a request is sent before a restart
// the waiter should receive Error::ConnectionReset
#[test]
fn test_request_before_disconnect_raises_error() -> Result<(), Error> {
    let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;

    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
        Exchange::request(packet.clone(), &["\0"]), // RESTART
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
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
    let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;

    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::request(packet.clone(), &[]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
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
    let contract = &Contract::stock("AAPL");

    let packet = encode_request_contract_data(173, 9000, contract)?;

    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
        Exchange::request(packet.clone(), &["\0"]),
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28||", &["15|1|DU1234567|", "9|1|1|"]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::stubbed(stream, 28);
    connection.establish_connection()?;
    let server_version = connection.server_version();
    let bus = Arc::new(TcpMessageBus::new(connection)?);
    bus.process_messages(server_version, std::time::Duration::from_secs(0))?;
    let client = Client::stubbed(bus.clone(), server_version);

    match client.contract_details(&contract) {
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
