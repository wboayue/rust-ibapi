//! Example of establishing an async connection to TWS/Gateway
//!
//! To run this example:
//! ```bash
//! cargo run --features async --example async_connect
//! ```
//!
//! Make sure TWS or IB Gateway is running with API connections enabled:
//! - For Gateway: Configure -> Settings -> API -> Settings
//! - Enable "Enable ActiveX and Socket Clients"
//! - Add "127.0.0.1" to "Trusted IPs"
//! - Default ports: 4002 (live), 4004 (paper)

use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Attempting to connect to IB Gateway...");

    // Connect to Gateway at the default paper trading port
    match Client::connect("127.0.0.1:4002", 100).await {
        Ok(client) => {
            println!("Connected successfully!");
            println!("Server version: {}", client.server_version());
            println!("Connection time: {:?}", client.connection_time());
            println!("Next order ID: {}", client.next_order_id());

            // Keep the connection alive for a few seconds
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            println!("Disconnecting...");
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            eprintln!("Make sure IB Gateway is running and API connections are enabled.");
            eprintln!("Check that the port (4002 for live, 4004 for paper) is correct.");
        }
    }

    Ok(())
}
