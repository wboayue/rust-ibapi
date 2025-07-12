#![allow(clippy::uninlined_format_args)]
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    println!("=== Requesting News Article ===");

    // You need a valid provider code and article ID
    // These would typically come from historical_news or contract_news
    let provider_code = "BRFG"; // Example provider
    let article_id = "BRFG$12345"; // Example article ID

    println!("Requesting article: {article_id} from provider: {provider_code}");

    match client.news_article(provider_code, article_id).await {
        Ok(article_body) => {
            println!("\n--- Article Content ---");
            match article_body.article_type {
                ibapi::news::ArticleType::Text => {
                    println!("Type: Text/HTML");
                    println!("Content:\n{}", article_body.article_text);
                }
                ibapi::news::ArticleType::Binary => {
                    println!("Type: Binary (Base64 encoded)");
                    println!("Content length: {} bytes", article_body.article_text.len());
                    // In a real application, you would decode the Base64 and save as PDF
                }
            }
        }
        Err(e) => {
            eprintln!("Error fetching article: {e:?}");
            eprintln!("Note: You need a valid article ID from historical_news or contract_news");
        }
    }

    Ok(())
}
