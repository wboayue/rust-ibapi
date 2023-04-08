use anyhow::Ok;
use clap::builder::PossibleValue;
use clap::{arg, Command};

use ibapi::orders::OrderDataResult;
use ibapi::Client;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let order_types = ["completed", "open", "all_open", "auto_open"];
    let order_types: Vec<PossibleValue> = order_types.iter().map(PossibleValue::new).collect();

    let matches = Command::new("orders")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Queries completed and open orders")
        .arg(arg!(<TYPE>).default_value("completed").value_parser(order_types))
        .arg(arg!(--connection_string <CONNECTION_STRING>).default_value("127.0.0.1:4002"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").unwrap();
    let order_type = matches.get_one::<String>("TYPE").unwrap();

    let client = Client::connect(connection_string, 100)?;

    match order_type.as_str() {
        "open" => {
            println!("Open orders:");
            let orders = client.open_orders()?;
            print_orders(orders);
        }
        "all_open" => {
            println!("All open orders:");
            let orders = client.all_open_orders()?;
            print_orders(orders);
        }
        "auto_open" => {
            println!("Auto open orders:");
            let orders = client.auto_open_orders(false)?;
            print_orders(orders);
        }
        "completed" => {
            println!("Completed orders:");
            let orders = client.completed_orders(false)?;
            print_orders(orders);
        }
        kind => {
            panic!("Unsupported order type: {kind}")
        }
    }

    Ok(())
}

fn print_orders(orders: impl Iterator<Item = OrderDataResult>) {
    for order in orders {
        println!("order: {order:?}")
    }
}
