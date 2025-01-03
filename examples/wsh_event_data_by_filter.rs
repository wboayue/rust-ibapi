use ibapi::Client;

// This example demonstrates requesting Wall Street Horizon event data by filter.
// This featured does not appear to be released yet.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let filter = r#"
    {
        "country": "All",
        "watchlist": ["8314"],
        "limit_region": 10,
        "limit": 10,
        "wshe_ed": "true",
        "wshe_bod": "true"
    }    
    "#;

    let filter = filter.replace("\n", "").to_string();

    let subscription = client
        .wsh_event_data_by_filter(&filter, None, None)
        .expect("request wsh event data failed");

    for event_data in &subscription {
        println!("{:?}", event_data);
    }

    // check for errors
    if let Some(error) = subscription.error() {
        println!("error: {}", error);
    }
}
