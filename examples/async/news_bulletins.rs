#![allow(clippy::uninlined_format_args)]
use futures::StreamExt;
use ibapi::news::AllMessages;
use ibapi::subscriptions::SubscriptionItemStreamExt;
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    println!("=== Subscribing to News Bulletins ===");

    // `Yes` also delivers the current day's existing bulletins; `No` only new ones
    let bulletins = client.news_bulletins(AllMessages::Yes).await?;

    println!("Waiting for news bulletins... (Press Ctrl+C to stop)");

    let mut bulletins = bulletins.filter_data();
    while let Some(result) = bulletins.next().await {
        match result {
            Ok(bulletin) => {
                println!("\n--- News Bulletin ---");
                println!("ID: {}", bulletin.message_id);
                println!("Type: {:?}", bulletin.message_type);
                println!("Exchange: {}", bulletin.exchange);
                println!("Message: {}", bulletin.message);
            }
            Err(e) => eprintln!("Error: {e:?}"),
        }
    }

    Ok(())
}
