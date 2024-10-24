use ibapi::{contracts::Contract, market_data::realtime::TickTypes, Client};

// This example demonstrates how to request realtime market data for a contract.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("AAPL");
    let generic_ticks = &[];
    let snapshot = false;
    let regulatory_snapshot = false;

    let subscription = client
        .market_data(&contract, generic_ticks, snapshot, regulatory_snapshot)
        .expect("error requesting market data");

    for tick in &subscription {
        println!("{tick:?}");

        if let TickTypes::SnapshotEnd = tick {
            subscription.cancel();
        }
    }
}
