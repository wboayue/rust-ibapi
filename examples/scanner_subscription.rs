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

    let subscription = client.scanner_subscription(&scanner_subscription, &Vec::default()).expect("request scanner parameters failed");
    for contract in subscription {
        match contract {
            scanner::Scanner::Data(data) => {
                println!("{:?}", data);
            }
            scanner::Scanner::End => {
                println!("End of scanner data");
                break;
            }
        }
    }
}
