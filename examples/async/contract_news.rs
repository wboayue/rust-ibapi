use futures::StreamExt;
use ibapi::contracts::Contract;
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    println!("=== Subscribing to Contract News ===");

    // Create a contract to monitor news for
    let contract = Contract::stock("AAPL");

    // Specify news providers to subscribe to (empty means all)
    let provider_codes = &["BRFG", "DJNL"];

    println!("Subscribing to news for {} from providers: {:?}", contract.symbol, provider_codes);

    let mut news_stream = client.contract_news(&contract, provider_codes).await?;

    println!("Waiting for news... (Press Ctrl+C to stop)");

    while let Some(result) = news_stream.next().await {
        match result {
            Ok(article) => {
                println!("\n--- Breaking News ---");
                println!("Time: {}", article.time);
                println!("Provider: {}", article.provider_code);
                println!("Article ID: {}", article.article_id);
                println!("Headline: {}", article.headline);
                if !article.extra_data.is_empty() {
                    println!("Extra data: {}", article.extra_data);
                }
            }
            Err(e) => {
                eprintln!("Error: {e:?}");
                break;
            }
        }
    }

    Ok(())
}
