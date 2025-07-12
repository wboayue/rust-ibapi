#![allow(clippy::uninlined_format_args)]
use ibapi::orders::TagValue;
use ibapi::scanner::ScannerSubscription;
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    println!("=== Complex Scanner Subscription ===");

    // Create scanner subscription for high option volume stocks
    let subscription = ScannerSubscription {
        number_of_rows: 25,
        instrument: Some("STK".to_string()),
        location_code: Some("STK.US".to_string()),
        scan_code: Some("OPT_VOLUME_MOST_ACTIVE".to_string()),
        above_price: Some(10.0),
        below_price: Some(500.0),
        above_volume: Some(500000),
        average_option_volume_above: Some(10000),
        market_cap_above: Some(1_000_000_000.0),     // $1B market cap
        stock_type_filter: Some("CORP".to_string()), // Corporations only
        ..Default::default()
    };

    println!("Scanning for stocks with high option volume...");
    println!("Criteria:");
    println!("  - US stocks");
    println!("  - Price between $10 and $500");
    println!("  - Stock volume above 500,000");
    println!("  - Option volume above 10,000");
    println!("  - Market cap above $1B");
    println!("  - Corporations only");
    println!("  - Top 25 results");

    // Add additional filters using TagValue
    let filters = vec![TagValue {
        tag: "changePercAbove".to_string(),
        value: "2".to_string(), // At least 2% change
    }];

    let mut scanner_results = client.scanner_subscription(&subscription, &filters).await?;

    println!("\nScanning market... (Press Ctrl+C to stop)");

    let mut batch_count = 0;
    while let Some(result) = scanner_results.next().await {
        match result {
            Ok(scanner_data_list) => {
                batch_count += 1;
                println!("\n=== Scanner Results Batch {} ===", batch_count);

                for data in scanner_data_list {
                    println!("\nRank #{}", data.rank);
                    println!(
                        "Symbol: {} ({})",
                        data.contract_details.contract.symbol, data.contract_details.contract.contract_id
                    );
                    println!("Name: {}", data.contract_details.long_name);
                    println!("Exchange: {}", data.contract_details.contract.exchange);
                    println!("Industry: {} - {}", data.contract_details.industry, data.contract_details.category);

                    if data.contract_details.ev_multiplier > 0.0 {
                        println!("EV Multiplier: {}", data.contract_details.ev_multiplier);
                    }
                }

                // Scanner typically sends one batch then completes
                println!("\nScanner batch complete. Waiting for updates...");
            }
            Err(e) => {
                eprintln!("Error: {e:?}");
                break;
            }
        }
    }

    Ok(())
}
