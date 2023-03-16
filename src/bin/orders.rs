use anyhow::Ok;
use clap::builder::PossibleValue;
use clap::{arg, Command};

use ibapi::client::Client;
use ibapi::orders::{self, OrderDataIterator};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let order_types = ["completed", "open", "all_open", "auto_open"];
    let order_types: Vec<PossibleValue> = order_types.iter().map(PossibleValue::new).collect();

    let matches = Command::new("orders")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Queries completed and open orders")
        .arg(arg!(<TYPE>).default_value("completed").value_parser(order_types))
        .arg(arg!(--connection_string <CONNECTION_STRING>).default_value("odin:4002"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").unwrap();
    let order_type = matches.get_one::<String>("TYPE").unwrap();

    let mut client = Client::connect(connection_string)?;

    match order_type.as_str() {
        "open" => {
            println!("Open orders:");
            let orders = orders::open_orders(&mut client)?;
            print_orders(orders);
        }
        "all_open" => {
            println!("All open orders:");
            let orders = orders::all_open_orders(&mut client)?;
            print_orders(orders);
        }
        "auto_open" => {
            println!("Auto open orders:");
            let orders = orders::auto_open_orders(&mut client, false)?;
            print_orders(orders);
        }
        "completed" => {
            println!("Completed orders:");
            let orders = orders::completed_orders(&mut client, false)?;
            print_orders(orders);
        }
        kind => {
            panic!("Unsupported order type: {kind}")
        }
    }

    Ok(())
}

fn print_orders(orders: OrderDataIterator) {
    for order in orders {
        println!("order: {order:?}")
    }
}
