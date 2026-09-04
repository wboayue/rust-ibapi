#![allow(clippy::uninlined_format_args)]
use futures::StreamExt;
use ibapi::contracts::SecurityType;
use ibapi::subscriptions::SubscriptionItemStreamExt;
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    // Request option chain for a stock. The contract id is the underlying's and is
    // required; leaving the exchange unset returns one chain per listing exchange.
    let symbol = "AAPL";
    let contract_id = 265598;

    println!("=== Requesting Option Chain for {symbol} ===");

    let option_chain_stream = client.option_chain(symbol, SecurityType::Stock, contract_id).subscribe().await?;

    let mut chain_count = 0;
    let mut option_chain_stream = option_chain_stream.filter_data();

    while let Some(result) = option_chain_stream.next().await {
        match result {
            Ok(chain) => {
                chain_count += 1;
                println!("\n--- Option Chain {chain_count} ---");
                println!("Exchange: {}", chain.exchange);
                println!("Underlying Contract ID: {}", chain.underlying_contract_id);
                println!("Trading Class: {}", chain.trading_class);
                println!("Multiplier: {}", chain.multiplier);

                if !chain.expirations.is_empty() {
                    println!("Expirations: {}", chain.expirations.join(", "));
                }

                if !chain.strikes.is_empty() {
                    println!("Number of strikes: {}", chain.strikes.len());
                    // Show first few strikes
                    let preview_strikes: Vec<String> = chain.strikes.iter().take(5).map(|s| format!("{s:.2}")).collect();
                    println!("Sample strikes: {}", preview_strikes.join(", "));
                    if chain.strikes.len() > 5 {
                        println!("  ... and {} more", chain.strikes.len() - 5);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error in option chain stream: {e:?}");
                break;
            }
        }
    }

    if chain_count == 0 {
        println!("No option chains received");
    } else {
        println!("\n=== Total option chains received: {chain_count} ===");
    }

    Ok(())
}
