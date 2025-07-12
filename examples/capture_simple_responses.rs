//! Simple example to capture next_valid_order_id response for testing

use ibapi::Client;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Enable message recording
    env::set_var("IBAPI_RECORDING_DIR", "/tmp/simple-responses");
    std::fs::create_dir_all("/tmp/simple-responses")?;

    println!("Connecting to TWS/Gateway...");
    let client = Client::connect("127.0.0.1:4002", 100)?;
    println!("Connected! Server version: {}", client.server_version());

    // Simple request - get next valid order ID
    println!("\nRequesting next valid order ID...");
    let order_id = client.next_valid_order_id()?;
    println!("Next valid order ID: {order_id:?}");

    // Get server time
    println!("\nRequesting server time...");
    let time = client.server_time()?;
    println!("Server time: {time:?}");

    println!("\n\nRaw messages saved to: /tmp/simple-responses/");
    println!("Check the incoming.log and outgoing.log files for exact message formats");

    Ok(())
}
