use byteorder::{BigEndian, ReadBytesExt};
use std::io::{Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use time_tz::timezones;

use crate::messages::{encode_length, OutgoingMessages};

#[derive(Debug, Clone)]
struct Interaction {
    request: OutgoingMessages,
    responses: Vec<String>,
}

pub struct MockGateway {
    handle: Option<thread::JoinHandle<()>>,
    requests: Arc<Mutex<Vec<String>>>,
    interactions: Vec<Interaction>,
    server_version: i32,
    address: Option<String>,
}

impl MockGateway {
    pub fn new(server_version: i32) -> Self {
        MockGateway {
            handle: None,
            requests: Arc::new(Mutex::new(Vec::new())),
            interactions: Vec::new(),
            server_version,
            address: None,
        }
    }

    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }

    pub fn address(&self) -> String {
        self.address.clone().unwrap_or_default()
    }

    pub fn add_interaction(&mut self, request: OutgoingMessages, responses: Vec<String>) {
        self.interactions.push(Interaction { request, responses });
    }

    pub fn server_version(&self) -> i32 {
        self.server_version
    }

    pub fn time_zone(&self) -> Option<&time_tz::Tz> {
        Some(timezones::db::EST)
    }

    pub fn start(&mut self) -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;

        let requests = Arc::clone(&self.requests);
        let interactions = self.interactions.clone();
        let server_version = self.server_version;

        let handle = thread::spawn(move || {
            // Handle single request and exit
            let stream = match listener.accept() {
                Ok((stream, addr)) => {
                    println!("MockGateway: Accepted connection from {}", addr);
                    stream
                }
                Err(e) => {
                    eprintln!("MockGateway: Failed to accept connection: {}", e);
                    return;
                }
            };

            let mut handler = ConnectionHandler::new(server_version, requests.clone(), interactions.clone());
            if let Err(err) = handler.handle(stream) {
                // Error handling connection
                eprintln!("MockGateway: Error handling connection: {}", err);
            }
        });

        self.handle = Some(handle);
        self.address = Some(address.to_string());

        Ok(())
    }
}

impl Drop for MockGateway {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().expect("Failed to join mock gateway thread");
        }
    }
}

struct ConnectionHandler {
    requests: Arc<Mutex<Vec<String>>>,
    interactions: Vec<Interaction>,
    current_interaction: usize,
    server_version: i32,
}

impl ConnectionHandler {
    pub fn new(server_version: i32, requests: Arc<Mutex<Vec<String>>>, interactions: Vec<Interaction>) -> Self {
        ConnectionHandler {
            requests,
            interactions,
            current_interaction: 0,
            server_version,
        }
    }

    pub fn handshake_response(&self) -> String {
        format!("{}\020240120 12:00:00 EST\0", self.server_version)
    }

    pub fn read_message(&mut self, stream: &mut TcpStream) -> Result<String, std::io::Error> {
        let size = self.read_size(stream)?;
        let mut buf = vec![0u8; size];
        stream.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    pub fn write_message(&mut self, stream: &mut TcpStream, message: String) -> Result<(), std::io::Error> {
        if self.server_version >= crate::server_versions::PROTOBUF {
            // Binary format: extract msg_id from first field, encode as binary msg_id + remaining text
            let packet = crate::messages::encode_request_binary_from_text(&message);
            stream.write_all(&packet)?;
        } else {
            let packet = encode_length(&message);
            stream.write_all(&packet)?;
        }
        Ok(())
    }

    pub fn handle(&mut self, mut stream: TcpStream) -> Result<(), std::io::Error> {
        self.handle_startup(&mut stream)?;

        if self.interactions.is_empty() {
            self.send_shutdown(&mut stream)?;
            return Ok(());
        }

        // Set a read timeout so we don't wait forever for requests
        stream.set_read_timeout(Some(std::time::Duration::from_millis(500)))?;

        while let Ok(raw) = self.read_raw_message(&mut stream) {
            // Extract msg_id from binary request
            let request_msg_id = if raw.len() >= 4 {
                let id = i32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]);
                if id > 200 {
                    id - 200
                } else {
                    id
                }
            } else {
                0
            };
            self.add_request(request_msg_id.to_string());

            // Check if we have a matching interaction
            if self.current_interaction < self.interactions.len() {
                let interaction = self.interactions[self.current_interaction].clone();
                if request_msg_id == interaction.request as i32 {
                    println!("MockGateway: Sending {} responses", interaction.responses.len());
                    for (i, response) in interaction.responses.iter().enumerate() {
                        let msg_type = response.split('\0').next().unwrap_or("unknown");
                        println!("MockGateway: Sending response {} (type {})", i + 1, msg_type);
                        self.write_message(&mut stream, response.clone())?;
                        stream.flush()?;
                        // Small delay between messages to ensure proper delivery
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    self.current_interaction += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Keep the connection alive for a bit to allow client to read all messages
        std::thread::sleep(std::time::Duration::from_millis(500));

        self.send_shutdown(&mut stream)?;
        println!("MockGateway: Shutdown sent, closing connection");

        // Give the client a moment to read the shutdown message
        std::thread::sleep(std::time::Duration::from_millis(50));

        Ok(())
    }

    pub fn read_raw_message(&mut self, stream: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
        let size = self.read_size(stream)?;
        let mut buf = vec![0u8; size];
        stream.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn handle_startup(&mut self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
        let magic_token = self.read_magic_token(stream)?;
        assert_eq!(magic_token, "API\0");

        // Supported server versions
        let supported_versions = self.read_message(stream)?;
        println!("Supported server versions: {}", supported_versions);

        // Send handshake response
        self.write_message(stream, self.handshake_response())?;

        // Start API
        // Protobuf binary format
        let raw = self.read_raw_message(stream)?;
        assert!(raw.len() >= 4, "start_api message too short");
        let msg_id = i32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]);
        assert_eq!(msg_id, 271, "expected protobuf StartApi (71+200)");

        // next valid order id
        self.write_message(stream, "9\01\090\0".to_string())?;

        // managed accounts
        self.write_message(stream, "15\01\02334\0".to_string())?;

        Ok(())
    }

    pub fn send_shutdown(&mut self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
        // signal shutdown
        println!("Sending shutdown message");
        self.write_message(stream, "-2\01\0".to_string())?;

        // Flush to ensure the message is sent
        stream.flush()?;

        Ok(())
    }

    pub fn add_request(&mut self, request: String) {
        let mut requests = self.requests.lock().unwrap();
        requests.push(request);
    }

    pub fn read_magic_token(&mut self, stream: &mut TcpStream) -> Result<String, std::io::Error> {
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    fn read_size(&mut self, stream: &mut TcpStream) -> Result<usize, std::io::Error> {
        let buffer = &mut [0_u8; 4];
        stream.read_exact(buffer)?;
        let mut reader = Cursor::new(buffer);
        let count = reader.read_u32::<BigEndian>()?;
        Ok(count as usize)
    }
}
