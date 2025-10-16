//! Cancel Orders example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example cancel_orders
//! ```

use clap::{arg, Command};

use ibapi::client::blocking::Client;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("cancel_order")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Cancels an open order")
        .arg(arg!(--order_id <ORDER_ID>).value_parser(clap::value_parser!(i32)).default_value("-1"))
        .arg(arg!(--connection_string <CONNECTION_STRING>).default_value("127.0.0.1:4002"))
        .arg(arg!(--manual_order_cancel_time <CANCEL_TIME>).default_value(""))
        .arg(arg!(--global).default_value("false"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").unwrap();
    let order_id = matches.get_one::<i32>("order_id").unwrap();
    let manual_order_cancel_time = matches.get_one::<String>("manual_order_cancel_time").unwrap();
    let global = matches.get_one::<bool>("global").unwrap();

    let client = Client::connect(connection_string, 100)?;

    if *global {
        println!("Requesting global cancel.");

        client.global_cancel()?
    } else {
        println!("Cancelling order {order_id}");

        let results = client.cancel_order(*order_id, manual_order_cancel_time)?;
        for result in results {
            println!("{result:?}");
        }
    };

    Ok(())
}
