//! Asynchronous connection implementation

use std::sync::atomic::{AtomicI32, Ordering};

use log::{debug, info};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::sleep;

use super::common::{parse_connection_time, AccountInfo, ConnectionHandler, ConnectionOptions, ConnectionProtocol, StartupMessageCallback};
use super::ConnectionMetadata;
use crate::errors::Error;
use crate::messages::{RequestMessage, ResponseMessage};
use crate::trace;
use crate::transport::common::{FibonacciBackoff, MAX_RECONNECT_ATTEMPTS};
use crate::transport::recorder::MessageRecorder;

type Response = Result<ResponseMessage, Error>;

/// Asynchronous connection to TWS
#[derive(Debug)]
pub struct AsyncConnection {
    pub(crate) client_id: i32,
    pub(crate) reader: Mutex<OwnedReadHalf>,
    pub(crate) writer: Mutex<OwnedWriteHalf>,
    pub(crate) connection_metadata: Mutex<ConnectionMetadata>,
    pub(crate) server_version_cache: AtomicI32,
    pub(crate) recorder: MessageRecorder,
    pub(crate) connection_handler: ConnectionHandler,
    pub(crate) connection_url: String,
    pub(crate) options: ConnectionOptions,
}

impl AsyncConnection {
    /// Create a new async connection
    #[allow(dead_code)]
    pub async fn connect(address: &str, client_id: i32) -> Result<Self, Error> {
        Self::connect_with_callback(address, client_id, None).await
    }

    /// Create a new async connection with a callback for unsolicited messages
    ///
    /// The callback will be invoked for any messages received during connection
    /// setup that are not part of the normal handshake (e.g., OpenOrder, OrderStatus).
    pub async fn connect_with_callback(address: &str, client_id: i32, startup_callback: Option<StartupMessageCallback>) -> Result<Self, Error> {
        Self::connect_with_options(address, client_id, startup_callback.into()).await
    }

    /// Create a new async connection with custom options.
    ///
    /// Applies settings from [`ConnectionOptions`] (e.g. `TCP_NODELAY`, startup callback)
    /// before performing the TWS handshake.
    pub async fn connect_with_options(address: &str, client_id: i32, options: ConnectionOptions) -> Result<Self, Error> {
        let socket = Self::connect_socket(address, &options).await?;
        let (read_half, write_half) = socket.into_split();

        let connection = Self {
            client_id,
            reader: Mutex::new(read_half),
            writer: Mutex::new(write_half),
            connection_metadata: Mutex::new(ConnectionMetadata {
                client_id,
                ..Default::default()
            }),
            server_version_cache: AtomicI32::new(0),
            recorder: MessageRecorder::from_env(),
            connection_handler: ConnectionHandler::default(),
            connection_url: address.to_string(),
            options,
        };

        let cb_ref = connection.options.startup_callback.as_deref();
        connection.establish_connection(cb_ref).await?;

        Ok(connection)
    }

    async fn connect_socket(address: &str, options: &ConnectionOptions) -> Result<TcpStream, Error> {
        let socket = TcpStream::connect(address).await?;
        socket.set_nodelay(options.tcp_no_delay)?;
        Ok(socket)
    }

    /// Get a copy of the connection metadata
    pub async fn connection_metadata(&self) -> ConnectionMetadata {
        let mut metadata = self.connection_metadata.lock().await.clone();
        metadata.server_version = self.server_version_cache.load(Ordering::Acquire);
        metadata
    }

    /// Get the server version (lock-free; cached after handshake)
    pub(crate) fn server_version(&self) -> i32 {
        self.server_version_cache.load(Ordering::Acquire)
    }

