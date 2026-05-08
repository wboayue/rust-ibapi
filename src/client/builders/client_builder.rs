//! Fluent builder for constructing a [`Client`](crate::Client).
//!
//! Replaces the v2-era `Client::connect_with_options` / `connect_with_callback`
//! entry points and folds the handshake-time notice surface into a single
//! linear chain. Pick one of two terminals based on whether handshake notices
//! matter:
//!
//! ```no_run
//! # #[cfg(feature = "async")]
//! # async fn run() -> Result<(), ibapi::Error> {
//! use ibapi::Client;
//!
//! // Connect, no handshake-notice stream
//! let client = Client::builder()
//!     .address("127.0.0.1:4002")
//!     .client_id(100)
//!     .connect()
//!     .await?;
//! drop(client);
//!
//! // Connect AND get a stream that captures handshake notices too
//! let (client, mut notices) = Client::builder()
//!     .address("127.0.0.1:4002")
//!     .client_id(101)
//!     .connect_with_notice_stream()
//!     .await?;
//! while let Some(n) = notices.next().await {
//!     println!("{n}");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! The sync builder lives at `client::blocking::ClientBuilder` and mirrors the
//! shape exactly — no `.await`, [`crate::client::blocking::NoticeStream`]
//! instead of the async one.

use std::sync::Arc;

use crate::connection::common::StartupMessage;
use crate::errors::Error;

/// Configuration state shared by [`sync_impl::ClientBuilder`] and
/// [`async_impl::ClientBuilder`]. Centralizes the field set and the
/// `InvalidArgument` validation messages so future configurators only need
/// to be added in one place. Terminals call [`BuilderState::validate`] to
/// extract the checked pieces.
#[derive(Default)]
pub(super) struct BuilderState {
    pub(super) address: Option<String>,
    pub(super) client_id: Option<i32>,
    pub(super) tcp_no_delay: bool,
    pub(super) startup_callback: Option<Arc<dyn Fn(StartupMessage) + Send + Sync>>,
}

/// Output of [`BuilderState::validate`] — same fields, but `address` and
/// `client_id` have been unwrapped.
pub(super) struct ValidatedPieces {
    pub(super) address: String,
    pub(super) client_id: i32,
    pub(super) tcp_no_delay: bool,
    pub(super) startup_callback: Option<Arc<dyn Fn(StartupMessage) + Send + Sync>>,
}

impl BuilderState {
    pub(super) fn validate(self) -> Result<ValidatedPieces, Error> {
        Ok(ValidatedPieces {
            address: self
                .address
                .ok_or_else(|| Error::InvalidArgument("ClientBuilder: address is required".into()))?,
            client_id: self
                .client_id
                .ok_or_else(|| Error::InvalidArgument("ClientBuilder: client_id is required".into()))?,
            tcp_no_delay: self.tcp_no_delay,
            startup_callback: self.startup_callback,
        })
    }
}

#[cfg(feature = "sync")]
pub mod sync_impl {
    //! Sync `ClientBuilder` for the blocking transport.

    use std::sync::Arc;

    use super::BuilderState;
    use crate::client::sync::Client;
    use crate::connection::common::StartupMessage;
    use crate::errors::Error;
    use crate::subscriptions::notice_stream::sync_impl::NoticeStream;
    use crate::transport::sync::NoticeBroadcaster;

    /// Builder for a synchronous [`Client`]. Acquire via
    /// [`Client::builder`](crate::client::blocking::Client::builder).
    ///
    /// Configurators (`address`, `client_id`, `tcp_no_delay`, `startup_callback`)
    /// chain on `self`. Terminate with [`connect`](Self::connect) or
    /// [`connect_with_notice_stream`](Self::connect_with_notice_stream).
    #[derive(Default)]
    #[must_use = "ClientBuilder does nothing until you call connect() or connect_with_notice_stream()"]
    pub struct ClientBuilder {
        state: BuilderState,
    }

    impl ClientBuilder {
        /// TWS / IB Gateway address, e.g. `"127.0.0.1:4002"`. Required.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # use ibapi::client::blocking::Client;
        /// let _ = Client::builder().address("127.0.0.1:4002").client_id(100).connect();
        /// ```
        pub fn address(mut self, addr: impl Into<String>) -> Self {
            self.state.address = Some(addr.into());
            self
        }

        /// Client id, e.g. `100`. Required.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # use ibapi::client::blocking::Client;
        /// let _ = Client::builder().address("127.0.0.1:4002").client_id(100).connect();
        /// ```
        pub fn client_id(mut self, id: i32) -> Self {
            self.state.client_id = Some(id);
            self
        }

