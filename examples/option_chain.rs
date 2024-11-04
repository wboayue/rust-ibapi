use ibapi::{
    contracts::{Contract, SecurityType},
    Client,
};

// This example demonstrates requesting option chain data from the TWS.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let symbol = "AAPL";
    let exchange = ""; // all exchanges
    let security_type = SecurityType::Stock;
    let contract_id = 265598;

    let subscription = client
        .option_chain(symbol, exchange, security_type, contract_id)
        .expect("request option chain failed!");

    for option_chain in &subscription {
        println!("{option_chain:?}")
    }
}
