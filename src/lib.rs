//! [![github]](https://github.com/wboayue/rust-ibapi)&ensp;[![crates-io]](https://crates.io/crates/ibapi)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//!
//! <br>
//!
//! A wrapper around the procedural macro API of the compiler's [`proc_macro`]
//! crate. This library serves two purposes:
//!
//! [`proc_macro`]: https://doc.rust-lang.org/proc_macro/
//! Fast and easy queue abstraction.
//!```no_run
//!     use anyhow;
//!     use ibapi::client::IBClient;     
//!     
//!     fn main() -> anyhow::Result<()> {
//!         let client = IBClient::connect("localhost:4002:100")?;
//!         println!("Client: {:?}", client);
//!         Ok(())
//!     }
//!```
/// TSW API Client.
pub mod client;

/// Describes primary data structures used by the model.
pub mod domain;

/// A [Contract] object represents trading instruments such as a stocks, futures or options.
///
/// Every time a new request that requires a contract (i.e. market data, order placing, etc.) is sent to the API, the system will try to match the provided contract object with a single candidate. If there is more than one contract matching the same description, the API will return an error notifying you there is an ambiguity. In these cases the API needs further information to narrow down the list of contracts matching the provided description to a single element.
pub mod contracts;

pub mod market_data;

/// News
pub mod news;

/// APIs for placing orders
pub mod orders;

/// APIs for requesting account information
pub mod accounts;

mod constants;
mod messages;
mod server_versions;

pub(crate) mod stubs;