        /// Enable `TCP_NODELAY` on the socket (disables Nagle, lower latency).
        /// Default: `false`.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # use ibapi::client::blocking::Client;
        /// let _ = Client::builder().address("127.0.0.1:4002").client_id(100).tcp_no_delay(true).connect();
        /// ```
        pub fn tcp_no_delay(mut self, enabled: bool) -> Self {
            self.state.tcp_no_delay = enabled;
            self
        }

        /// Set a callback for unsolicited typed messages during the handshake.
        ///
        /// Fires for `OpenOrder`, `OrderStatus`, account updates, and other
        /// frames TWS emits before `next_valid_id` lands. Callback fires on the
        /// initial connect *and* every auto-reconnect handshake.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # use ibapi::client::blocking::Client;
        /// # use ibapi::StartupMessage;
        /// let _ = Client::builder()
        ///     .address("127.0.0.1:4002")
        ///     .client_id(100)
        ///     .startup_callback(|msg| if let StartupMessage::OpenOrder(o) = msg {
        ///         println!("startup open order: {}", o.order_id);
        ///     })
        ///     .connect();
        /// ```
        pub fn startup_callback(mut self, callback: impl Fn(StartupMessage) + Send + Sync + 'static) -> Self {
            self.state.startup_callback = Some(Arc::new(callback));
            self
        }

        /// Establish the connection and return a [`Client`].
        ///
        /// Handshake-time notices are not surfaced to the caller — see
        /// [`connect_with_notice_stream`](Self::connect_with_notice_stream) if
        /// you need them. Post-connect, `client.notice_stream()` still works
        /// for runtime-only unrouted notices.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # use ibapi::client::blocking::Client;
        /// let client = Client::builder()
        ///     .address("127.0.0.1:4002")
        ///     .client_id(100)
        ///     .connect()
        ///     .expect("connection failed");
        /// drop(client);
        /// ```
        pub fn connect(self) -> Result<Client, Error> {
            let broadcaster = Arc::new(NoticeBroadcaster::new());
            self.connect_with_broadcaster(broadcaster)
        }

        /// Establish the connection AND a pre-bound [`NoticeStream`] that
        /// captures handshake-time notices (farm-status 2104/2106/2158,
        /// connectivity 1100/1101/1102, etc.) plus every unrouted notice for
        /// the lifetime of the connection. Survives auto-reconnects.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # use ibapi::client::blocking::Client;
        /// let (client, notices) = Client::builder()
        ///     .address("127.0.0.1:4002")
        ///     .client_id(100)
        ///     .connect_with_notice_stream()
        ///     .expect("connection failed");
        /// for n in notices.iter() {
        ///     println!("{n}");
        /// }
        /// drop(client);
        /// ```
        pub fn connect_with_notice_stream(self) -> Result<(Client, NoticeStream), Error> {
            let broadcaster = Arc::new(NoticeBroadcaster::new());
            let stream = NoticeStream::new(broadcaster.subscribe());
            let client = self.connect_with_broadcaster(broadcaster)?;
            Ok((client, stream))
        }

        fn connect_with_broadcaster(self, broadcaster: Arc<NoticeBroadcaster>) -> Result<Client, Error> {
            let pieces = self.state.validate()?;
            Client::connect_with_pieces(
                &pieces.address,
                pieces.client_id,
                pieces.tcp_no_delay,
                pieces.startup_callback,
                broadcaster,
            )
        }
    }
}

#[cfg(feature = "async")]
pub mod async_impl {
    //! Async `ClientBuilder` for the tokio-backed transport.

    use std::sync::Arc;

    use tokio::sync::broadcast;

    use super::BuilderState;
    use crate::client::r#async::Client;
    use crate::connection::common::StartupMessage;
    use crate::errors::Error;
    use crate::messages::Notice;
    use crate::subscriptions::notice_stream::async_impl::NoticeStream;
    use crate::transport::r#async::BROADCAST_CHANNEL_CAPACITY;

    /// Builder for an async [`Client`]. Acquire via
    /// [`Client::builder`](crate::Client::builder).
    ///
    /// Configurators (`address`, `client_id`, `tcp_no_delay`, `startup_callback`)
    /// chain on `self`. Terminate with [`connect`](Self::connect) or
    /// [`connect_with_notice_stream`](Self::connect_with_notice_stream).
    #[derive(Default)]
    #[must_use = "ClientBuilder does nothing until you call connect() or connect_with_notice_stream()"]
    pub struct ClientBuilder {
        state: BuilderState,
    }

