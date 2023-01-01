use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::io::Cursor;
use std::iter::Iterator;
use std::net::TcpStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, error, info};
use time::macros::{datetime, format_description};
use time::OffsetDateTime;

use crate::client::{RequestMessage, ResponseMessage};
use crate::messages::IncomingMessages;
use crate::server_versions;

pub trait MessageBus {
    fn read_packet(&mut self) -> Result<ResponseMessage>;
    fn read_packet_for_request(&mut self, request_id: i32) -> Result<ResponseMessage>;
    fn write_message(&mut self, packet: &RequestMessage) -> Result<()>;
    fn write_message_for_request(
        &mut self,
        request_id: i32,
        packet: &RequestMessage,
    ) -> Result<ResponsePacketPromise>;
    fn write(&mut self, packet: &str) -> Result<()>;
    fn process_messages(&mut self, server_version: i32) -> Result<()>;
}

#[derive(Debug)]
struct Outbox(Sender<ResponseMessage>);

#[derive(Debug)]
pub struct TcpMessageBus {
    reader: Arc<TcpStream>,
    writer: Box<TcpStream>,
    handles: Vec<JoinHandle<i32>>,
    requests: Arc<RwLock<HashMap<i32, Outbox>>>,
    recorder: MessageRecorder,
}

unsafe impl Send for Outbox {}
unsafe impl Sync for Outbox {}

impl TcpMessageBus {
    pub fn connect(connection_string: &str) -> Result<TcpMessageBus> {
        let stream = TcpStream::connect(connection_string)?;

        let reader = Arc::new(stream.try_clone()?);
        let writer = Box::new(stream);
        let requests = Arc::new(RwLock::new(HashMap::default()));

        Ok(TcpMessageBus {
            reader,
            writer,
            handles: Vec::default(),
            requests,
            recorder: MessageRecorder::new(),
        })
    }

    fn add_sender(&mut self, request_id: i32, sender: Sender<ResponseMessage>) -> Result<()> {
        let requests = Arc::clone(&self.requests);

        match requests.write() {
            Ok(mut hash) => {
                hash.insert(request_id, Outbox(sender));
            }
            Err(e) => {
                return Err(anyhow!("{}", e));
            }
        }

        Ok(())
    }
}

// impl read/write?

const UNSPECIFIED_REQUEST_ID: i32 = -1;

impl MessageBus for TcpMessageBus {
    fn read_packet(&mut self) -> Result<ResponseMessage> {
        read_packet(&self.reader)
    }

    fn read_packet_for_request(&mut self, request_id: i32) -> Result<ResponseMessage> {
        debug!("read message for request_id {:?}", request_id);

        let requests = Arc::clone(&self.requests);

        let collection = requests.read().unwrap();
        let _request = match collection.get(&request_id) {
            Some(request) => request,
            _ => {
                return Err(anyhow!("no request found for request_id {:?}", request_id));
            }
        };

        // debug!("found request {:?}", request);
        // // FIXME still conviluted
        // let data = request.rx.recv()?;

        // let mut mut_collection = requests.write().unwrap();
        // mut_collection.remove(&request_id);

        // Ok(request.rx.recv()?)
        Err(anyhow!("no way"))
    }

    fn write_message_for_request(
        &mut self,
        request_id: i32,
        packet: &RequestMessage,
    ) -> Result<ResponsePacketPromise> {
        let (sender, receiver) = mpsc::channel();

        self.add_sender(request_id, sender)?;
        self.write_message(packet)?;

        Ok(ResponsePacketPromise::new(receiver))
    }

    fn write_message(&mut self, message: &RequestMessage) -> Result<()> {
        let encoded = message.encode();
        debug!("{:?} ->", encoded);

        let data = encoded.as_bytes();
        let mut header = vec![];
        header.write_u32::<BigEndian>(data.len() as u32)?;

        self.writer.write_all(&header)?;
        self.writer.write_all(data)?;

        self.recorder.record_request(message);

        Ok(())
    }

    fn write(&mut self, data: &str) -> Result<()> {
        debug!("{:?} ->", data);
        self.writer.write_all(data.as_bytes())?;
        Ok(())
    }

