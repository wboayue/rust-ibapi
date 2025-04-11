use crate::client::Client;
use crate::contracts::encoders::encode_request_contract_data;
use crate::contracts::Contract;
use crate::errors::Error;
use crate::messages::{RequestMessage, ResponseMessage};
use crate::transport::{encode_packet, Connection, Io, MessageBus, Reconnect, Stream, TcpMessageBus, MAX_RETRIES};
use std::cell::Cell;
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};

use log::debug;
use std::collections::VecDeque;
use std::io::Read;
use std::sync::atomic::AtomicI32;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
struct MockSocket {
    events: Arc<Mutex<VecDeque<Event>>>,
    response_buf: Arc<Mutex<Vec<u8>>>,
    is_reconnecting: AtomicBool,
    reconnect_call_count: AtomicI32,
    expected_retries: i32,
}

impl MockSocket {
    pub fn new(events: Vec<Event>, expected_retries: i32) -> Self {
        Self {
            events: Arc::new(Mutex::new(VecDeque::from(events))),
            response_buf: Arc::new(Mutex::new(vec![0_u8; 0])),
            is_reconnecting: AtomicBool::new(false),
            reconnect_call_count: AtomicI32::new(0),
            expected_retries,
        }
    }
    fn handle_write(&self, buf: &[u8], request: RequestMessage, responses: Vec<ResponseMessage>) {
        let raw_string = str::from_utf8(buf).unwrap();
        debug!("mock <- {:?}", raw_string);

        // remove API\0 to assert against the length encoded portion of the handshake
        match buf.starts_with(b"API\0") {
            true => {
                let encoded = request.encode();
                let length_encoded = encode_packet(&encoded[..encoded.len() - 1]);
                assert_eq!(&raw_string[4..], &length_encoded);
            }
            false => {
                let encoded = encode_packet(&request.encode());
                assert_eq!(&raw_string, &encoded);
            }
        };

        let mut response_buf = self.response_buf.lock().unwrap();
        for res in responses {
            let encoded = res.encode();
            let length_encoded = encode_packet(&encoded);
            response_buf.extend_from_slice(length_encoded.as_bytes());
        }
    }
}

impl Reconnect for MockSocket {
    fn reconnect(&self) -> Result<(), Error> {
        let is_reconnecting = self.is_reconnecting.load(Ordering::SeqCst);
        assert!(is_reconnecting, "Reconnect called while socket connected");

        let call_count = self.reconnect_call_count.load(Ordering::SeqCst);

        if call_count == self.expected_retries {
            self.is_reconnecting.store(false, Ordering::SeqCst);
            self.reconnect_call_count.store(0, Ordering::SeqCst);
            return Ok(());
        }

        self.reconnect_call_count.store(call_count + 1, Ordering::SeqCst);

        let io_error = std::io::Error::new(ErrorKind::ConnectionRefused, "Simulated ConnectionRefused Error");
        return Err(Error::Io(Arc::new(io_error)));
    }
    fn sleep(&self, duration: std::time::Duration) {}
}

impl Stream for MockSocket {}

impl Io for MockSocket {
    fn read_exact(&self, buf: &mut [u8]) -> Result<(), Error> {
        let mut response_buf = self.response_buf.lock().unwrap();
        let events = self.events.lock().unwrap();

        let response_buf_len = response_buf.len();

        // if no events remaining & buffer is empty - test has finished, gracefully shutdown
        if response_buf_len == 0 && events.is_empty() {
            let io_error = std::io::Error::new(ErrorKind::WouldBlock, "Simulated WouldBlock error");
            debug!("mock -> {}", io_error);
            return Err(Error::Io(Arc::new(io_error)));
        }

        // If the next Event is Event::Restart & buffer is empty - restart
        if response_buf_len == 0 && self.is_reconnecting.load(Ordering::SeqCst) {
            let io_error = std::io::Error::new(ErrorKind::ConnectionReset, "Simulated ConnectionReset error");
            debug!("mock -> {}", io_error);
            return Err(Error::Io(Arc::new(io_error)));
        }

        if response_buf_len > 0 {
            let response_slice = &response_buf[..buf.len()];
            buf.copy_from_slice(response_slice);
            debug!("mock -> {:?}", str::from_utf8(response_slice).unwrap());
            response_buf.drain(..buf.len()).for_each(drop);
        }

        Ok(())
    }
    fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        let mut events = self.events.lock().unwrap();
        assert!(
            !events.is_empty(),
            "write() called with no events remaining - The test is scheduled incorrectly."
        );