    impl ClientBuilder {
        /// TWS / IB Gateway address, e.g. `"127.0.0.1:4002"`. Required.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # async fn run() -> Result<(), ibapi::Error> {
        /// use ibapi::Client;
        /// let _client = Client::builder().address("127.0.0.1:4002").client_id(100).connect().await?;
        /// # Ok(()) }
        /// ```
        pub fn address(mut self, addr: impl Into<String>) -> Self {
            self.state.address = Some(addr.into());
            self
        }

        /// Client id, e.g. `100`. Required.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # async fn run() -> Result<(), ibapi::Error> {
        /// use ibapi::Client;
        /// let _client = Client::builder().address("127.0.0.1:4002").client_id(100).connect().await?;
        /// # Ok(()) }
        /// ```
        pub fn client_id(mut self, id: i32) -> Self {
            self.state.client_id = Some(id);
            self
        }

        /// Enable `TCP_NODELAY` on the socket (disables Nagle, lower latency).
        /// Default: `false`.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # async fn run() -> Result<(), ibapi::Error> {
        /// use ibapi::Client;
        /// let _client = Client::builder().address("127.0.0.1:4002").client_id(100).tcp_no_delay(true).connect().await?;
        /// # Ok(()) }
        /// ```
        pub fn tcp_no_delay(mut self, enabled: bool) -> Self {
            self.state.tcp_no_delay = enabled;
            self
        }

        /// Set a callback for unsolicited typed messages during the handshake.
        ///
        /// Fires for `OpenOrder`, `OrderStatus`, account updates, and other
        /// frames TWS emits before `next_valid_id` lands. Callback fires on the
        /// initial connect *and* every auto-reconnect handshake.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # async fn run() -> Result<(), ibapi::Error> {
        /// use ibapi::{Client, StartupMessage};
        /// let _client = Client::builder()
        ///     .address("127.0.0.1:4002")
        ///     .client_id(100)
        ///     .startup_callback(|msg| if let StartupMessage::OpenOrder(o) = msg {
        ///         println!("startup open order: {}", o.order_id);
        ///     })
        ///     .connect()
        ///     .await?;
        /// # Ok(()) }
        /// ```
        pub fn startup_callback(mut self, callback: impl Fn(StartupMessage) + Send + Sync + 'static) -> Self {
            self.state.startup_callback = Some(Arc::new(callback));
            self
        }

        /// Establish the connection and return a [`Client`].
        ///
        /// Handshake-time notices are not surfaced to the caller — see
        /// [`connect_with_notice_stream`](Self::connect_with_notice_stream) if
        /// you need them. Post-connect, `client.notice_stream()` still works
        /// for runtime-only unrouted notices.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # async fn run() -> Result<(), ibapi::Error> {
        /// use ibapi::Client;
        /// let client = Client::builder()
        ///     .address("127.0.0.1:4002")
        ///     .client_id(100)
        ///     .connect()
        ///     .await?;
        /// drop(client);
        /// # Ok(()) }
        /// ```
        pub async fn connect(self) -> Result<Client, Error> {
            let (sender, _rx) = broadcast::channel::<Notice>(BROADCAST_CHANNEL_CAPACITY);
            self.connect_with_sender(sender).await
        }

        /// Establish the connection AND a pre-bound [`NoticeStream`] that
        /// captures handshake-time notices (farm-status 2104/2106/2158,
        /// connectivity 1100/1101/1102, etc.) plus every unrouted notice for
        /// the lifetime of the connection. Survives auto-reconnects.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # async fn run() -> Result<(), ibapi::Error> {
        /// use ibapi::Client;
        /// let (client, mut notices) = Client::builder()
        ///     .address("127.0.0.1:4002")
        ///     .client_id(100)
        ///     .connect_with_notice_stream()
        ///     .await?;
        /// while let Some(n) = notices.next().await {
        ///     println!("{n}");
        /// }
        /// drop(client);
        /// # Ok(()) }
        /// ```
        pub async fn connect_with_notice_stream(self) -> Result<(Client, NoticeStream), Error> {
            let (sender, receiver) = broadcast::channel::<Notice>(BROADCAST_CHANNEL_CAPACITY);
            let stream = NoticeStream::new(receiver);
            let client = self.connect_with_sender(sender).await?;
            Ok((client, stream))
        }

        async fn connect_with_sender(self, sender: broadcast::Sender<Notice>) -> Result<Client, Error> {
            let pieces = self.state.validate()?;
            Client::connect_with_pieces(&pieces.address, pieces.client_id, pieces.tcp_no_delay, pieces.startup_callback, sender).await
        }
    }
}

#[cfg(test)]
#[path = "client_builder_tests.rs"]
mod tests;
