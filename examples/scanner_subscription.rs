use ibapi::{scanner, Client};

// This example demonstrates setting up a market scanner.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let scanner_subscription = scanner::ScannerSubscription {
        number_of_rows: 10,
        instrument: Some("STK".to_string()),
        location_code: Some("STK.US.MAJOR".to_string()),
        scan_code: Some("MOST_ACTIVE".to_string()),
        ..Default::default()
    };

    let subscription = client
        .scanner_subscription(&scanner_subscription, &Vec::default())
        .expect("request scanner parameters failed");
    for scan_results in subscription {
        for scan_data in scan_results.iter() {
            println!(
                "rank: {}, contract_id: {}, symbol: {}",
                scan_data.rank, scan_data.contract_details.contract.contract_id, scan_data.contract_details.contract.symbol
            );
        }
        break;
    }
}
