//! Example of establishing a sync connection to TWS/Gateway
//!
//! To run this example:
//! ```bash
//! cargo run --example sync_connect
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

    println!("Attempting to connect to IB Gateway...");

    // Connect to Gateway at the default paper trading port
    match Client::connect("127.0.0.1:4002", 100) {
        Ok(client) => {
            println!("Connected successfully!");
            println!("Server version: {}", client.server_version());
            println!("Connection time: {:?}", client.connection_time());
            println!("Next order ID: {}", client.next_order_id());

            // Keep the connection alive for a few seconds
            std::thread::sleep(std::time::Duration::from_secs(5));

            println!("Disconnecting...");
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            eprintln!("Make sure IB Gateway is running and API connections are enabled.");
            eprintln!("Check that the port (4002 for paper, 4001 for live) is correct.");
            
            // Try alternative port
            println!("\nTrying alternative port 4001 (live trading)...");
            match Client::connect("127.0.0.1:4001", 100) {
                Ok(client) => {
                    println!("Connected successfully to port 4001!");
                    println!("Server version: {}", client.server_version());
                }
                Err(e2) => {
                    eprintln!("Failed to connect to port 4001: {}", e2);
                    eprintln!("Both ports failed. IB Gateway/TWS may not be running.");
                }
            }
        }
    }

    Ok(())
}