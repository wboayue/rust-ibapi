use ibapi::contracts::Contract;
use ibapi::Client;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("AAPL");
    let subscription = client.calculate_implied_volatility(&contract, 300.0, 235.0).expect("request failed");
    for calculation in &subscription {
        println!("calculation: {:?}", calculation);
    }
}