    fn process_messages(&mut self, server_version: i32) -> Result<()> {
        let reader = Arc::clone(&self.reader);
        let requests = Arc::clone(&self.requests);
        let recorder = self.recorder.clone();

        let handle = thread::spawn(move || loop {
            match read_packet(&reader) {
                Ok(message) => {
                    recorder.record_response(&message);
                    dispatch_message(message, server_version, &requests);
                },
                Err(err) => {
                    error!("error reading packet: {:?}", err);
                    // thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };

            // FIXME - does read block?
            // thread::sleep(Duration::from_secs(1));
        });

        self.handles.push(handle);

        Ok(())
    }
}

fn dispatch_message(mut message: ResponseMessage, server_version: i32, requests: &Arc<RwLock<HashMap<i32, Outbox>>>) {
    match message.message_type() {
        IncomingMessages::Error => {
            let request_id = message.peek_int(2).unwrap_or(-1);

            if request_id == UNSPECIFIED_REQUEST_ID {
                error_event(server_version, message).unwrap();
            } else {
                process_response(&requests, message);
            }
        }
        IncomingMessages::NextValidId => {
            process_next_valid_id(server_version, message)
        }
        IncomingMessages::ManagedAccounts => {
            process_managed_accounts(server_version, message)
        }
        _ => process_response(&requests, message),
    };
}

fn read_packet(mut reader: &TcpStream) -> Result<ResponseMessage> {
    let message_size = read_header(reader)?;
    let mut data = vec![0_u8; message_size];

    reader.read_exact(&mut data)?;

    let packet = ResponseMessage::from(&String::from_utf8(data)?);
    debug!("raw string: {:?}", packet);

    Ok(packet)
}

fn read_header(mut reader: &TcpStream) -> Result<usize> {
    let buffer = &mut [0_u8; 4];
    reader.read_exact(buffer)?;

    let mut reader = Cursor::new(buffer);
    let count = reader.read_u32::<BigEndian>()?;

    Ok(count as usize)
}

fn error_event(server_version: i32, mut packet: ResponseMessage) -> Result<()> {
    packet.skip(); // message_id

    let version = packet.next_int()?;

    if version < 2 {
        let message = packet.next_string()?;
        error!("version 2 erorr: {}", message);
        Ok(())
    } else {
        let request_id = packet.next_int()?;
        let error_code = packet.next_int()?;
        let error_message = packet.next_string()?;
        // let error_message = if server_version >= server_versions::ENCODE_MSG_ASCII7 {
        //     // Regex.Unescape(ReadString()) : ReadString();
        //     packet.next_string()?
        // } else {
        //     packet.next_string()?
        // };

        let mut advanced_order_reject_json: String = "".to_string();
        if server_version >= server_versions::ADVANCED_ORDER_REJECT {
            advanced_order_reject_json = packet.next_string()?;
            // if (!Util.StringIsEmpty(tempStr))
            // {
            //     advancedOrderRejectJson = Regex.Unescape(tempStr);
            // }
        }
        error!(
            "request_id: {}, error_code: {}, error_message: {}, advanced_order_reject_json: {}",
            request_id, error_code, error_message, advanced_order_reject_json
        );
        Ok(())
    }
}

fn process_next_valid_id(_server_version: i32, mut packet: ResponseMessage) {
    packet.skip(); // message_id
    packet.skip(); // version

    let order_id = packet.next_string().unwrap_or_else(|_| String::default());
    info!("next_valid_order_id: {}", order_id)
}

fn process_managed_accounts(_server_version: i32, mut packet: ResponseMessage) {
    packet.skip(); // message_id
    packet.skip(); // version

    let managed_accounts = packet.next_string().unwrap_or_else(|_| String::default());
    info!("managed accounts: {}", managed_accounts)
}

fn process_response(requests: &Arc<RwLock<HashMap<i32, Outbox>>>, packet: ResponseMessage) {
    let collection = requests.read().unwrap();

    let request_id = packet.request_id().unwrap_or(-1);
    let outbox = match collection.get(&request_id) {
        Some(outbox) => outbox,
        _ => {
            debug!(
                "no request found for request_id {:?} - {:?}",
                request_id, packet
            );
            return;
        }
    };

    outbox.0.send(packet).unwrap();
}

#[derive(Debug)]
pub struct ResponsePacketPromise {
    receiver: Receiver<ResponseMessage>,
}

impl ResponsePacketPromise {
    pub fn new(receiver: Receiver<ResponseMessage>) -> ResponsePacketPromise {
        ResponsePacketPromise { receiver }
    }

    pub fn message(&self) -> Result<ResponseMessage> {
        // Duration::from_millis(100)

        Ok(self.receiver.recv_timeout(Duration::from_millis(20000))?)
        // return Err(anyhow!("no message"));
    }
}

// impl IntoIterator for ResponsePacketPromise {
//     type Item = ResponsePacket;
//     type IntoIter = ResponsePacketIterator;
//     fn into_iter(self) -> Self::IntoIter {
//         todo!()
//     }
// }

pub struct ResponsePacketIterator {}

impl Iterator for ResponsePacketPromise {
    type Item = ResponseMessage;
    fn next(&mut self) -> Option<Self::Item> {
        match self.receiver.recv_timeout(Duration::from_millis(10000)) {
            Err(e) => {
                error!("error receiving packet: {:?}", e);
                None
            }
            Ok(message) => Some(message),
        }
    }
}

static RECORDING_SEQ: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug)]
struct MessageRecorder {
    enabled: bool,
    recording_dir: String,
}

