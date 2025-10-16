//! Bracket Order example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example bracket_order
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::Action;
use ibapi::orders::{order_builder, PlaceOrder};
use std::thread;

fn place_bracket_order(client: &Client, contract: &Contract, parent_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    let orders = order_builder::bracket_order(parent_id, Action::Buy, 100.0, 220.00, 230.0, 210.0);
    let mut subscriptions = Vec::new();

    for order in &orders {
        let subscription = client.place_order(order.order_id, contract, order)?;
        subscriptions.push(subscription);
    }

    let mut num_submitted = 0;
    while num_submitted < orders.len() {
        subscriptions
            .iter()
            .filter_map(|subscription| subscription.try_next())
            .for_each(|event| match event {
                PlaceOrder::OrderStatus(event) if event.status == "Submitted" => {
                    println!("{event:?}");
                    num_submitted += 1;
                }
                _ => println!("Received other event: {event:?}"),
            });
        thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("Bracket order placed successfully");
    Ok(())
}

fn main() {
    env_logger::init();
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let parent_id = client.next_valid_order_id().expect("error getting next order id");
    let contract = Contract::stock("AAPL").build();

    if let Err(e) = place_bracket_order(&client, &contract, parent_id) {
        eprintln!("Failed to place bracket order: {e}");
    }
}
