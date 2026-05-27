//! Smart components example (sync).
//!
//! Requests the underlying exchanges that contribute to a consolidated (BBO)
//! feed. Pass a BBO exchange code as the first argument (defaults to
//! `"a6"`). The BBO code is an opaque per-session token typically obtained
//! from the `LAST_EXCHANGE` market-data tick (tick type 84); `"a6"` is the
//! canonical sample value used in the IBKR Python testbed.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example smart_components -- a6
//! ```

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let bbo_exchange = std::env::args().nth(1).unwrap_or_else(|| "a6".to_string());

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let components = client.smart_components(&bbo_exchange).expect("smart_components request failed");

    println!("Smart components for {bbo_exchange}:");
    for component in &components {
        println!("  bit {}: {} ({})", component.bit_number, component.exchange, component.exchange_letter);
    }
}
