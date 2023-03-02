use clap::{arg, Command};

use ibapi::client::IBClient;
use ibapi::orders;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("cancel_order")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Cancels an open order")
        .arg(arg!(<ORDER_ID>).value_parser(clap::value_parser!(i32)))
        .arg(arg!(--connection_string <CONNECTION_STRING>).default_value("odin:4002"))
        .arg(arg!(--manual_order_cancel_time <CANCEL_TIME>).default_value(""))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").unwrap();
    let order_id = matches.get_one::<i32>("ORDER_ID").unwrap();
    let manual_order_cancel_time = matches.get_one::<String>("manual_order_cancel_time").unwrap();

    let mut client = IBClient::connect(connection_string)?;

    let results = orders::cancel_order(&mut client, *order_id, manual_order_cancel_time)?;
    for result in results {
        println!("{result:?}");
    }

    Ok(())
}
