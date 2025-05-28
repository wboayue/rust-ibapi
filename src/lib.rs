//! [![github]](https://github.com/wboayue/rust-ibapi)&ensp;[![crates-io]](https://crates.io/crates/ibapi)&ensp;[![license]](https://opensource.org/licenses/MIT)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [license]: https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge&labelColor=555555
//!
//! <br>
//!
//! A comprehensive Rust implementation of the Interactive Brokers [TWS API](https://interactivebrokers.github.io/tws-api/introduction.html), providing a robust and
//! user-friendly interface for TWS and IB Gateway. Designed with simplicity in mind, it integrates smoothly into trading systems.
//!
//! This fully featured API enables the retrieval of account information, access to real-time and historical market data, order management,
//! market scanning, and access to news and Wall Street Horizons (WSH) event data. Future updates will focus on bug fixes,
//! maintaining parity with the official API, and enhancing usability.
//!
//! For an overview of API usage, refer to the [README](https://github.com/wboayue/rust-ibapi/blob/main/README.md).

/// Describes items present in an account.
pub mod accounts;

/// TSW API Client.
///
/// The Client establishes the connection to TWS or the Gateway.
/// It manages the routing of messages between TWS and the application.
pub mod client;

pub(crate) mod transport;

/// A [Contract](crate::contracts::Contract) object represents trading instruments such as a stocks, futures or options.
///
/// Every time a new request that requires a contract (i.e. market data, order placing, etc.) is sent to the API, the system will try to match the provided contract object with a single candidate. If there is more than one contract matching the same description, the API will return an error notifying you there is an ambiguity. In these cases the API needs further information to narrow down the list of contracts matching the provided description to a single element.
pub mod contracts;
// Describes primary data structures used by the model.
pub mod errors;
/// APIs for retrieving market data
pub mod market_data;
mod messages;
pub mod news;
/// Data types for building and placing orders.
pub mod orders;
/// APIs for working with the market scanner.
pub mod scanner;
/// APIs for working with Wall Street Horizon: Earnings Calendar & Event Data.
pub mod wsh;

/// A prelude module for convenient importing of commonly used types.
pub mod prelude;

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
