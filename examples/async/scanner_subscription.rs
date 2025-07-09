use futures::StreamExt;
use ibapi::scanner::ScannerSubscription;
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    println!("=== Market Scanner Subscription ===");

    // Create scanner subscription for top percentage gainers
    let subscription = ScannerSubscription {
        number_of_rows: 10,
        instrument: Some("STK".to_string()),
        location_code: Some("STK.US.MAJOR".to_string()),
        scan_code: Some("TOP_PERC_GAIN".to_string()),
        above_price: Some(5.0),
        below_price: Some(1000.0),
        above_volume: Some(100000),
        ..Default::default()
    };

    println!("Scanning for top percentage gainers...");
    println!("Criteria:");
    println!("  - US Major stocks");
    println!("  - Price between $5 and $1000");
    println!("  - Volume above 100,000");
    println!("  - Top 10 results");

    // No additional filters for this example
    let filters = vec![];

    let mut scanner_results = client.scanner_subscription(&subscription, &filters).await?;

    println!("\nScanning market... (Press Ctrl+C to stop)");

    while let Some(result) = scanner_results.next().await {
        match result {
            Ok(scanner_data_list) => {
                println!("\n=== Scanner Results ===");
                for data in scanner_data_list {
                    println!("\nRank #{}", data.rank);
                    println!("Symbol: {}", data.contract_details.contract.symbol);
                    println!("Exchange: {}", data.contract_details.contract.exchange);
                    println!("Currency: {}", data.contract_details.contract.currency);
                    println!("Long Name: {}", data.contract_details.long_name);

                    if !data.leg.is_empty() {
                        println!("Leg: {}", data.leg);
                    }
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
