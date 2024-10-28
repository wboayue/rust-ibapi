use ibapi::{
    contracts::{Contract, SecurityType},
    orders::{self, order_builder, PlaceOrder},
    Client,
};

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = create_option_contract("AAPL", 180.0, "C", "20250221");

    let order_id = client.next_valid_order_id().expect("could not get next valid order id");
    //    let order_id = client.next_order_id();
    println!("next order id: {order_id}");

    let order = order_builder::market_order(orders::Action::Buy, 5.0);
    println!("contract: {contract:?}, order: {order:?}");

    let subscription = client.place_order(order_id, &contract, &order).expect("could not place order");
    for status in subscription {
        println!("{status:?}")
    }
    let order_id = client.next_order_id();
    println!("next order id: {order_id}");

    let order = order_builder::market_order(orders::Action::Buy, 5.0);
    println!("contract: {contract:?}, order: {order:?}");

    let subscription = client.place_order(order_id, &contract, &order).expect("could not place order");
    for status in subscription {
        println!("{status:?}");
        if let PlaceOrder::OrderStatus(order_status) = status {
            if order_status.remaining == 0.0 {
                break;
            }
        }
    }
}

fn create_option_contract(symbol: &str, strike: f64, right: &str, last_trade_date_or_contract_month: &str) -> Contract {
    Contract {
        symbol: symbol.to_owned(),
        security_type: SecurityType::Option,
        exchange: "SMART".to_owned(),
        currency: "USD".to_owned(),
        last_trade_date_or_contract_month: last_trade_date_or_contract_month.to_owned(),
        strike,
        right: right.to_owned(),
        multiplier: "100".to_owned(),
        ..Default::default()
    }
}
