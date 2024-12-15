use clap::{arg, ArgMatches, Command};
use log::{debug, info};

use ibapi::contracts::Contract;
use ibapi::orders::{self, order_builder, PlaceOrder};
use ibapi::Client;

fn main() {
    env_logger::init();

    let matches = Command::new("place_order")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Submits order to broker")
        .arg(arg!(--connection_string <VALUE>).default_value("127.0.0.1:4002"))
        .arg(arg!(--stock <SYMBOL>).required(true))
        .arg(arg!(--buy <QUANTITY>).value_parser(clap::value_parser!(i32)))
        .arg(arg!(--sell <QUANTITY>).value_parser(clap::value_parser!(i32)))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let stock_symbol = matches.get_one::<String>("stock").expect("stock symbol is required");

    if let Some((action, quantity)) = get_order(&matches) {
        println!("action: {action}, quantity: {quantity}");
    }

    println!("connection_string: {connection_string}, stock_symbol: {stock_symbol}");

    let client = Client::connect(&connection_string, 100).expect("connection failed");

    info!("Connected {client:?}");

    let mut contract = Contract::stock(stock_symbol);
    contract.currency = "USD".to_string();
    debug!("contract template {contract:?}");

    let order_id = client.next_order_id();
    println!("order_id: {order_id}");
    let order = order_builder::market_order(orders::Action::Buy, 100.0);

    println!("contract: {contract:?}, order: {order:?}");

    let subscription = client.place_order(order_id, &contract, &order).expect("could not place order");

    for status in subscription {
        match status {
            PlaceOrder::OrderStatus(order_status) => {
                println!("order status: {order_status:?}")
            }
            PlaceOrder::OpenOrder(open_order) => println!("open order: {open_order:?}"),
            PlaceOrder::ExecutionData(execution) => println!("execution: {execution:?}"),
            PlaceOrder::CommissionReport(report) => println!("commission report: {report:?}"),
            PlaceOrder::Message(message) => println!("notice: {message}"),
        }
    }
}

fn get_order(matches: &ArgMatches) -> Option<(String, i32)> {
    if let Some(quantity) = matches.get_one::<i32>("buy") {
        Some(("BUY".to_string(), *quantity))
    } else {
        matches.get_one::<i32>("sell").map(|quantity| ("SELL".to_string(), *quantity))
    }
}
