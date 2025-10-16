//! Next Order Id example
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
