use crate::client::Client;
use crate::contracts::Contract;
use crate::transport::{encode_packet, read_header};
use log::debug;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::net::{SocketAddr, TcpListener};
use std::str::from_utf8;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;

#[derive(Clone, Debug)]
enum Event {
    Restart,
    Message(Message),
}

#[derive(Clone, Debug, PartialEq)]
struct Message {
    request: String,
    responses: Vec<String>,
}

impl Event {
    fn message(request: &str, response: Vec<&str>) -> Self {
        let response_vec = response.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        let message = Message {
            request: request.to_owned(),
            responses: response_vec,
        };
        Event::Message(message)
    }
}

#[derive(PartialEq)]
enum MockStreamState {
    Disconnected,
    Connected,
    // Maintenance,
}

struct MockStream {
    stream: TcpStream,
    messages: VecDeque<Message>,
    state: MockStreamState,
    // keep the stream alive after messages have processed
    keep_alive: bool,
}

impl MockStream {
    pub fn new(stream: TcpStream, messages: VecDeque<Message>, keep_alive: bool) -> Self {
        Self {
            stream,
            messages,
            state: MockStreamState::Disconnected,
            keep_alive,
        }
    }

    fn write_responses(&mut self, responses: Vec<String>) {
        for res in responses {
            debug!("mock -> {:#?}", res);
            self.stream.write_all(&encode_packet(&res).clone().as_bytes()).unwrap();
        }
    }

    fn process_handshake(&mut self) {
        let mut data = vec![0_u8; 4];
        self.stream.read(&mut data).unwrap();
        assert_eq!(
            data,
            b"API\0",
            "Handshake message did not start with API.\n  left: {}\n  right: {}",
            from_utf8(&data).unwrap(),
            from_utf8(&b"API\0"[..]).unwrap(),
        );
        self.state = MockStreamState::Connected;
    }

    fn process_message(&mut self, message: Message, message_size: usize) {
        let mut data = vec![0_u8; message_size];
        self.stream.read_exact(&mut data).unwrap();

        debug!("mock <- {:#?}", from_utf8(&data).unwrap());

        let request = message.request.as_bytes();

        assert_eq!(
            data,
            request,
            "Mock Server event request != Client request.\n  left: {},\n  right: {}",
            from_utf8(&data).unwrap(),
            message.request
        );

        self.write_responses(message.responses);
    }

    pub fn process(&mut self) {
        loop {
            let message = match self.messages.pop_front() {
                Some(message) => message,
                None => break,
            };

            if self.state == MockStreamState::Disconnected {
                self.process_handshake();
            }

            let message_size = read_header(&self.stream).unwrap();

            self.process_message(message, message_size);
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

const AAPL_CONTRACT: &str  = "AAPL\0STK\0\00\0\0SMART\0USD\0AAPL\0NMS\0NMS\0265598\00.01\0\0ACTIVETIM,AD,ADDONT,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SMARTSTG,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF\0SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,NASDAQ,DRCTEDGE,BEX,BATS,EDGEA,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,IBEOS,OVERNIGHT,TPLUS0,PSX\01\00\0APPLE INC\0NASDAQ\0\0Technology\0Computers\0Computers\0US/Eastern\020250324:0400-20250324:2000;20250325:0400-20250325:2000;20250326:0400-20250326:2000;20250327:0400-20250327:2000;20250328:0400-20250328:2000\020250324:0930-20250324:1600;20250325:0930-20250325:1600;20250326:0930-20250326:1600;20250327:0930-20250327:1600;20250328:0930-20250328:1600\0\0\01\0ISIN\0US0378331005\01\0\0\026,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26\0\0COMMON\00.0001\00.0001\0100\0";

// Examples:
// Testing connect -> request -> disconnect -> reconnect -> request
#[test]
fn test_mock_connection() {
    env_logger::init();

    // these messages could also be read from the output of MessageRecorder...
    // assuming the MessageRecorder records the full establish_connection() routine
    let events = vec![
        Event::message("v100..173", vec!["173\020250323 22:21:01 Greenwich Mean Time\0"]),
        Event::message("71\02\028\0\0", vec!["15\01\0DU1234567\0", "9\01\01\0"]),
        Event::message(
            "9\08\09000\00\0AAPL\0STK\0\00\0\0\0SMART\0\0USD\0\0\00\0\0\0",
            vec![&format!("10\09000\0{}", AAPL_CONTRACT), "52\01\09000\0"],
        ),
        Event::Restart,
        Event::message("v100..173", vec!["173\020250323 22:21:01 Greenwich Mean Time\0"]),
        Event::message("71\02\028\0\0", vec!["15\01\0DU1234567\0", "9\01\01\0"]),
        Event::message(
            "9\08\09001\00\0AAPL\0STK\0\00\0\0\0SMART\0\0USD\0\0\00\0\0\0",
            vec![&format!("10\09001\0{}", AAPL_CONTRACT), "52\01\09001\0"],
        ),
    ];

    let server = TestServer::start(events);

    let client = Client::connect(&server.address().to_string(), 28).unwrap();

    let contract = Contract::stock("AAPL");
    client.contract_details(&contract).unwrap();

    drop(server);

    // TODO: this should wait until the client has reconnected instead of sleeping
    sleep(Duration::from_secs(2));

    client.contract_details(&contract).unwrap();
}
