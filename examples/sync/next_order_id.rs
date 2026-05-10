//! Next Order Id (advanced — bring your own order id)
//!
//! For normal use, prefer `client.order(&contract).buy(qty).<type>().submit()` —
//! `submit()` allocates the next id internally. This example demonstrates the manual
//! id-allocation flow used when coordinating with an external allocator (e.g. a
//! multi-process system that reserves ids out of band).
//!
//! - `next_valid_order_id()` round-trips to TWS to fetch the server-side starting id
//!   (useful right after connect, or after a manual id reset).
//! - `next_order_id()` (not shown here) returns the next id from the in-memory
//!   counter without contacting TWS — use it for repeated allocations after the
//!   initial `next_valid_order_id()`.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example next_order_id
//! ```

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).unwrap();

    let order_id = client.next_valid_order_id().unwrap();
    println!("Next valid order id: {order_id}");
}
