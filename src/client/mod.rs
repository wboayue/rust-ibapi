//! Client implementation with sync/async support

pub(crate) mod builders;
pub(crate) mod error_handler;
pub(crate) mod id_generator;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

// Re-export the appropriate Client based on feature
#[cfg(feature = "sync")]
pub use sync::Client;

#[cfg(feature = "async")]
pub use r#async::Client;

// Re-export subscription types from subscriptions module
#[cfg(feature = "sync")]
pub use crate::subscriptions::{SharesChannel, Subscription};

#[cfg(feature = "sync")]
pub(crate) use crate::subscriptions::{ResponseContext, StreamDecoder};

#[cfg(feature = "async")]
pub use crate::subscriptions::Subscription;

// Re-export builder traits (internal use only)
pub(crate) use builders::{ClientRequestBuilders, SubscriptionBuilderExt};

#[cfg(test)]
pub mod mocks {
    use byteorder::{BigEndian, ReadBytesExt};
    use std::io::{Cursor, Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
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
        listener: Option<TcpListener>,
        handle: Option<thread::JoinHandle<()>>,
        running: Arc<AtomicBool>,
        requests: Arc<Mutex<Vec<String>>>,
        interactions: Vec<Interaction>,
    }

    impl MockGateway {
        pub fn new() -> Self {
            MockGateway {
                listener: None,
                handle: None,
                running: Arc::new(AtomicBool::new(true)),
                requests: Arc::new(Mutex::new(Vec::new())),
                interactions: Vec::new(),
            }
        }

        pub fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }

        pub fn add_interaction(&mut self, request: OutgoingMessages, responses: Vec<String>) {
            self.interactions.push(Interaction { request, responses });
        }

        pub fn start(&mut self) -> Result<String, Box<dyn std::error::Error>> {
            // Bind to a random available port
            let listener = TcpListener::bind("127.0.0.1:0")?;
            let address = listener.local_addr()?;

            let requests = Arc::clone(&self.requests);
            let interactions = self.interactions.clone();

            // Spawn a thread to handle connections
            let listener_clone = listener.try_clone()?;
            let handle = thread::spawn(move || {
                for stream in listener_clone.incoming() {
                    if let Ok(stream) = stream {
                        let mut handler = ConnectionHandler::new(stream, requests.clone(), interactions.clone());
                        let e = handler.handle_connection();
                        if let Err(err) = e {
                            eprintln!("Error handling connection: {}", err);
                        }
                    }
                }
            });

            self.handle = Some(handle);
            self.listener = Some(listener);

            // Return the address as a string
            Ok(address.to_string())
        }

        pub fn server_version(&self) -> i32 {
            176 // Return a realistic server version
        }

        pub fn time_zone(&self) -> Option<&time_tz::Tz> {
            Some(&timezones::db::EST)
        }
    }

    impl Drop for MockGateway {
        fn drop(&mut self) {
            self.running.store(false, Ordering::SeqCst);

            println!("MockGateway is being dropped, cleaning up...");
            // Clean up the listener when dropped
            if let Some(listener) = self.listener.take() {
                drop(listener);
            }

            if let Some(handle) = self.handle.take() {
                // handle.join().expect("Failed to join mock gateway thread");
            }
        }
    }

    struct ConnectionHandler {
        stream: std::net::TcpStream,
        requests: Arc<Mutex<Vec<String>>>,
        interactions: Vec<Interaction>,
        current_interaction: usize,
    }

    impl ConnectionHandler {
        pub fn new(stream: std::net::TcpStream, requests: Arc<Mutex<Vec<String>>>, interactions: Vec<Interaction>) -> Self {
            ConnectionHandler {
                stream,
                requests,
                interactions,
                current_interaction: 0,
            }
        }

        pub fn handshake_response(&self) -> String {
            "176\020240120 12:00:00 EST\0".to_string()
        }

        pub fn read_message(&mut self) -> Result<String, std::io::Error> {
            let size = self.read_size()?;
            let mut buf = vec![0u8; size];
            self.stream.read_exact(&mut buf)?;
            Ok(String::from_utf8_lossy(&buf).into_owned())
        }

        pub fn write_message(&mut self, message: String) -> Result<(), std::io::Error> {
            let packet = encode_length(&message);
            self.stream.write_all(&packet)?;
            Ok(())
        }

        pub fn handle_connection(&mut self) -> Result<(), std::io::Error> {
            let magic_token = self.read_magic_token()?;
            println!("Received magic token: {}", magic_token);

            let message = self.read_message()?;
            println!("Received message: {}", message);

            self.write_message(self.handshake_response())?;
            println!("Sent handshake response: {:?}", self.handshake_response());

            let request = self.read_message()?;
            println!("Received request-1: {:?}", request);

            self.write_message("9\01\090\0".to_string())?;
            self.write_message("15\01\02334\0".to_string())?;

            let request = self.read_message()?;
            println!("Received request-2: {:?}", request);

            self.add_request(request);

            let interaction = self.interactions[self.current_interaction].clone();
            for response in &interaction.responses {
                self.write_message(response.clone())?;
                println!("Sent response: {}", response);
            }

            Ok(())
        }

        pub fn add_request(&mut self, request: String) {
            let mut requests = self.requests.lock().unwrap();
            requests.push(request);
        }

        pub fn read_magic_token(&mut self) -> Result<String, std::io::Error> {
            let mut buf = [0u8; 4];
            self.stream.read_exact(&mut buf)?;
            Ok(String::from_utf8_lossy(&buf).into_owned())
        }

        fn read_size(&mut self) -> Result<usize, std::io::Error> {
            let buffer = &mut [0_u8; 4];
            self.stream.read_exact(buffer)?;
            let mut reader = Cursor::new(buffer);
            let count = reader.read_u32::<BigEndian>()?;
            Ok(count as usize)
        }
    }
}
