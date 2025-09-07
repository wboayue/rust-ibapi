#![allow(clippy::uninlined_format_args)]
//! Test that subscriptions can be cloned with broadcast channels

use futures::future::join;
use ibapi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to Gateway
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected successfully!");

    // Create a stock contract
    let contract = Contract::stock("AAPL").build();

    // Request market data
    let subscription = client.market_data(&contract, &[], false, false).await?;

    // Clone the subscription
    let subscription_clone = subscription.clone();

    println!("Created original and cloned subscriptions");

    // Process both subscriptions concurrently
    let handle1 = tokio::spawn(async move {
        let mut sub = subscription;
        let mut count = 0;
        while let Some(tick) = sub.next().await {
            if let Ok(tick) = tick {
                println!("[Original] Received tick: {:?}", tick);
                count += 1;
                if count >= 5 {
                    break;
                }
            }
        }
        println!("[Original] Processed {} ticks", count);
    });

    let handle2 = tokio::spawn(async move {
        let mut sub = subscription_clone;
        let mut count = 0;
        while let Some(tick) = sub.next().await {
            if let Ok(tick) = tick {
                println!("[Clone] Received tick: {:?}", tick);
                count += 1;
                if count >= 5 {
                    break;
                }
            }
        }
        println!("[Clone] Processed {} ticks", count);
    });

    // Wait for both to complete
    let _ = join(handle1, handle2).await;

    println!("Test completed successfully!");
    Ok(())
}
