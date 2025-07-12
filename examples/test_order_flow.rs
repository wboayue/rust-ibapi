//! Test complete order flow to capture exact message formats

use ibapi::contracts::Contract;
use ibapi::orders::{order_builder, Action};
use ibapi::Client;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Enable message recording
    env::set_var("IBAPI_RECORDING_DIR", "/tmp/order-flow-test");
    std::fs::create_dir_all("/tmp/order-flow-test")?;

    let client = Client::connect("127.0.0.1:4002", 100)?;
    println!("Connected to TWS/Gateway");

    // Get next valid order ID
    let order_id = client.next_valid_order_id()?;
    println!("Next valid order ID: {order_id:?}");

    // Create an ES futures order
    let contract = Contract {
        symbol: "ES".to_string(),
        security_type: ibapi::contracts::SecurityType::Future,
        exchange: "CME".to_string(),
        currency: "USD".to_string(),
        local_symbol: "ESU5".to_string(),
        ..Default::default()
    };

    // Use a market order that might fill immediately
    let mut order = order_builder::market_order(Action::Buy, 1.0);
    order.order_id = order_id;

    println!("\nPlacing MARKET order {order_id} for 1 ESU5 contract...");

    // Place the order and capture responses
    let subscription = client.place_order(order_id, &contract, &order)?;

    println!("\nWaiting for order responses (10 seconds):");
    let mut count = 0;

    // Collect responses
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 10 {
        if let Some(msg) = subscription.try_next() {
            count += 1;
            println!("[{count}] {msg:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("\n\nRaw messages saved to: /tmp/order-flow-test/");
    println!("Use parse_recorded_messages to extract the messages");

    Ok(())
}
