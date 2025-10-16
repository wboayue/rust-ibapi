//! Example demonstrating connection monitoring with Client.is_connected()
//!
//! This example shows how to monitor the connection status to TWS/IB Gateway
//! and respond to connection changes.

use ibapi::client::blocking::Client;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to IB Gateway (or TWS)
    let address = "127.0.0.1:4002"; // IB Gateway paper trading port
    let client_id = 100;

    println!("Connecting to {} with client ID {}...", address, client_id);
    let client = Client::connect(address, client_id)?;

    println!("Successfully connected to TWS/Gateway");
    println!("Server version: {}", client.server_version());

    // Initial connection check
    if client.is_connected() {
        println!("✓ Client is connected");
    } else {
        println!("✗ Client is not connected");
    }

    // Monitor connection status periodically
    println!("\nMonitoring connection status (press Ctrl+C to exit)...");

    let mut connected_previously = true;
    loop {
        let is_connected = client.is_connected();

        // Log state changes
        if is_connected != connected_previously {
            if is_connected {
                println!("✓ Connection restored");
            } else {
                println!("✗ Connection lost");
            }
            connected_previously = is_connected;
        }

        // Perform operations only when connected
        if is_connected {
            // Example: Request server time periodically
            match client.server_time() {
                Ok(time) => {
                    println!("Server time: {}", time);
                }
                Err(e) => {
                    println!("Error getting server time: {}", e);
                }
            }
        } else {
            println!("Waiting for connection to be restored...");
        }

        // Wait before next check
        thread::sleep(Duration::from_secs(5));
    }
}
