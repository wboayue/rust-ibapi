use ibapi::{scanner, Client};

// This example demonstrates setting up a market scanner.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let scanner_subscription = scanner::ScannerSubscription {
        number_of_rows: 10,
        instrument: None,
        location_code: Some("STK.US.MAJOR".to_string()),
        scan_code: Some("TOP_PERC_GAIN".to_string()),
        above_price: 0.0,
        below_price: 100.0,
        above_volume: 1000000,
        average_option_volume_above: 0,
        market_cap_above: f64::MAX,
        market_cap_below: f64::MAX,
        moody_rating_above: None,
        moody_rating_below: None,
        sp_rating_above: None,
        sp_rating_below: None,
        maturity_date_above: None,
        maturity_date_below: None,
        coupon_rate_above: f64::MAX,
        coupon_rate_below: f64::MAX,
        exclude_convertible: false,
        scanner_setting_pairs: None,
        stock_type_filter: None,
    };

    let subscription = client.scanner_subscription(scanner_subscription, Vec::default()).expect("request scanner parameters failed");
    for scanner_data in subscription {
        match scanner_data {
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
