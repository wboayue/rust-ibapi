use futures::StreamExt;
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    println!("=== Subscribing to Broad Tape News ===");
    
    // Subscribe to a specific news provider's broad tape
    let provider_code = "BRFG"; // Briefing.com example
    
    println!("Subscribing to broad tape news from provider: {}", provider_code);
    
    let mut news_stream = client.broad_tape_news(provider_code).await?;
    
    println!("Waiting for broad tape news... (Press Ctrl+C to stop)");
    println!("Note: This will show all news from the provider, not limited to specific contracts");
    
    while let Some(result) = news_stream.next().await {
        match result {
            Ok(article) => {
                println!("\n--- Broad Tape News ---");
                println!("Time: {}", article.time);
                println!("Provider: {}", article.provider_code);
                println!("Article ID: {}", article.article_id);
                println!("Headline: {}", article.headline);
                if !article.extra_data.is_empty() {
                    println!("Extra data: {}", article.extra_data);
                }
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}