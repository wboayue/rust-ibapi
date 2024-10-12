use ibapi::contracts::Contract;
use ibapi::orders::{order_builder, Action, OrderNotification};
use ibapi::Client;

pub fn main() {
    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL");

    // Creates a market order to purchase 100 shares
    let order_id = client.next_order_id();
    let order = order_builder::market_order(Action::Buy, 100.0);

    let subscription = client.place_order(order_id, &contract, &order).expect("place order request failed!");

    for notice in subscription {
        if let OrderNotification::ExecutionData(data) = notice {
            println!("{} {} shares of {}", data.execution.side, data.execution.shares, data.contract.symbol);
        } else {
            println!("{:?}", notice);
        }
    }
}