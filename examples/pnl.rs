use clap::{arg, Command};
use ibapi::Client;

fn main() {
    env_logger::init();

    let matches = Command::new("pnl")
        .about("Gets realtime profit and loss updates")
        .arg(arg!(--connection_url <VALUE>).default_value("127.0.0.1:4002"))
        .arg(arg!(--account <ACCOUNT>).required(true))
        .get_matches();

    let gateway_url = matches.get_one::<String>("connection_url").expect("connection_string is required");
    let account = matches.get_one::<String>("account").expect("account is required");

    let client = Client::connect(&gateway_url, 919).expect("connection failed");

    let mut subscription = client.pnl(&account, None).expect("pnl request failed");

    // Get next item non-blocking
    if let Some(pnl) = subscription.try_next() {
        println!("non-blocking PnL: {:?}", pnl);
    }

    // Consume items blocking for next
    while let Some(pnl) = subscription.next() {
        println!("PnL: {:?}", pnl);
        subscription.cancel();
    }
}
