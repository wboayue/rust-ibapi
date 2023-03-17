//! [![github]](https://github.com/wboayue/rust-ibapi)&ensp;[![crates-io]](https://crates.io/crates/ibapi)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//!
//! <br>
//!
//! An implementation of the Interactive Brokers [TWS API](https://interactivebrokers.github.io/tws-api/introduction.html) for Rust. The official TWS API is an event driven API. This implementation provides a synchronous API that simplifies the development of trading strategies.
//!
//! This is a work in progress and targets support for TWS API 10.20. The primary reference for this implementation is the [C# source code](https://github.com/InteractiveBrokers/tws-api-public).
//!
//! The initial release focuses on APIs for [contracts](crate::contracts), [realtime data](crate::market_data::realtime) and [order management](crate::orders).
//! 
//! The list of open issues are tracked [here](https://github.com/wboayue/rust-ibapi/issues). If you run into an issue or need a missing feature, check the [issues list](https://github.com/wboayue/rust-ibapi/issues) first and then report the issue if it is not already tracked.
//!
//!```no_run
//!     use anyhow;
//!     use ibapi::client::Client;     
//!     
//!     fn main() -> anyhow::Result<()> {
//!         let client = Client::connect("localhost:4002:100")?;
//!         println!("Client: {:?}", client);
//!         Ok(())
//!     }
//!```

/// TSW API Client.
pub mod client;

/// Describes primary data structures used by the model.
pub(crate) mod domain;

/// A [Contract](crate::contracts::Contract) object represents trading instruments such as a stocks, futures or options.
///
/// Every time a new request that requires a contract (i.e. market data, order placing, etc.) is sent to the API, the system will try to match the provided contract object with a single candidate. If there is more than one contract matching the same description, the API will return an error notifying you there is an ambiguity. In these cases the API needs further information to narrow down the list of contracts matching the provided description to a single element.
pub mod contracts;

/// APIs for retrieving market data
pub mod market_data;

/// News
pub(crate) mod news;

/// APIs for placing orders
pub mod orders;

/// APIs for requesting account information
mod accounts;

mod constants;
mod messages;
mod server_versions;

pub(crate) mod stubs;

// TODO make public to crate only
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

impl ToField for &str {
    fn to_field(&self) -> String {
        <&str>::clone(self).to_string()
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

fn encode_option_field<T: ToField>(val: &Option<T>) -> String {
    match val {
        Some(val) => val.to_field(),
        None => String::from(""),
    }
}
