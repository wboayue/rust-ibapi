//! [![github]](https://github.com/wboayue/rust-ibapi)&ensp;[![crates-io]](https://crates.io/crates/ibapi)&ensp;[![license]](https://opensource.org/licenses/MIT)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [license]: https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge&labelColor=555555
//!
//! <br>
//!
//! A comprehensive Rust implementation of the Interactive Brokers TWS API, providing a robust and
//! user-friendly interface for TWS and IB Gateway. Designed with simplicity in mind, it integrates smoothly into trading systems.
//!
//! **API Documentation:**
//! * [TWS API Reference](https://interactivebrokers.github.io/tws-api/introduction.html) - Detailed technical documentation
//! * [IBKR Campus](https://ibkrcampus.com/ibkr-api-page/trader-workstation-api/) - IB's official learning platform
//!
//! This fully featured API enables the retrieval of account information, access to real-time and historical market data, order management,
//! market scanning, and access to news and Wall Street Horizons (WSH) event data. Future updates will focus on bug fixes,
//! maintaining parity with the official API, and enhancing usability.
//!
//! For an overview of API usage, refer to the [README](https://github.com/wboayue/rust-ibapi/blob/main/README.md).

#![warn(missing_docs)]
// Allow octal-looking escapes in string literals (used in test data)
#![allow(clippy::octal_escapes)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::useless_format)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::assertions_on_constants)]

// Feature guards
#[cfg(not(any(feature = "sync", feature = "async")))]
compile_error!(
    "You must enable at least one of the 'sync' or 'async' features to use this crate.\n\
     The 'async' feature is enabled by default; if you disabled default features, be sure to\n\
     opt back into either API:\n\
         ibapi = { version = \"2.0\", default-features = false, features = [\"sync\"] }\n\
         ibapi = { version = \"2.0\", default-features = false, features = [\"async\"] }\n\
     You may also enable both to access the synchronous API under `client::blocking`."
);

/// Describes items present in an account.
pub mod accounts;

/// TWS API Client.
///
/// The Client establishes the connection to TWS or the Gateway.
/// It manages the routing of messages between TWS and the application.
pub mod client;

pub(crate) mod transport;

/// Connection management
pub(crate) mod connection;

/// Callback for handling unsolicited messages during connection setup.
///
/// When TWS sends messages like `OpenOrder` or `OrderStatus` during the connection
/// handshake, this callback is invoked to allow the application to process them
/// instead of discarding them.
///
/// # Example
///
/// ```ignore
/// use ibapi::{Client, StartupMessageCallback};
/// use ibapi::messages::IncomingMessages;
/// use std::sync::{Arc, Mutex};
///
/// #[tokio::main]
/// async fn main() {
///     let orders = Arc::new(Mutex::new(Vec::new()));
///     let orders_clone = orders.clone();
///
///     let callback: StartupMessageCallback = Box::new(move |msg| {
///         match msg.message_type() {
///             IncomingMessages::OpenOrder | IncomingMessages::OrderStatus => {
///                 orders_clone.lock().unwrap().push(msg);
///             }
///             _ => {}
///         }
///     });
///
///     let client = Client::connect_with_callback("127.0.0.1:4002", 100, Some(callback))
///         .await
///         .expect("connection failed");
///
///     println!("Received {} startup orders", orders.lock().unwrap().len());
/// }
/// ```
pub use connection::StartupMessageCallback;

/// Common utilities shared across modules
pub(crate) mod common;

/// Display groups subscription support
pub mod display_groups;

/// Subscription types for streaming data
pub mod subscriptions;

/// A [Contract](crate::contracts::Contract) object represents trading instruments such as a stocks, futures or options.
///
/// Every time a new request that requires a contract (i.e. market data, order placing, etc.) is sent to the API, the system will try to match the provided contract object with a single candidate. If there is more than one contract matching the same description, the API will return an error notifying you there is an ambiguity. In these cases the API needs further information to narrow down the list of contracts matching the provided description to a single element.
pub mod contracts;
// Describes primary data structures used by the model.
pub mod errors;
/// APIs for retrieving market data
pub mod market_data;
pub mod messages;
/// APIs for retrieving news data including articles, bulletins, and providers
pub mod news;
/// Data types for building and placing orders.
pub mod orders;
/// APIs for working with the market scanner.
pub mod scanner;
/// APIs for working with Wall Street Horizon: Earnings Calendar & Event Data.
pub mod wsh;

/// Server interaction tracing for debugging and monitoring
pub mod trace;

/// A prelude module for convenient importing of commonly used types.
pub mod prelude;

/// Protocol version checking and constants for TWS API features.
pub mod protocol;

mod server_versions;

#[doc(inline)]
pub use errors::Error;

#[doc(inline)]
pub use client::Client;
use std::sync::LazyLock;
use time::{
    format_description::{self, BorrowedFormatItem},
    Date,
};

#[cfg(test)]
pub(crate) mod stubs;

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
pub(crate) mod testdata;

// ToField

pub(crate) trait ToField {
    fn to_field(&self) -> String;
}

impl ToField for bool {
    fn to_field(&self) -> String {
        if *self {
            String::from("1")
        } else {
            String::from("0")
        }
    }
}

impl ToField for String {
    fn to_field(&self) -> String {
        self.clone()
    }
}

impl ToField for Option<String> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for &str {
    fn to_field(&self) -> String {
        <&str>::clone(self).to_string()
    }
}

impl ToField for Option<&str> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for usize {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for i32 {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<i32> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for f64 {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<f64> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

fn date_format() -> Vec<BorrowedFormatItem<'static>> {
    format_description::parse("[year][month][day]").unwrap()
}

static DATE_FORMAT: LazyLock<Vec<BorrowedFormatItem<'static>>> = LazyLock::new(date_format);

impl ToField for Date {
    fn to_field(&self) -> String {
        self.format(&DATE_FORMAT).unwrap()
    }
}

impl ToField for Option<Date> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

fn encode_option_field<T: ToField>(val: &Option<T>) -> String {
    match val {
        Some(val) => val.to_field(),
        None => String::from(""),
    }
}

// max attempts to retry failed tws requests
const MAX_RETRIES: i32 = 5;