    /// Reconnect to TWS with fibonacci backoff
    pub async fn reconnect(&self) -> Result<(), Error> {
        let mut backoff = FibonacciBackoff::new(30);

        for i in 0..MAX_RECONNECT_ATTEMPTS {
            let next_delay = backoff.next_delay();
            info!("next reconnection attempt in {next_delay:#?}");

            sleep(next_delay).await;

            match Self::connect_socket(&self.connection_url, &self.options).await {
                Ok(new_socket) => {
                    info!("reconnected !!!");

                    let (new_reader, new_writer) = new_socket.into_split();
                    {
                        let mut reader = self.reader.lock().await;
                        *reader = new_reader;
                    }
                    {
                        let mut writer = self.writer.lock().await;
                        *writer = new_writer;
                    }

                    self.reset_connection_metadata().await;

                    // Reconnection doesn't use startup callback
                    self.establish_connection(None).await?;

                    return Ok(());
                }
                Err(e) => {
                    info!("reconnection attempt {}/{} failed: {e}", i + 1, MAX_RECONNECT_ATTEMPTS);
                }
            }
        }

        Err(Error::ConnectionFailed)
    }

    async fn reset_connection_metadata(&self) {
        self.server_version_cache.store(0, Ordering::Release);

        let mut connection_metadata = self.connection_metadata.lock().await;
        *connection_metadata = ConnectionMetadata {
            client_id: self.client_id,
            ..Default::default()
        };
    }

    /// Establish connection to TWS
    pub(crate) async fn establish_connection(&self, startup_callback: Option<&(dyn Fn(ResponseMessage) + Send + Sync)>) -> Result<(), Error> {
        self.handshake().await?;
        self.start_api().await?;
        self.receive_account_info(startup_callback).await?;
        Ok(())
    }

    /// Write a message to the connection
    pub(crate) async fn write_message(&self, message: &RequestMessage) -> Result<(), Error> {
        self.recorder.record_request(message);
        let encoded = message.encode();
        debug!("-> {encoded:?}");

        // Record the request if debug logging is enabled
        if log::log_enabled!(log::Level::Debug) {
            trace::record_request(encoded.clone()).await;
        }

        let length_encoded = crate::messages::encode_length(&encoded);

        let mut writer = self.writer.lock().await;
        writer.write_all(&length_encoded).await?;
        writer.flush().await?;
        Ok(())
    }

    /// Read a message from the connection
    pub(crate) async fn read_message(&self) -> Response {
        let mut reader = self.reader.lock().await;

        // Read message length
        let mut length_bytes = [0u8; 4];
        match reader.read_exact(&mut length_bytes).await {
            Ok(_) => {}
            Err(e) => {
                debug!("Error reading message length: {:?}", e);
                return Err(Error::Io(e));
            }
        }

        let message_length = u32::from_be_bytes(length_bytes) as usize;

        // Read message data
        let mut data = vec![0u8; message_length];
        reader.read_exact(&mut data).await?;

        drop(reader);

        let raw_string = String::from_utf8_lossy(&data).into_owned();
        debug!("<- {raw_string:?}");

        // Record the response if debug logging is enabled
        if log::log_enabled!(log::Level::Debug) {
            trace::record_response(raw_string.clone()).await;
        }

        let message = ResponseMessage::from(&raw_string).with_server_version(self.server_version());

        self.recorder.record_response(&message);

        Ok(message)
    }

