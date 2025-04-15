use crate::client::Client;
use crate::contracts::encoders::encode_request_contract_data;
use crate::contracts::Contract;
use crate::errors::Error;
use crate::messages::{encode_length, RequestMessage, ResponseMessage};
use crate::orders::encoders::encode_place_order;
use crate::orders::{order_builder, Action};
use crate::transport::{read_message, Connection, Io, MessageBus, Reconnect, Stream, TcpMessageBus, MAX_RETRIES};
use std::io::ErrorKind;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::{debug, trace};
use std::collections::VecDeque;
use std::sync::Arc;

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
    // Mutated by writer threads
    exchange_index: AtomicUsize,

    // Mutated from reader thread
    read_call_count: AtomicUsize,

    // Mutated by writer threads
    write_call_count: AtomicUsize,
}

impl MockSocket {
    pub fn new(exchanges: Vec<Exchange>, expected_retries: usize) -> Self {
        Self {
            exchanges,
            expected_retries,
            reconnect_call_count: AtomicUsize::new(0),

            exchange_index: AtomicUsize::new(0),
            read_call_count: AtomicUsize::new(0),
            write_call_count: AtomicUsize::new(0),
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
    fn sleep(&self, duration: std::time::Duration) {}
}

impl Stream for MockSocket {}

impl Io for MockSocket {
    fn read_message(&self) -> Result<Vec<u8>, Error> {
        let exchange_index = self.exchange_index.load(Ordering::SeqCst);
        let exchange = match self.exchanges.get(exchange_index) {
            Some(ex) => ex,
            None => {
                // keep alive
                return Err(mock_socket_error(ErrorKind::WouldBlock));
            }
        };

        let response_index = self.read_call_count.load(Ordering::SeqCst);
        let responses = &exchange.responses;
        let new_response_index = (response_index + 1) % responses.len();

        trace!(
            "mock read: responses.len(): {}, response_index: {}, new_response_index: {}, exchange_index: {}",
            responses.len(),
            self.read_call_count.load(Ordering::SeqCst),
            new_response_index,
            self.exchange_index.load(Ordering::SeqCst)
        );

        let response = responses.get(response_index).unwrap();

        self.read_call_count.store(new_response_index, Ordering::SeqCst);

        // disconnect if a null byte response is encountered
        if response.fields[0] == "\0" {
            return Err(mock_socket_error(ErrorKind::ConnectionReset));
        }

        // process the declared response in the test with transport read_message()
        // to force any errors
        let encoded = response.encode();
        debug!("mock -> {:?}", &encoded);
        let expected = encode_length(&encoded);
        Ok(read_message(&mut expected.as_slice())?)
    }

    fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        // do not increment exchange_index on first call
        // otherwise increment
        let write_call_count = self.write_call_count.load(Ordering::SeqCst);
        if write_call_count > 0 {
            self.exchange_index.fetch_add(1, Ordering::SeqCst);
        }
        let exchange_index = self.exchange_index.load(Ordering::SeqCst);

        trace!("mock write: exchange_index: {exchange_index}, write_call_count: {write_call_count}");

        let exchange = self.exchanges.get(exchange_index).unwrap();
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
        debug!("mock -> {:?}", raw_string);

        assert_eq!(
            expected,
            buf,
            "assertion left == right failed\nleft: {:?}\n right: {:?}\n",
            std::str::from_utf8(expected).unwrap(),
            std::str::from_utf8(buf).unwrap()
        );

        self.write_call_count.fetch_add(1, Ordering::SeqCst);

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
// #[ignore = "TODO"]
fn test_bus_send_order_request() -> Result<(), Error> {
    let order = order_builder::market_order(Action::Buy, 100.0);
    let contract = &Contract::stock("AAPL");
    let request = encode_place_order(176, 5, contract, &order)?;

    let events = vec![
        Exchange::simple("v100..173", &["173|20250415 19:38:30 British Summer Time|"]),
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|5|"]),
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
            "71|2|28|",
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
    Connection::connect(stream, 28)?;

    Ok(())
}

#[test]
fn test_reconnect_failed() -> Result<(), Error> {
    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
    ];
    let socket = MockSocket::new(events, MAX_RETRIES as usize + 1);

    let connection = Connection::connect(socket, 28)?;

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
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
    ];
    let socket = MockSocket::new(events, MAX_RETRIES as usize - 1);

    let connection = Connection::connect(socket, 28)?;

    // simulated dispatcher thread read to trigger disconnection
    let _ = connection.read_message();

    Ok(connection.reconnect()?)
}

// TODO: test takes minimum 1 sec due to signal_recv.recv_timeout(Duration::from_secs(1)) in
// MessageBus::start_cleanup_thread()
#[test]
#[ignore = "TODO"]
fn test_client_reconnect() -> Result<(), Error> {
    // TODO: why 17|1 and not 17|1| for a shared request to assert true in MockSocket write_all ??
    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Exchange::simple("17|1", &["\0"]), // ManagedAccounts RESTART
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Exchange::simple("17|1", &["15|1|DU1234567|"]), // ManagedAccounts
    ];
    let stream = MockSocket::new(events, 0);
    let connection = Connection::connect(stream, 28)?;
    let server_version = connection.server_version();
    let bus = Arc::new(TcpMessageBus::new(connection)?);
    bus.process_messages(server_version)?;
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
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Exchange::request(packet.clone(), &[expected_response, "52|1|9001|"]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::connect(stream, 28)?;
    let server_version = connection.server_version();
    let bus = TcpMessageBus::new(connection)?;

    bus.dispatch(server_version)?;

    let subscription = bus.send_request(9000, &packet)?;

    bus.dispatch(server_version)?;
    bus.dispatch(server_version)?;

    subscription.next().unwrap()?;

    // TODO: assert after encoding is fixed
    // assert_eq!(&result.encode_simple(), expected_response);

    Ok(())
}

//
// // Test Error::ConnectionReset is raised on subscription.next()
// // when sending request during disconnect
#[test]
fn test_request_before_disconnect_raises_error() -> Result<(), Error> {
    let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;

    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Exchange::request(packet.clone(), &["\0"]), // RESTART
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::connect(stream, 28)?;
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
//
// // Test Error::ConnectionReset is raised on subscription.next()
// // when sending request during disconnect
#[test]
fn test_request_during_disconnect_raises_error() -> Result<(), Error> {
    env_logger::init();
    let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;

    let events = vec![
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|", "\0"]), // RESTART
        Exchange::simple("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Exchange::request(packet.clone(), &[]),
        Exchange::simple("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
    ];

    let stream = MockSocket::new(events, 0);
    let connection = Connection::connect(stream, 28)?;

    match connection.read_message() {
        Ok(_) => panic!(""),
        Err(_) => {
            debug!("reconnect");
            connection.socket.reconnect()?;
            debug!("handshake");
            connection.handshake()?;
            debug!("write_message");
            connection.write_message(&packet)?;
            debug!("start_api");
            connection.start_api()?;
            connection.receive_account_info()?;
        }
    };

    Ok(())
}
//
//
// // TODO: This test repeats test_request_during_disconnect() with the client instead
// // the response should be the same, Error::ConnectionReset
// #[test]
// #[ignore = "propagate error from contract_details() to fix"]
// fn test_client_request_during_disconnect() -> Result<(), Box<dyn std::error::Error>> {
//     let events = vec![
//         Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
//         Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
//         Event::Restart,
//         Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
//         Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
//     ];
//
//     let server = TestServer::start(events);
//
//     let client = Client::connect(&server.address().to_string(), 28).unwrap();
//
//     // sleep so the request is sent after the dispatcher thread enters the reconnection
//     // routine
//     std::thread::sleep(Duration::from_millis(1));
//
//     // now attempt to send the request
//     let contract = &Contract::stock("AAPL");
//
//     match client.contract_details(&contract) {
//         Err(Error::ConnectionReset) => {}
//         _ => panic!(),
//     }
//
//     Ok(())
// }
//
// // TODO: fix this
// #[test]
// #[ignore = ""]
// fn test_simple_encoding_roundtrip() {
//     // let res = RequestMessage::from_simple(&format!("10|9000|{}", AAPL_CONTRACT_RESPONSE));
//     let expected = "17|1|";
//
//     let req = RequestMessage::from_simple(expected);
//     let req = RequestMessage::from(&req.encode()).encode_simple();
//     assert_eq!(req, expected);
//
//     let res = RequestMessage::from_simple(expected);
//     let res = RequestMessage::from(&res.encode()).encode_simple();
//     assert_eq!(res, expected);
// }
