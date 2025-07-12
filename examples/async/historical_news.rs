use futures::StreamExt;
use ibapi::Client;
use time::macros::datetime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    println!("=== Requesting Historical News ===");

    // Get contract ID for a stock (you would get this from contract_details)
    // For this example, we'll use a known contract ID
    let contract_id = 265598; // AAPL contract ID (example)

    // Specify provider codes (empty array means all providers)
    let provider_codes = &["BRFG", "DJNL"]; // Example providers

    // Set time range for historical news
    let end_time = time::OffsetDateTime::now_utc();
    let start_time = end_time - time::Duration::days(7); // Last 7 days

    // Maximum number of results
    let total_results = 100;

    let mut news_stream = client
        .historical_news(contract_id, provider_codes, start_time, end_time, total_results)
        .await?;

    println!("Fetching historical news from {} to {}", start_time, end_time);
    println!("Contract ID: {}, Providers: {:?}", contract_id, provider_codes);

    let mut count = 0;
    while let Some(result) = news_stream.next().await {
        match result {
            Ok(article) => {
                count += 1;
                println!("\n--- Article {} ---", count);
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

    if count == 0 {
        println!("No historical news found for the specified criteria.");
    } else {
        println!("\nTotal articles received: {count:?}");
    }

    Ok(())
}