    // sends server handshake
    pub(crate) async fn handshake(&self) -> Result<(), Error> {
        let handshake = self.connection_handler.format_handshake();
        debug!("-> handshake: {handshake:?}");

        {
            let mut writer = self.writer.lock().await;
            writer.write_all(&handshake).await?;
        }

        let ack = self.read_message().await;

        let mut connection_metadata = self.connection_metadata.lock().await;

        match ack {
            Ok(mut response) => {
                let handshake_data = self.connection_handler.parse_handshake_response(&mut response)?;
                self.server_version_cache.store(handshake_data.server_version, Ordering::Release);

                let (time, tz) = parse_connection_time(&handshake_data.server_time)?;
                connection_metadata.connection_time = time;
                connection_metadata.time_zone = tz;
            }
            Err(Error::Io(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(Error::Simple(format!("The server may be rejecting connections from this host: {err}")));
            }
            Err(err) => {
                return Err(err);
            }
        }
        Ok(())
    }

    // asks server to start processing messages
    pub(crate) async fn start_api(&self) -> Result<(), Error> {
        let server_version = self.server_version();
        let message = self.connection_handler.format_start_api(self.client_id, server_version);
        self.write_message(&message).await?;
        Ok(())
    }

    // Fetches next order id and managed accounts.
    pub(crate) async fn receive_account_info(&self, startup_callback: Option<&(dyn Fn(ResponseMessage) + Send + Sync)>) -> Result<(), Error> {
        let mut account_info = AccountInfo::default();

        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 100;
        loop {
            let mut message = self.read_message().await?;
            let info = self.connection_handler.parse_account_info(&mut message, startup_callback)?;

            // Merge received info
            if info.next_order_id.is_some() {
                account_info.next_order_id = info.next_order_id;
            }
            if info.managed_accounts.is_some() {
                account_info.managed_accounts = info.managed_accounts;
            }

            attempts += 1;
            if (account_info.next_order_id.is_some() && account_info.managed_accounts.is_some()) || attempts > MAX_ATTEMPTS {
                break;
            }
        }

        // Update connection metadata
        let mut connection_metadata = self.connection_metadata.lock().await;
        if let Some(next_order_id) = account_info.next_order_id {
            connection_metadata.next_order_id = next_order_id;
        }
        if let Some(managed_accounts) = account_info.managed_accounts {
            connection_metadata.managed_accounts = managed_accounts;
        }

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::AsyncConnection;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{mpsc, Arc},
        thread,
        time::Duration,
    };

    use crate::{client::common::tests::setup_connect, messages::encode_length, server_versions};

    const CLIENT_ID: i32 = 100;

    #[tokio::test]
    async fn test_reset_connection_metadata_clears_handshake_state() {
        let gateway = setup_connect();
        let connection = AsyncConnection::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect");

        let metadata = connection.connection_metadata().await;
        assert_eq!(metadata.client_id, CLIENT_ID);
        assert_eq!(metadata.server_version, gateway.server_version());
        assert_eq!(metadata.next_order_id, 90);
        assert_eq!(metadata.managed_accounts, "2334");
        assert!(metadata.connection_time.is_some());
        assert!(metadata.time_zone.is_some());

        connection.reset_connection_metadata().await;

        let metadata = connection.connection_metadata().await;
        assert_eq!(metadata.client_id, CLIENT_ID);
        assert_eq!(metadata.server_version, 0);
        assert_eq!(metadata.next_order_id, 0);
        assert_eq!(metadata.managed_accounts, "");
        assert!(metadata.connection_time.is_none());
        assert!(metadata.time_zone.is_none());
    }

    #[tokio::test]
    async fn test_reconnect_clears_metadata_while_waiting_for_server_version_handshake() {
        let mut gateway = PausedReconnectGateway::start(server_versions::IPO_PRICES);
        let connection = Arc::new(AsyncConnection::connect(&gateway.address(), CLIENT_ID).await.expect("Failed to connect"));

        let metadata = connection.connection_metadata().await;
        assert_eq!(metadata.server_version, gateway.server_version);
        assert_eq!(metadata.next_order_id, 90);
        assert_eq!(metadata.managed_accounts, "2334");
        assert!(metadata.connection_time.is_some());
        assert!(metadata.time_zone.is_some());

        let reconnect = {
            let connection = Arc::clone(&connection);
            tokio::spawn(async move { connection.reconnect().await })
        };

        gateway.wait_for_paused_reconnect_handshake().await;

        let metadata = connection.connection_metadata().await;
        assert_eq!(metadata.client_id, CLIENT_ID);
        assert_eq!(metadata.server_version, 0);
        assert_eq!(metadata.next_order_id, 0);
        assert_eq!(metadata.managed_accounts, "");
        assert!(metadata.connection_time.is_none());
        assert!(metadata.time_zone.is_none());

        gateway.release_reconnect_handshake();

        reconnect.await.expect("reconnect task panicked").expect("reconnect failed");

        let metadata = connection.connection_metadata().await;
        assert_eq!(metadata.client_id, CLIENT_ID);
        assert_eq!(metadata.server_version, gateway.server_version);
        assert_eq!(metadata.next_order_id, 90);
        assert_eq!(metadata.managed_accounts, "2334");
        assert!(metadata.connection_time.is_some());
        assert!(metadata.time_zone.is_some());
    }

    pub(crate) struct PausedReconnectGateway {
        address: String,
        pub(crate) server_version: i32,
        reconnect_handshake_started: mpsc::Receiver<()>,
        release_reconnect: Option<mpsc::Sender<()>>,
        shutdown: Option<mpsc::Sender<()>>,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl PausedReconnectGateway {
        pub(crate) fn start(server_version: i32) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind test gateway");
            let address = listener.local_addr().expect("Failed to read test gateway address");
            let (reconnect_handshake_started, reconnect_handshake_ready) = mpsc::channel();
            let (release_reconnect, reconnect_released) = mpsc::channel();
            let (shutdown_tx, shutdown_rx) = mpsc::channel();

            let handle = thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("Failed to accept initial connection");
                handle_startup(&mut stream, server_version, None).expect("Failed initial startup");
                drop(stream);

                let (mut stream, _) = listener.accept().expect("Failed to accept reconnect");
                handle_startup(&mut stream, server_version, Some((reconnect_handshake_started, reconnect_released)))
                    .expect("Failed reconnect startup");

                // Keep the reconnected stream alive until the test signals shutdown,
                // so callers can observe post-reconnect state before the socket closes
                // and triggers a fresh connection-error cycle.
                let _ = shutdown_rx.recv();
                drop(stream);
            });

            Self {
                address: address.to_string(),
                server_version,
                reconnect_handshake_started: reconnect_handshake_ready,
                release_reconnect: Some(release_reconnect),
                shutdown: Some(shutdown_tx),
                handle: Some(handle),
            }
        }

