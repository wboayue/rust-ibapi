use crate::client::Client;
use crate::contracts::Contract;
use crate::transport::{encode_packet, read_header};
use log::debug;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener};
use std::str::from_utf8;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

#[derive(Clone)]
enum Message {
    Handshake { request: String, response: Vec<String> },
    Packet { request: String, response: Vec<String> },
    Eof,
}

impl Message {
    fn handshake(request: &str, response: Vec<&str>) -> Self {
        let response_vec = response.iter().map(|s| s.to_string()).collect::<Vec<String>>();

        Message::Handshake {
            request: request.to_owned(),
            response: response_vec,
        }
    }

    fn packet(request: &str, response: Vec<&str>) -> Self {
        let response_vec = response.iter().map(|s| s.to_string()).collect::<Vec<String>>();

        Message::Packet {
            request: request.to_owned(),
            response: response_vec,
        }
    }
}

fn start_mock_server(mut messages: VecDeque<Message>) -> (SocketAddr, thread::JoinHandle<std::io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let local_addr = listener.local_addr().unwrap();

    let mut message_index = 0;
    let total_messages = messages.len();

    let join_handle = thread::spawn(move || {
        let (mut stream, _socketaddr) = listener.accept()?;
        loop {
            if messages.is_empty() {
                continue;
            }

            let message = messages.pop_front().unwrap();

            message_index += 1;

            if let Message::Eof = message {
                debug!("{}/{} mock -> eof", message_index, total_messages);
                stream.shutdown(Shutdown::Both)?;
                stream = listener.accept().unwrap().0;
                continue;
            }

            let message_size = match &message {
                Message::Handshake { request, .. } => request.len(),
                Message::Packet { .. } => read_header(&stream).unwrap(),
                _ => unreachable!(),
            };

            let mut data = vec![0_u8; message_size];
            stream.read_exact(&mut data)?;

            debug!("{}/{} mock <- {:#?}", message_index, total_messages, from_utf8(&data).unwrap());

            match &message {
                Message::Handshake { request, response } | Message::Packet { request, response } => {
                    assert_eq!(request, from_utf8(&data).unwrap());

                    for res in response {
                        debug!("{}/{} mock -> {:#?}", message_index, total_messages, res);
                        stream.write_all(&encode_packet(&res).clone().as_bytes())?;
                    }
                }
                _ => unreachable!(),
            }
        }
    });

    (local_addr, join_handle)
}

const AAPL_CONTRACT: &str  = "AAPL\0STK\0\00\0\0SMART\0USD\0AAPL\0NMS\0NMS\0265598\00.01\0\0ACTIVETIM,AD,ADDONT,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SMARTSTG,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF\0SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,NASDAQ,DRCTEDGE,BEX,BATS,EDGEA,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,IBEOS,OVERNIGHT,TPLUS0,PSX\01\00\0APPLE INC\0NASDAQ\0\0Technology\0Computers\0Computers\0US/Eastern\020250324:0400-20250324:2000;20250325:0400-20250325:2000;20250326:0400-20250326:2000;20250327:0400-20250327:2000;20250328:0400-20250328:2000\020250324:0930-20250324:1600;20250325:0930-20250325:1600;20250326:0930-20250326:1600;20250327:0930-20250327:1600;20250328:0930-20250328:1600\0\0\01\0ISIN\0US0378331005\01\0\0\026,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26\0\0COMMON\00.0001\00.0001\0100\0";

// Examples:
// Testing connect -> request -> disconnect -> reconnect -> request
#[test]
fn test_mock_connection() {
    env_logger::init();

    // these messages could also be read from the output of MessageRecorder...
    // assuming the MessageRecorder records the full establish_connection() routine
    let messages = VecDeque::from(vec![
        Message::handshake("API\0\0\0\0\tv100..173", vec!["173\020250323 22:21:01 Greenwich Mean Time\0"]),
        Message::packet("71\02\028\0\0", vec!["15\01\0DU1234567\0", "9\01\01\0"]),
        Message::packet(
            "9\08\09000\00\0AAPL\0STK\0\00\0\0\0SMART\0\0USD\0\0\00\0\0\0",
            vec![&format!("10\09000\0{}", AAPL_CONTRACT), "52\01\09000\0"],
        ),
        Message::Eof,
        Message::handshake("API\0\0\0\0\tv100..173", vec!["173\020250323 22:21:01 Greenwich Mean Time\0"]),
        Message::packet("71\02\028\0\0", vec!["15\01\0DU1234567\0", "9\01\01\0"]),
        Message::packet(
            "9\08\09001\00\0AAPL\0STK\0\00\0\0\0SMART\0\0USD\0\0\00\0\0\0",
            vec![&format!("10\09001\0{}", AAPL_CONTRACT), "52\01\09001\0"],
        ),
    ]);
    let (addr, _) = start_mock_server(messages);

    let client_address = &format!("127.0.0.1:{}", addr.port());

    let client = Client::connect(client_address, 28).unwrap();

    let contract = Contract::stock("AAPL");
    client.contract_details(&contract).unwrap();

    // TODO: this should wait until the client has reconnected instead of sleeping
    sleep(Duration::from_secs(2));

    client.contract_details(&contract).unwrap();
}

// Testing connect -> disconnect during request -> reconnect -> request
#[test]
fn test_disconnect_during_request() {
    env_logger::init();

    // these messages could also be read from the output of MessageRecorder...
    // assuming the MessageRecorder records the full establish_connection() routine
    let messages = VecDeque::from(vec![
        Message::handshake("API\0\0\0\0\tv100..173", vec!["173\020250323 22:21:01 Greenwich Mean Time\0"]),
        Message::packet("71\02\028\0\0", vec!["15\01\0DU1234567\0", "9\01\01\0"]),
        // do not send a response back to keep the client waiting...
        Message::packet("9\08\09000\00\0AAPL\0STK\0\00\0\0\0SMART\0\0USD\0\0\00\0\0\0", vec![]),
        Message::Eof,
        Message::handshake("API\0\0\0\0\tv100..173", vec!["173\020250323 22:21:01 Greenwich Mean Time\0"]),
        Message::packet("71\02\028\0\0", vec!["15\01\0DU1234567\0", "9\01\01\0"]),
        Message::packet(
            "9\08\09001\00\0AAPL\0STK\0\00\0\0\0SMART\0\0USD\0\0\00\0\0\0",
            vec![&format!("10\09001\0{}", AAPL_CONTRACT), "52\01\09001\0"],
        ),
    ]);
    let (addr, _) = start_mock_server(messages);

    let client_address = &format!("127.0.0.1:{}", addr.port());

    let client = Client::connect(client_address, 28).unwrap();

    let contract = Contract::stock("AAPL");
    client.contract_details(&contract).unwrap();

    // TODO: this should wait until the client has reconnected instead of sleeping
    sleep(Duration::from_secs(2));

    client.contract_details(&contract).unwrap();
}
