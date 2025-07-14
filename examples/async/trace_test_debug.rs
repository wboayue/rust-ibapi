//! Debug async trace and routing

use ibapi::client::Client;
use futures::StreamExt;
use log::debug;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    println!("Testing async message routing...");
    
    // Connect to TWS/Gateway
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to TWS/Gateway");
    
    // Try to get server time manually
    println!("\nSending server time request...");
    
    // Just call server_time and see what happens
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        client.server_time()
    ).await {
        Ok(Ok(time)) => {
            println!("Got server time: {}", time);
        }
        Ok(Err(e)) => {
            println!("Error getting server time: {:?}", e);
        }
        Err(_) => {
            println!("Timeout waiting for server time");
        }
    }
    
    println!("Done");
    Ok(())
}