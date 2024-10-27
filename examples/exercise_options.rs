use ibapi::{
    contracts::{Contract, SecurityType},
    orders::{self, order_builder, ExerciseAction},
    Client,
};

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = create_option_contract("AAPL", 180.0, "C", "20250221");

    purchase_option(&client, &contract);

    let accounts = client.managed_accounts().expect("could not get managed accounts");
    let account = &accounts[0];
    let manual_order_time = None;

    let subscription = client
        .exercise_options(&contract, ExerciseAction::Exercise, 100, account, true, manual_order_time)
        .expect("exercise options request failed!");

    for status in &subscription {
        println!("{status:?}")
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

fn purchase_option(client: &Client, contract: &Contract) {
    let order_id = client.next_order_id();
    println!("next order id: {order_id}");

    let order = order_builder::market_order(orders::Action::Buy, 5.0);
    println!("contract: {contract:?}, order: {order:?}");

    let subscription = client.place_order(order_id, &contract, &order).expect("could not place order");
    for status in subscription {
        println!("{status:?}")
    }
}
