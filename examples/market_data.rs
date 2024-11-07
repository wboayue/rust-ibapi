use ibapi::{contracts::Contract, market_data::realtime::TickTypes, Client};

// This example demonstrates how to request realtime market data for a contract.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("AAPL");

    // https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#available-tick-types
    let generic_ticks = &["233", "293"];
    let snapshot = false;
    let regulatory_snapshot = false;

    let subscription = client
        .market_data(&contract, generic_ticks, snapshot, regulatory_snapshot)
        .expect("error requesting market data");

    for tick in &subscription {
        match tick {
            TickTypes::Price(tick_price) => println!("{:?}", tick_price),
            TickTypes::Size(tick_size) => println!("{:?}", tick_size),
            TickTypes::PriceSize(tick_price_size) => println!("{:?}", tick_price_size),
            TickTypes::Generic(tick_generic) => println!("{:?}", tick_generic),
            TickTypes::String(tick_string) => println!("{:?}", tick_string),
            TickTypes::EFP(tick_efp) => println!("{:?}", tick_efp),
            TickTypes::OptionComputation(option_computation) => println!("{:?}", option_computation),
            TickTypes::RequestParameters(tick_request_parameters) => println!("{:?}", tick_request_parameters),
            TickTypes::SnapshotEnd => subscription.cancel(),
            TickTypes::Notice(notice) => println!("{:?}", notice),
        }
    }
}
