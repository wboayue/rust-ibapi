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
//!     use ibapi::client::BasicClient;     
//!     
//!     fn main() -> anyhow::Result<()> {
//!         let client = BasicClient::connect("localhost:4002:100")?;
//!         println!("Client: {:?}", client);
//!         Ok(())
//!     }
//!```
/// TSW API Client.
pub mod client;

/// Describes primary data structures used by the model.
pub mod domain;

pub mod contracts;

pub mod historical_market_data;

/// News
pub mod news;

pub mod server_versions;

pub mod streaming_market_data;

pub mod transport;
