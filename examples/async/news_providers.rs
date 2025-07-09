use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    println!("=== Requesting News Providers ===");
    
    let providers = client.news_providers().await?;
    
    if providers.is_empty() {
        println!("No news providers available. You may need to subscribe to news services.");
    } else {
        println!("Available news providers:");
        for provider in providers {
            println!("  Code: {} - Name: {}", provider.code, provider.name);
        }
    }

    Ok(())
}