//! Simple async trace test without env_logger

use ibapi::client::Client;
use ibapi::trace;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing trace functionality in async mode (simple)...");

    // Connect to TWS/Gateway
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to TWS/Gateway");

    // Enable debug logging programmatically
    log::set_max_level(log::LevelFilter::Debug);

    // Make a simple request to trigger trace recording
    let server_time = client.server_time().await?;
    println!("Server time: {server_time}");

    // Check if we captured the interaction
    if let Some(interaction) = trace::last_interaction().await {
        println!("\nCaptured interaction:");
        println!("Request: {}", interaction.request);
        println!("Responses: {} response(s)", interaction.responses.len());
    } else {
        println!("No interaction captured");
    }

    Ok(())
}
