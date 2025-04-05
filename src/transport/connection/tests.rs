use crate::client::Client;
use crate::contracts::encoders::encode_request_contract_data;
use crate::contracts::Contract;
use crate::errors::Error;
use crate::messages::{RequestHandshake, RequestMessage, ResponseMessage};
use crate::transport::{read_header, Connection, MessageBus, TcpMessageBus};
use log::debug;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::net::{SocketAddr, TcpListener};
use std::str::from_utf8;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

// Build Events with Stubs
// struct EventBuilder {
//     events: Vec<Event>
// }
// impl EventBuilder {
//     fn handshake(&self) {
//         self.events.push();
//
//     }
// }

// #[derive(Clone, Debug)]
// struct Message {
//     request: RequestMessage,
//     responses: Vec<ResponseMessage>,
// }
#[derive(Clone, Debug)]
enum Message {
    Handshake {
        request: RequestHandshake,
        responses: Vec<ResponseMessage>,
    },
    Request {
        request: RequestMessage,
        responses: Vec<ResponseMessage>,
    },
    // Subscription
}

impl Message {
    pub fn request(request: RequestMessage, responses: Vec<ResponseMessage>) -> Self {
        Message::Request {
            request,
            responses: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
enum Event {
    Restart,
    Message(Message),
}

impl Event {
    fn request(request: &str, responses: &[&str]) -> Self {
        let responses = responses
            .into_iter()
            .map(|s| ResponseMessage::from_simple(s))
            .collect::<Vec<ResponseMessage>>();
        let message = Message::Request {
            request: RequestMessage::from_simple(request),
            responses,
        };
        Event::Message(message)
    }

    fn handshake(request: &str, responses: &[&str]) -> Self {
        let responses = responses
            .into_iter()
            .map(|s| ResponseMessage::from_simple(s))
            .collect::<Vec<ResponseMessage>>();
        let message = Message::Handshake {
            request: RequestHandshake::from_simple(request),
            responses,
        };
        Event::Message(message)
    }
}

struct MockStream {
    stream: TcpStream,
    messages: VecDeque<Message>,
    // keep the stream alive after messages have processed
    keep_alive: bool,
}

impl MockStream {
    pub fn new(stream: TcpStream, messages: VecDeque<Message>, keep_alive: bool) -> Self {
        Self {
            stream,
            messages,
            keep_alive,
        }
    }

    fn read_prefix(&mut self) {
        let mut data = vec![0_u8; 4];
        self.stream.read(&mut data).unwrap();
        assert_eq!(
            data,
            b"API\0",
            "Handshake message did not start with API prefix.\n  left: {}\n  right: {}",
            from_utf8(&data).unwrap(),
            from_utf8(&b"API\0"[..]).unwrap(),
        );
    }

    fn handle_message(&mut self, request: String, responses: Vec<ResponseMessage>) {
        let message_size = read_header(&self.stream).unwrap();

        let mut data = vec![0_u8; message_size];

        self.stream.read_exact(&mut data).unwrap();

        let raw_string = String::from_utf8(data).unwrap();
        debug!("mock <- {:#?}", raw_string);

        // let request = message.request.encode();

        assert_eq!(
            raw_string,
            request,
            // "Mock Server event request != Client request.\n  left: {},\n  right: {}",
        );

        for res in responses {
            let packet = res.packet();
            debug!("mock -> {:#?}", packet);
            self.stream.write_all(packet.as_bytes()).unwrap();
        }
    }

    pub fn process(&mut self) {
        loop {
            let message = match self.messages.pop_front() {
                Some(message) => message,
                None => break,
            };

            match message {
                Message::Handshake { request, responses } => {
                    self.read_prefix();
                    self.handle_message(request.encode(), responses);
                }
                Message::Request { request, responses } => self.handle_message(request.encode(), responses),
            }
        }

        if self.keep_alive {
            debug!("mock - keeping alive...");
            self.keep_alive()
        }
        debug!("mock - stream instance finishing...")
    }

    fn keep_alive(&mut self) {
        // TODO: get this working on drop()
        // assert!(
        //     self.messages.is_empty(),
        //     "MockStream finished with {} remaining messages..",
        //     self.messages.len()
        // );

        let mut data = vec![0_u8; 1024];
        loop {
            match self.stream.read(&mut data) {
                Ok(0) => {
                    debug!("Client closed connection to Mock Server");
                    break;
                }
                Ok(_) => {
                    panic!("Mock Server received more client requests than declared");
                }
                Err(e) => {
                    panic!("Mock Server stream errored during keep_alive: {}", e);
                }
            };
        }
    }
}

struct MockServer {
    listener: TcpListener,
}

impl MockServer {
    fn new(listener: TcpListener) -> Self {
        Self { listener }
    }

    fn process(&self, mut events: VecDeque<Event>) {
        let events_len = events.len();
        loop {
            if events.is_empty() {
                break;
            }

            let mut messages = VecDeque::new();
            while let Some(event) = events.pop_front() {
                match event {
                    Event::Message(message) => messages.push_back(message),
                    Event::Restart => break,
                }
            }

            debug!("Creating Mock Exchange with {}/{} queued requests", messages.len(), events_len);
            MockStream::new(self.listener.accept().unwrap().0, messages, events.is_empty()).process();
        }
    }
}

struct TestServer {
    listener: TcpListener,
    handle: JoinHandle<()>,
}
impl TestServer {
    fn start(events: Vec<Event>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();

        let listener_clone = listener.try_clone().unwrap();

        let handle = std::thread::spawn(move || {
            let server = MockServer::new(listener_clone);
            server.process(VecDeque::from(events));
        });
        Self { listener, handle }
    }
    fn address(&self) -> SocketAddr {
        self.listener.local_addr().unwrap()
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

const NEWS_RESPONSE: &str = "85|08|0BRFG|Briefing.com General Market Columns|BRFUPDN|Briefing.com Analyst Actions|DJ-N|Dow Jones News Service|DJ-RTA|Dow Jones Real-Time News Asia Pacific|DJ-RTE|Dow Jones Real-Time News Europe|DJ-RTG|Dow Jones Real-Time News Global|DJ-RTPRO|Dow Jones Real-Time News Pro|DJNL|Dow Jones Newsletters|";
// increases transport.rs code cov by ~16%%

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

const AAPL_CONTRACT_RESPONSE: &str  = "AAPL|STK||0||SMART|USD|AAPL|NMS|NMS|265598|0.01||ACTIVETIM,AD,ADDONT,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SMARTSTG,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,NASDAQ,DRCTEDGE,BEX,BATS,EDGEA,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,IBEOS,OVERNIGHT,TPLUS0,PSX|1|0|APPLE INC|NASDAQ||Technology|Computers|Computers|US/Eastern|20250324:0400-20250324:2000;20250325:0400-20250325:2000;20250326:0400-20250326:2000;20250327:0400-20250327:2000;20250328:0400-20250328:2000|20250324:0930-20250324:1600;20250325:0930-20250325:1600;20250326:0930-20250326:1600;20250327:0930-20250327:1600;20250328:0930-20250328:1600|||1|ISIN|US0378331005|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|0.0001|0.0001|100|";
#[test]
fn test_send_request() -> Result<(), Box<dyn std::error::Error>> {
    let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;

    let events = vec![
        Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::Message(Message::Request {
            request: packet.clone(),
            responses: vec![
                ResponseMessage::from_simple(&format!("10|9000|{}", AAPL_CONTRACT_RESPONSE)),
                ResponseMessage::from_simple("52|1|9001|"),
            ],
        }),
    ];
    let server = TestServer::start(events);

    let connection = Connection::connect(28, &server.address().to_string())?;
    let server_version = connection.server_version();
    let bus = Arc::new(TcpMessageBus::new(connection)?);

    bus.process_messages(server_version)?;

    let subscription = bus.send_request(9000, &packet)?;
    let result = subscription.next();

    Ok(())
}

// Test Error::ConnectionReset is raised on subscription.next()
// when sending request during disconnect
#[test]
fn test_request_before_disconnect_raises_error() -> Result<(), Box<dyn std::error::Error>> {
    let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;

    let events = vec![
        Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::Message(Message::Request {
            request: packet.clone(),
            responses: vec![],
        }),
        Event::Restart,
        Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
    ];

    let server = TestServer::start(events);

    let connection = Connection::connect(28, &server.address().to_string())?;
    let server_version = connection.server_version();
    let bus = Arc::new(TcpMessageBus::new(connection)?);

    bus.process_messages(server_version)?;

    let subscription = bus.send_request(9000, &packet)?;

    match subscription.next() {
        Some(Err(Error::ConnectionReset)) => {}
        _ => panic!(),
    }

    Ok(())
}

// Test Error::ConnectionReset is raised on subscription.next()
// when sending request during disconnect
#[test]
fn test_request_during_disconnect_raises_error() -> Result<(), Box<dyn std::error::Error>> {
    let events = vec![
        Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::Restart,
        Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
    ];

    let server = TestServer::start(events);

    let connection = Connection::connect(28, &server.address().to_string())?;
    let server_version = connection.server_version();
    let bus = Arc::new(TcpMessageBus::new(connection)?);

    bus.process_messages(server_version)?;

    // sleep so the request is sent after the dispatcher thread enters the reconnection
    // routine
    std::thread::sleep(Duration::from_millis(1));

    // now attempt to send the request
    let packet = encode_request_contract_data(173, 9000, &Contract::stock("AAPL"))?;
    let subscription = bus.send_request(9000, &packet)?;

    match subscription.next() {
        Some(Err(Error::ConnectionReset)) => {}
        _ => panic!(),
    }

    Ok(())
}

// TODO: This test repeats test_request_during_disconnect() with the client instead
// the response should be the same, Error::ConnectionReset
#[test]
#[ignore = "propagate error from contract_details() to fix"]
fn test_client_request_during_disconnect() -> Result<(), Box<dyn std::error::Error>> {
    let events = vec![
        Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::Restart,
        Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
    ];

    let server = TestServer::start(events);

    let client = Client::connect(&server.address().to_string(), 28).unwrap();

    // sleep so the request is sent after the dispatcher thread enters the reconnection
    // routine
    std::thread::sleep(Duration::from_millis(1));

    // now attempt to send the request
    let contract = &Contract::stock("AAPL");

    match client.contract_details(&contract) {
        Err(Error::ConnectionReset) => {}
        _ => panic!(),
    }

    Ok(())
}
