//! Example of establishing a sync connection to TWS/Gateway
//!
//! To run this example:
//! ```bash
//! cargo run --features sync --example connect
//! ```
//!
//! Make sure TWS or IB Gateway is running with API connections enabled:
//! - For Gateway: Configure -> Settings -> API -> Settings
//! - Enable "Enable ActiveX and Socket Clients"
//! - Add "127.0.0.1" to "Trusted IPs"
//! - Default ports: 4001 (live), 4002 (paper)

use ibapi::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Connecting to IB Gateway...");

    // Connect to Gateway at the default paper trading port
    let client = Client::connect("127.0.0.1:4002", 100)?;

    println!("Connected successfully!");
    println!("Server version: {}", client.server_version());
    println!("Connection time: {:?}", client.connection_time());
    println!("Next order ID: {}", client.next_order_id());

    // Get server time to verify connection is working
    match client.server_time() {
        Ok(time) => println!("Server time: {time:?}"),
        Err(e) => eprintln!("Failed to get server time: {e:?}"),
    }

    // Keep the connection alive for a moment
    std::thread::sleep(std::time::Duration::from_secs(1));

    println!("Disconnecting...");
    Ok(())
}
