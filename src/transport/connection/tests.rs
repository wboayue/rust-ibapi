use crate::client::Client;
use crate::contracts::Contract;
use crate::messages::{RequestHandshake, RequestMessage, ResponseMessage};
use crate::transport::read_header;
use log::debug;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::net::{SocketAddr, TcpListener};
use std::str::from_utf8;
use std::thread::JoinHandle;

// Build Events with Stubs
// struct EventBuilder {
//     events:
// }
// impl EventBuilder {
//     startup()
// }

// Trying to keep the shared functionality between the TestServer and Connection
// in a location they can be accessed by both

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
            self.keep_alive()
        }
    }

    fn keep_alive(&mut self) {
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

const AAPL_CONTRACT: &str  = "AAPL|STK||0||SMART|USD|AAPL|NMS|NMS|265598|0.01||ACTIVETIM,AD,ADDONT,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SMARTSTG,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,NASDAQ,DRCTEDGE,BEX,BATS,EDGEA,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,IBEOS,OVERNIGHT,TPLUS0,PSX|1|0|APPLE INC|NASDAQ||Technology|Computers|Computers|US/Eastern|20250324:0400-20250324:2000;20250325:0400-20250325:2000;20250326:0400-20250326:2000;20250327:0400-20250327:2000;20250328:0400-20250328:2000|20250324:0930-20250324:1600;20250325:0930-20250325:1600;20250326:0930-20250326:1600;20250327:0930-20250327:1600;20250328:0930-20250328:1600|||1|ISIN|US0378331005|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|0.0001|0.0001|100|";

// Examples:
// Testing connect -> request -> disconnect -> reconnect -> request
#[test]
fn test_mock_connection() {
    env_logger::init();

    // these messages could also be read from the output of MessageRecorder...
    // assuming the MessageRecorder records the full establish_connection() routine
    let events = vec![
        Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        Event::request(
            "9|8|9000|0|AAPL|STK||0|||SMART||USD|||0||",
            &[&format!("10|9000|{}", AAPL_CONTRACT), "52|1|9000|"],
        ),
        Event::Restart,
        Event::handshake("v100..173", &["173|20250323 22:21:01 Greenwich Mean Time|"]),
        Event::request("71|2|28|", &["15|1|DU1234567|", "9|1|1|"]),
        // Event::request(
        //     "9|8|9001|0|AAPL|STK||0|||SMART||USD|||0||",
        //     &[&format!("10|9001|{}", AAPL_CONTRACT), "52|1|9001|"],
        // ),
    ];

    let server = TestServer::start(events);

    let client = Client::connect(&server.address().to_string(), 28).unwrap();

    let contract = Contract::stock("AAPL");
    client.contract_details(&contract).unwrap();

    // TODO: this should wait until the client has reconnected instead of sleeping
    // sleep(Duration::from_secs(2));
    // match client.contract_details(&contract) {
    //     Ok(details) => {
    //         println!("{}", "details");
    //         println!("{:?}", details)
    //     }
    //     Err(e) => {
    //         println!("{}", "error");
    //         println!("{:#?}", e)
    //     }
    // }

    // let responses = client.send_request(client.next_request_id(), packet)?;
}

#[test]
fn test_connection() {
    env_logger::init();
    let client = Client::connect("192.168.0.5:4002", 28).unwrap();

    let contract = Contract::stock("AAPL");
    client.contract_details(&contract).unwrap();
}
