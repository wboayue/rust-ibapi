#![allow(clippy::uninlined_format_args)]
use ibapi::Client;

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

    // Headlines from the last 7 days, newest first, at most 100
    let since = time::OffsetDateTime::now_utc() - time::Duration::days(7);

    let page = client.historical_news(contract_id, provider_codes).since(since).limit(100).page().await?;

    println!("Fetching historical news since {since}");
    println!("Contract ID: {contract_id}, Providers: {provider_codes:?}");

    let mut count = 0;
    for article in &page.articles {
        count += 1;
        println!("\n--- Article {count} ---");
        match article.time {
            Some(time) => println!("Time: {time}"),
            None => println!("Time: unknown"),
        }
        println!("Provider: {}", article.provider_code);
        println!("Article ID: {}", article.article_id);
        println!("Headline: {}", article.headline);
        if !article.extra_data.is_empty() {
            println!("Extra data: {}", article.extra_data);
        }
    }
    if page.has_more {
        println!("\nMore headlines are available than the 100 returned.");
    }

    if count == 0 {
        println!("No historical news found for the specified criteria.");
    } else {
        println!("\nTotal articles received: {count:?}");
    }

    Ok(())
}
