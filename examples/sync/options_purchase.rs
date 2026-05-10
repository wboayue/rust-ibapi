//! Options Purchase example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example options_purchase
//! ```

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::OrderUpdate;

fn main() {
    env_logger::init();

    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).expect("connection failed"));

    // Run a background monitor for all order updates so we can observe fills.
    let monitor_client = client.clone();
    let _monitor = thread::spawn(move || {
        let stream = match monitor_client.order_update_stream() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to open order update stream: {e}");
                return;
            }
        };
        for update in stream.iter_data() {
            match update {
                Ok(OrderUpdate::OrderStatus(s)) => {
                    println!(
                        "status: order {} {} (filled {}/{})",
                        s.order_id,
                        s.status,
                        s.filled,
                        s.filled + s.remaining
                    );
                    if s.status.is_terminal() {
                        break;
                    }
                }
                Ok(other) => println!("{other:?}"),
                Err(e) => {
                    eprintln!("error: {e}");
                    break;
                }
            }
        }
    });
    thread::sleep(Duration::from_millis(100));

    let contract = Contract::call("AAPL").strike(180.0).expires_on(2025, 2, 21).build();
    println!("contract: {contract:?}");

    // Submit two 5-contract market buys via the fluent path. `submit()` allocates the id internally
    // and uses fire-and-forget delivery; status flows through the monitor above.
    for i in 1..=2 {
        match client.order(&contract).buy(5).market().submit() {
            Ok(id) => println!("Submitted order #{i}: {id}"),
            Err(e) => eprintln!("Submit #{i} failed: {e}"),
        }
    }

    // Allow the monitor time to print fills.
    thread::sleep(Duration::from_secs(10));
}
