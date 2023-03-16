use clap::{arg, ArgMatches, Command};
use log::{debug, info};

use ibapi::client::Client;
use ibapi::contracts::Contract;
use ibapi::orders::{self, order_builder, OrderNotification};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("place_order")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Submits order to broker")
        .arg(arg!(--connection_string <VALUE>).default_value("odin:4002"))
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

    let mut client = Client::connect("odin:4002")?;

    info!("Connected {client:?}");

    let mut contract = Contract::stock(stock_symbol);
    contract.currency = "USD".to_string();
    debug!("contract template {contract:?}");

    let order_id = client.next_order_id();
    println!("order_id: {order_id}");
    let order = order_builder::market_order(orders::Action::Buy, 100.0);

    println!("contract: {contract:?}, order: {order:?}");

    let results = orders::place_order(&mut client, order_id, &contract, &order)?;

    for status in results {
        match status {
            OrderNotification::OrderStatus(order_status) => {
                println!("order status: {order_status:?}")
            }
            OrderNotification::OpenOrder(open_order) => println!("open order: {open_order:?}"),
            OrderNotification::ExecutionData(execution) => println!("execution: {execution:?}"),
            OrderNotification::CommissionReport(report) => println!("commision report: {report:?}"),
            OrderNotification::Message(message) => println!("notice: {message}"),
        }
    }

    Ok(())
}

fn get_order(matches: &ArgMatches) -> Option<(String, i32)> {
    if let Some(quantity) = matches.get_one::<i32>("buy") {
        Some(("BUY".to_string(), *quantity))
    } else {
        matches.get_one::<i32>("sell").map(|quantity| ("SELL".to_string(), *quantity))
    }
}
