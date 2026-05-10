//! Options Purchase example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example options_purchase
//! ```

use ibapi::client::blocking::Client;
use ibapi::{
    contracts::Contract,
    orders::{self, order_builder, PlaceOrder},
};

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::call("AAPL").strike(180.0).expires_on(2025, 2, 21).build();

    let order_id = client.next_valid_order_id().expect("could not get next valid order id");
    //    let order_id = client.next_order_id();
    println!("next order id: {order_id}");

    let order = order_builder::market_order(orders::Action::Buy, 5.0);
    println!("contract: {contract:?}, order: {order:?}");

    let subscription = client.place_order(order_id, &contract, &order).expect("could not place order");
    for status in subscription.iter_data() {
        match status {
            Ok(status) => println!("{status:?}"),
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        }
    }
    let order_id = client.next_order_id();
    println!("next order id: {order_id}");

    let order = order_builder::market_order(orders::Action::Buy, 5.0);
    println!("contract: {contract:?}, order: {order:?}");

    let subscription = client.place_order(order_id, &contract, &order).expect("could not place order");
    for status in subscription.iter_data() {
        let status = match status {
            Ok(status) => status,
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        };
        println!("{status:?}");
        if let PlaceOrder::OrderStatus(order_status) = status {
            if order_status.remaining == 0.0 {
                break;
            }
        }
    }
}