impl MessageRecorder {
    fn new() -> Self {
        match env::var("IBAPI_RECORDING_DIR") {
            Ok(dir) => {
                if dir.is_empty() {
                    MessageRecorder {
                        enabled: false,
                        recording_dir: String::from(""),
                    }
                } else {
                    let format = format_description!("[year]-[month]-[day]-[hour]-[minute]");
                    let now = OffsetDateTime::now_utc();
                    let recording_dir = format!("{}/{}", dir, now.format(&format).unwrap());

                    fs::create_dir_all(&recording_dir).unwrap();

                    MessageRecorder {
                        enabled: true,
                        recording_dir: recording_dir,
                    }
                }
            }
            _ => MessageRecorder {
                enabled: false,
                recording_dir: String::from(""),
            },
        }
    }

    fn request_file(&self, record_id: usize) -> String {
        format!("{}/{:04}-request.msg", self.recording_dir, record_id)
    }

    fn response_file(&self, record_id: usize) -> String {
        format!("{}/{:04}-response.msg", self.recording_dir, record_id)
    }

    fn record_request(&self, message: &RequestMessage) {
        if !self.enabled {
            return;
        }

        let record_id = RECORDING_SEQ.fetch_add(1, Ordering::SeqCst);
        fs::write(
            self.request_file(record_id),
            message.encode().replace("\0", "|"),
        )
        .unwrap();
    }

    fn record_response(&self, message: &ResponseMessage) {
        if !self.enabled {
            return;
        }

        let record_id = RECORDING_SEQ.fetch_add(1, Ordering::SeqCst);
        fs::write(
            self.response_file(record_id),
            message.encode().replace("\0", "|"),
        )
        .unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;

    #[test]
    fn env_var_enables_recorder() {
        let key = String::from("IBAPI_RECORDING_DIR");
        let dir = String::from("/tmp/records");

        env::set_var(&key, &dir);

        let recorder = MessageRecorder::new();

        assert_eq!(true, recorder.enabled);
        assert!(
            &recorder.recording_dir.starts_with(&dir),
            "{} != {}",
            &recorder.recording_dir,
            &dir
        )
    }

    #[test]
    fn recorder_is_disabled() {
        let key = String::from("IBAPI_RECORDING_DIR");

        env::set_var(&key, &"");

        let recorder = MessageRecorder::new();

        assert_eq!(false, recorder.enabled);
        assert_eq!("", &recorder.recording_dir);
    }

    #[test]
    fn recorder_generates_output_file() {
        let recording_dir = String::from("/tmp/records");

        let recorder = MessageRecorder {
            enabled: true,
            recording_dir: recording_dir,
        };

        assert_eq!(
            format!("{}/0001-request.msg", recorder.recording_dir),
            recorder.request_file(1)
        );
        assert_eq!(
            format!("{}/0002-response.msg", recorder.recording_dir),
            recorder.response_file(2)
        );
    }
}
