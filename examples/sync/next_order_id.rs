//! Bring-your-own order id (advanced)
//!
//! For normal use, prefer `client.order(&contract).buy(qty).<type>().submit()` —
//! `submit()` allocates the next id internally. This example demonstrates the manual
//! id-allocation flow used when coordinating with an external allocator (e.g. a
//! multi-process system that reserves ids out of band).
//!
//! - `next_valid_order_id()` round-trips to TWS to fetch the server-side starting id
//!   (useful right after connect, or after a manual id reset).
//! - `next_order_id()` (in-memory) returns subsequent ids without contacting TWS.
//! - `OrderBuilder::build_order()` returns the bare `Order` from the fluent builder
//!   so you can submit with your own id via `place_order` / `submit_order`.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example next_order_id
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    // Fetch the server-side starting id. In a real BYO-id flow this would come from
    // your external allocator instead.
    let order_id = client.next_valid_order_id().expect("could not fetch next valid order id");
    println!("Allocated order id: {order_id}");

    // Build the order with the fluent builder, but stop short of submitting.
    let contract = Contract::stock("AAPL").build();
    let order = client.order(&contract).buy(100).market().build_order().expect("order build failed");

    // Submit with the externally-allocated id.
    client.submit_order(order_id, &contract, &order).expect("submit failed");
    println!("Submitted order with externally-allocated id: {order_id}");
}
