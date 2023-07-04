//! [![github]](https://github.com/wboayue/rust-ibapi)&ensp;[![crates-io]](https://crates.io/crates/ibapi)&ensp;[![license]](https://opensource.org/licenses/MIT)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [license]: https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge&labelColor=555555
//!
//! <br>
//!
//! An implementation of the Interactive Brokers [TWS API](https://interactivebrokers.github.io/tws-api/introduction.html) for Rust.
//! This implementation is not a direct port of the official TWS API.
//! It provides a synchronous API that simplifies the development of trading strategies.
//!
//! This is a work in progress and was tested using TWS 10.19. The primary reference for this implementation is the [C# source code](https://github.com/InteractiveBrokers/tws-api-public).
//!
//! The following example gives a flavor of the API style. It is not a trading strategy recommendation and not a complete implementation.
//!
//!```no_run
//! use std::collections::VecDeque;
//!
//! use ibapi::contracts::Contract;
//! use ibapi::market_data::realtime::{BarSize, Bar, WhatToShow};
//! use ibapi::orders::{order_builder, Action, OrderNotification};
//! use ibapi::Client;
//!
//! let client = Client::connect("127.0.0.1:4002", 100).unwrap();
//!
//! let symbol = "TSLA";
//! let contract = Contract::stock(symbol); // defaults to USD and SMART exchange.
//!
//! let bars = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, false).unwrap();
//!
//! let mut channel = BreakoutChannel::new(30);
//!
//! for bar in bars {
//!     channel.add_bar(&bar);
//!
//!     // Ensure enough bars and no open positions.
//!     if !channel.ready() || has_position(&client, symbol) {
//!         continue;
//!     }
//!
//!     let action = if bar.close > channel.high() {
//!         Action::Buy
//!     } else if bar.close < channel.low() {
//!         Action::Sell
//!     } else {
//!         continue;
//!     };
//!
//!     let order_id = client.next_order_id();
//!     let order = order_builder::market_order(action, 100.0);
//!
//!     let notices = client.place_order(order_id, &contract, &order).unwrap();
//!     for notice in notices {
//!         if let OrderNotification::ExecutionData(data) = notice {
//!             println!("{} {} shares of {}", data.execution.side, data.execution.shares, data.contract.symbol);
//!         } else {
//!             println!("{:?}", notice);
//!         }
//!     }
//! }
//!
//! fn has_position(client: &Client, symbol: &str) -> bool {
//!     if let Ok(mut positions) = client.positions() {
//!         positions.find(|p| p.contract.symbol == symbol).is_some()
//!     } else {
//!         false
//!     }
//! }
//!
//! struct BreakoutChannel {
//!     ticks: VecDeque<(f64, f64)>,
//!     size: usize,
//! }
//!
//! impl BreakoutChannel {
//!     fn new(size: usize) -> BreakoutChannel {
//!         BreakoutChannel {
//!             ticks: VecDeque::with_capacity(size + 1),
//!             size,
//!         }
//!     }
//!
//!     fn ready(&self) -> bool {
//!         self.ticks.len() >= self.size
//!     }
//!
//!     fn add_bar(&mut self, bar: &Bar) {
//!         self.ticks.push_back((bar.high, bar.low));
//!
//!         if self.ticks.len() > self.size {
//!             self.ticks.pop_front();
//!         }
//!     }
//!
//!     fn high(&self) -> f64 {
//!         self.ticks.iter().map(|x| x.0).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()
//!     }
//!
//!     fn low(&self) -> f64 {
//!         self.ticks.iter().map(|x| x.1).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()
//!     }
//! }
//!```

/// Describes items present in an account.
pub mod accounts;

/// TSW API Client.
///
/// The Client establishes the connection to TWS or the Gateway.
/// It manages the routing of messages between TWS and the application.
pub mod client;

/// A [Contract](crate::contracts::Contract) object represents trading instruments such as a stocks, futures or options.
///
/// Every time a new request that requires a contract (i.e. market data, order placing, etc.) is sent to the API, the system will try to match the provided contract object with a single candidate. If there is more than one contract matching the same description, the API will return an error notifying you there is an ambiguity. In these cases the API needs further information to narrow down the list of contracts matching the provided description to a single element.
pub mod contracts;
// Describes primary data structures used by the model.
//pub(crate) mod domain;
pub mod errors;
/// APIs for retrieving market data
pub mod market_data;
mod messages;
pub(crate) mod news;
/// Data types for building and placing orders.
pub mod orders;

mod server_versions;
pub(crate) mod stubs;

#[doc(inline)]
pub use errors::Error;

#[doc(inline)]
pub use client::Client;

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