        let event = events.pop_front().unwrap();
        match event {
            Event::Exchange { request, responses } => {
                self.handle_write(buf, request, responses);

                // handle Event::Restart after the last client write
                if let Some(event) = events.front() {
                    if let Event::Restart = event {
                        events.pop_front();
                        debug!("restart will happen on next read");
                        self.is_reconnecting.store(true, Ordering::SeqCst);
                    }
                }
            }
            Event::Restart => panic!("events vector scheduled incorrectly"),
        };
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct MockExchange {
    request: RequestMessage,
    responses: VecDeque<ResponseMessage>,
}

#[derive(Clone, Debug)]
enum Event {
    Restart,
    Exchange {
        request: RequestMessage,
        responses: Vec<ResponseMessage>,
    },
}

impl Event {
    fn from(request: &str, responses: &[&str]) -> Self {
        let responses = responses
            .into_iter()
            .map(|s| ResponseMessage::from_simple(s))
            .collect::<Vec<ResponseMessage>>();
        Event::Exchange {
            request: RequestMessage::from_simple(request),
            responses,
        }
    }
    fn from_request(request: RequestMessage, responses: &[&str]) -> Self {
        let responses = responses
            .into_iter()
            .map(|s| ResponseMessage::from_simple(s))
            .collect::<Vec<ResponseMessage>>();
        Event::Exchange { request, responses }
    }
}

#[test]
#[ignore = "TODO"]
fn test_order_request() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[test]
fn test_connection_establish_connection() -> Result<(), Error> {
    env_logger::init();
    let events = vec![
        Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::from(
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
fn test_bus_reconnect_failed() -> Result<(), Error> {
    let events = vec![
        Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::from("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::Restart,
    ];
    let socket = MockSocket::new(events, MAX_RETRIES + 1);

    let connection = Connection::connect(socket, 28)?;

    match connection.reconnect() {
        Err(Error::ConnectionFailed) => return Ok(()),
        _ => panic!(""),
    }
}

#[test]
fn test_bus_reconnect_success() -> Result<(), Error> {
    let events = vec![
        Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::from("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::Restart,
        Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::from("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
    ];
    let socket = MockSocket::new(events, MAX_RETRIES - 1);

    let connection = Connection::connect(socket, 28)?;

    Ok(connection.reconnect()?)
}

// TODO: test takes minimum 1 sec due to signal_recv.recv_timeout(Duration::from_secs(1)) in
// MessageBus::start_cleanup_thread()
#[test]
fn test_client_reconnect() -> Result<(), Error> {
    // TODO: why 17|1 and not 17|1| for a shared request to assert true in MockSocket write_all ??
    let events = vec![
        Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::from("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::from("17|1", &[]), // ManagedAccounts
        Event::Restart,
        Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::from("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::from("17|1", &["15|1|DU1234567|"]), // ManagedAccounts
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
        Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::from("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::Restart,
        Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::from("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::from_request(packet.clone(), &[expected_response, "52|1|9001|"]),
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

// const NEWS_RESPONSE: &str = "85|08|0BRFG|Briefing.com General Market Columns|BRFUPDN|Briefing.com Analyst Actions|DJ-N|Dow Jones News Service|DJ-RTA|Dow Jones Real-Time News Asia Pacific|DJ-RTE|Dow Jones Real-Time News Europe|DJ-RTG|Dow Jones Real-Time News Global|DJ-RTPRO|Dow Jones Real-Time News Pro|DJNL|Dow Jones Newsletters|";
// #[test]
// fn test_shared_request() -> Result<(), Box<dyn std::error::Error>> {
//     env_logger::init();
//
//     let events = vec![
//         Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
//         Event::from("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
//         Event::from("|85|", &[NEWS_RESPONSE]),
//     ];
//
//     let client = Client::connect("192.168.0.5:4002", 28).expect("connection failed");
//
//     client.news_providers().expect("request news providers failed");
//
//     Ok(())
// }

//
// // Test Error::ConnectionReset is raised on subscription.next()
// // when sending request during disconnect
// #[test]
// fn test_request_before_disconnect_raises_error() -> Result<(), Box<dyn std::error::Error>> {
//     let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;
//
//     let events = vec![
//         Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
//         Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
//         Event::Message(MockExchange::Request {
//             request: packet.clone(),
//             responses: vec![],
//         }),
//         Event::Restart,
//         Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
//         Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
//     ];
//
//     let server = TestServer::start(events);
//
//     let connection = Connection::connect(28, &server.address().to_string())?;
//     let server_version = connection.server_version();
//     let bus = Arc::new(TcpMessageBus::new(connection)?);
//
//     bus.process_messages(server_version)?;
//
//     let subscription = bus.send_request(9000, &packet)?;
//
//     match subscription.next() {
//         Some(Err(Error::ConnectionReset)) => {}
//         _ => panic!(),
//     }
//
//     Ok(())
// }
//
// // Test Error::ConnectionReset is raised on subscription.next()
// // when sending request during disconnect
// #[test]
// fn test_request_during_disconnect_raises_error() -> Result<(), Box<dyn std::error::Error>> {
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
//     let connection = Connection::connect(28, &server.address().to_string())?;
//     let server_version = connection.server_version();
//     let bus = Arc::new(TcpMessageBus::new(connection)?);
//
//     bus.process_messages(server_version)?;
//
//     // sleep so the request is sent after the dispatcher thread enters the reconnection
//     // routine
//     std::thread::sleep(Duration::from_millis(1));
//
//     // now attempt to send the request
//     let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;
//     let subscription = bus.send_request(9000, &packet)?;
//
//     match subscription.next() {
//         Some(Err(Error::ConnectionReset)) => {}
//         _ => panic!(),
//     }
//
//     Ok(())
// }
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
