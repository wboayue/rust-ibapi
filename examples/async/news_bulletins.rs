use futures::StreamExt;
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    println!("=== Subscribing to News Bulletins ===");

    // Subscribe to all news bulletins (true) or just regular bulletins (false)
    let all_messages = true;
    let mut bulletins = client.news_bulletins(all_messages).await?;

    println!("Waiting for news bulletins... (Press Ctrl+C to stop)");

    while let Some(result) = bulletins.next().await {
        match result {
            Ok(bulletin) => {
                println!("\n--- News Bulletin ---");
                println!("ID: {}", bulletin.message_id);
                println!(
                    "Type: {} ({})",
                    bulletin.message_type,
                    match bulletin.message_type {
                        1 => "Regular bulletin",
                        2 => "Exchange unavailable",
                        3 => "Exchange available",
                        _ => "Unknown",
                    }
                );
                println!("Exchange: {}", bulletin.exchange);
                println!("Message: {}", bulletin.message);
            }
            Err(e) => eprintln!("Error: {:?}", e),
        }
    }

    Ok(())
}