        pub(crate) fn address(&self) -> String {
            self.address.clone()
        }

        pub(crate) async fn wait_for_paused_reconnect_handshake(&self) {
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    match self.reconnect_handshake_started.try_recv() {
                        Ok(()) => break,
                        Err(mpsc::TryRecvError::Empty) => tokio::time::sleep(Duration::from_millis(10)).await,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            panic!("Reconnect handshake signal channel closed")
                        }
                    }
                }
            })
            .await
            .expect("Reconnect did not reach the server-version handshake wait point");
        }

        pub(crate) fn release_reconnect_handshake(&mut self) {
            if let Some(release_reconnect) = self.release_reconnect.take() {
                release_reconnect.send(()).expect("Failed to release reconnect handshake");
            }
        }
    }

    impl Drop for PausedReconnectGateway {
        fn drop(&mut self) {
            self.release_reconnect_handshake();
            if let Some(shutdown) = self.shutdown.take() {
                let _ = shutdown.send(());
            }
            if let Some(handle) = self.handle.take() {
                handle.join().expect("Failed to join test gateway thread");
            }
        }
    }

    fn handle_startup(
        stream: &mut TcpStream,
        server_version: i32,
        pause_before_handshake_response: Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>,
    ) -> std::io::Result<()> {
        let mut magic_token = [0u8; 4];
        stream.read_exact(&mut magic_token)?;
        assert_eq!(&magic_token, b"API\0");

        let _supported_versions = read_message(stream)?;

        if let Some((started, release)) = pause_before_handshake_response {
            started.send(()).expect("Failed to signal reconnect handshake");
            release
                .recv_timeout(Duration::from_secs(3))
                .expect("Timed out waiting to release reconnect handshake");
        }

        write_message(stream, format!("{server_version}\020240120 12:00:00 EST\0"))?;

        let start_api = read_message(stream)?;
        if server_version > 72 {
            assert_eq!(start_api, "71\02\0100\0\0");
        } else {
            assert_eq!(start_api, "71\02\0100\0");
        }

        write_message(stream, "9\01\090\0".to_string())?;
        write_message(stream, "15\01\02334\0".to_string())?;

        Ok(())
    }

    fn read_message(stream: &mut TcpStream) -> std::io::Result<String> {
        let mut length_bytes = [0u8; 4];
        stream.read_exact(&mut length_bytes)?;
        let message_length = u32::from_be_bytes(length_bytes) as usize;
        let mut data = vec![0u8; message_length];
        stream.read_exact(&mut data)?;
        Ok(String::from_utf8_lossy(&data).into_owned())
    }

    fn write_message(stream: &mut TcpStream, message: String) -> std::io::Result<()> {
        stream.write_all(&encode_length(&message))?;
        stream.flush()
    }
}
