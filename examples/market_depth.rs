use ibapi::contracts::Contract;
use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("AAPL");

    let subscription = client.market_depth(&contract, 5, true).expect("error requesting market depth");
    for row in &subscription {
        println!("row: {row:?}")
    }
}
