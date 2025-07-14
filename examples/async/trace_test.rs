//! Test async trace functionality

use ibapi::client::Client;
use ibapi::trace;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    println!("Testing trace functionality in async mode...");

    // Connect to TWS/Gateway
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to TWS/Gateway");

    // Make a simple request to trigger trace recording
    let server_time = client.server_time().await?;
    println!("Server time: {}", server_time);

    // Check if we captured the interaction
    if let Some(interaction) = trace::last_interaction().await {
        println!("\nCaptured interaction:");
        println!("Request: {}", interaction.request);
        println!("Responses: {} response(s)", interaction.responses.len());
        for (i, response) in interaction.responses.iter().enumerate() {
            println!("  Response {}: {} bytes", i + 1, response.len());
            // Print first 100 chars of response
            if response.len() > 100 {
                println!("    {}", &response[..100]);
            } else {
                println!("    {}", response);
            }
        }
    } else {
        println!("No interaction captured (this shouldn't happen with debug logging)");
    }

    Ok(())
}
