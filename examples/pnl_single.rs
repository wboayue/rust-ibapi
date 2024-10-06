use clap::{arg, Command};
use ibapi::Client;
use time::format_description::parse;

fn main() {
    env_logger::init();

    let matches = Command::new("pnl")
        .about("Gets realtime profit and loss updates of single contract")
        .arg(arg!(--connection_url <VALUE>).default_value("127.0.0.1:4002"))
        .arg(arg!(--account <ACCOUNT>).required(true))
        .arg(arg!(--contract_id <CONTRACT>).required(true))
        .get_matches();

    let gateway_url = matches.get_one::<String>("connection_url").expect("connection_string is required");
    let account = matches.get_one::<String>("account").expect("account is required");
    let contract_id = matches.get_one::<String>("contract_id").expect("contract_id is required");
    let contract_id = contract_id.parse::<i32>().expect("invalid number");

    let client = Client::connect(&gateway_url, 919).expect("connection failed");

    let mut subscription = client.pnl_single(&account, contract_id, None).expect("pnl single request failed");

    // Get next item non-blocking
    if let Some(pnl) = subscription.try_next() {
        println!("non-blocking PnL: {:?}", pnl);
    }

    // Consume items blocking for next
    while let Some(pnl) = subscription.next() {
        println!("PnL: {:?}", pnl);

        // After processing items subscription could be cancelled.
        subscription.cancel();
    }
}
