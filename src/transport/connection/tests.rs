use crate::client::Client;
use crate::contracts::encoders::encode_request_contract_data;
use crate::contracts::Contract;
use crate::errors::Error;
use crate::messages::{RequestMessage, ResponseMessage};
use crate::transport::{encode_packet, Connection, Io, MessageBus, Reconnect, Stream, TcpMessageBus};
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};

use log::debug;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
struct MockSocket {
    events: Arc<Mutex<VecDeque<Event>>>,
    response_buf: Arc<Mutex<Vec<u8>>>,
    restart: AtomicBool,
}

impl MockSocket {
    pub fn new(events: Vec<Event>) -> Self {
        Self {
            events: Arc::new(Mutex::new(VecDeque::from(events))),
            response_buf: Arc::new(Mutex::new(vec![0_u8; 0])),
            restart: AtomicBool::new(false),
        }
    }
    fn handle_exchange(&self, buf: &[u8], request: RequestMessage, responses: Vec<ResponseMessage>) {
        let raw_string = String::from_utf8(buf.to_vec()).unwrap();
        debug!("mock <- {:#?}", raw_string);

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

        // check the next event
        // if Event::Restart,
    }
}

impl Reconnect for MockSocket {
    fn reconnect(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl Stream for MockSocket {}

impl Io for MockSocket {
    fn read_exact(&self, buf: &mut [u8]) -> Result<(), Error> {
        let mut response_buf = self.response_buf.lock().unwrap();

        if self.restart.load(Ordering::SeqCst) && response_buf.len() == 0 {
            self.restart.store(false, Ordering::SeqCst);
            let io_error = std::io::Error::new(ErrorKind::ConnectionReset, "Simulated error");
            debug!("mock -> {}", io_error);
            return Err(Error::Io(Arc::new(io_error)));
        }

        debug!("mock -> {:#?}", String::from_utf8(response_buf.clone()).unwrap());

        buf.copy_from_slice(&response_buf[..buf.len()]);
        response_buf.drain(..buf.len()).for_each(drop);

        Ok(())
    }
    fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        let mut events = self.events.lock().unwrap();
        let event = events.pop_front().unwrap();
        match event {
            Event::Exchange { request, responses } => {
                self.handle_exchange(buf, request, responses);

                // handle Event::Restart after the last client write
                if let Some(event) = events.front() {
                    if let Event::Restart = event {
                        events.pop_front();
                        self.restart.store(true, Ordering::SeqCst);
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
fn test_handshake() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[test]
#[ignore = "TODO"]
fn test_reconnect() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[test]
#[ignore = "TODO"]
fn test_order_request() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

const AAPL_CONTRACT_RESPONSE: &str  = "AAPL|STK||0||SMART|USD|AAPL|NMS|NMS|265598|0.01||ACTIVETIM,AD,ADDONT,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SMARTSTG,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,NASDAQ,DRCTEDGE,BEX,BATS,EDGEA,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,IBEOS,OVERNIGHT,TPLUS0,PSX|1|0|APPLE INC|NASDAQ||Technology|Computers|Computers|US/Eastern|20250324:0400-20250324:2000;20250325:0400-20250325:2000;20250326:0400-20250326:2000;20250327:0400-20250327:2000;20250328:0400-20250328:2000|20250324:0930-20250324:1600;20250325:0930-20250325:1600;20250326:0930-20250326:1600;20250327:0930-20250327:1600;20250328:0930-20250328:1600|||1|ISIN|US0378331005|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|0.0001|0.0001|100|";

#[test]
fn test_connection_no_threads() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;

    let events = vec![
        Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::from("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::Restart,
        Event::from("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::from("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::from_request(packet.clone(), &[&format!("10|9000|{}", AAPL_CONTRACT_RESPONSE), "52|1|9001|"]),
    ];

    let stream = MockSocket::new(events);
    let connection = Connection::connect(stream, 28)?;
    let server_version = connection.server_version();
    let bus = TcpMessageBus::new(connection)?;

    bus.dispatch(server_version)?;

    let subscription = bus.send_request(9000, &packet)?;

    bus.dispatch(server_version)?;
    bus.dispatch(server_version)?;

    let result = subscription.next().unwrap();
    println!("{:#?}", result);

    // bus.dispatch(server_version)?;
    Ok(())
}

const NEWS_RESPONSE: &str = "85|08|0BRFG|Briefing.com General Market Columns|BRFUPDN|Briefing.com Analyst Actions|DJ-N|Dow Jones News Service|DJ-RTA|Dow Jones Real-Time News Asia Pacific|DJ-RTE|Dow Jones Real-Time News Europe|DJ-RTG|Dow Jones Real-Time News Global|DJ-RTPRO|Dow Jones Real-Time News Pro|DJNL|Dow Jones Newsletters|";
// // increases transport.rs code cov by ~16%%
//
#[test]
fn test_shared_request() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let events = vec![
        Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::request("|85|", &[NEWS_RESPONSE]),
    ];

    let client = Client::connect("192.168.0.5:4002", 28).expect("connection failed");

    client.news_providers().expect("request news providers failed");

    Ok(())
}
//
// #[test]
// fn test_send_request() -> Result<(), Box<dyn std::error::Error>> {
//     let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;
//
//     let events = vec![
//         Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
//         Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
//         Event::Message(MockExchange::Request {
//             request: packet.clone(),
//             responses: vec![
//                 ResponseMessage::from_simple(&format!("10|9000|{}", AAPL_CONTRACT_RESPONSE)),
//                 ResponseMessage::from_simple("52|1|9001|"),
//             ],
//         }),
//     ];
//     let server = TestServer::start(events);
//
//     let connection = Connection::connect(28, &server.address().to_string())?;
//     let server_version = connection.server_version();
//     let bus = Arc::new(TcpMessageBus::new(connection)?);
//
//     bus.process_messages(server_version)?;
//
//     let subscription = bus.send_request(9000, &packet)?;
//     let result = subscription.next();
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
