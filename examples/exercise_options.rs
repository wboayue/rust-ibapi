use ibapi::{
    contracts::{Contract, SecurityType},
    orders::ExerciseAction,
    Client,
};

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    // Create option contract
    let contract = Contract {
        symbol: "AAPL".to_owned(),
        security_type: SecurityType::Option,
        exchange: "SMART".to_owned(),
        currency: "USD".to_owned(),
        last_trade_date_or_contract_month: "20250221".to_owned(),
        strike: 180.0,
        right: "C".to_owned(),
        multiplier: "100".to_owned(),
        ..Default::default()
    };

    let account = "DU1234567";
    let manual_order_time = None;

    let subscription = client
        .exercise_options(&contract, ExerciseAction::Exercise, 100, account, true, manual_order_time)
        .expect("exercise options request failed!");
    for status in &subscription {
        println!("{status:?}")
    }
}
