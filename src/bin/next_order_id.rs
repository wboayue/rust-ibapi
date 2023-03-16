use clap::{arg, Command};

use ibapi::client::Client;
use ibapi::orders;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("next_order_id")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Gets next valid order id")
        .arg(arg!(--connection_string <CONNECTION_STRING>).default_value("odin:4002"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").unwrap();

    let mut client = Client::connect(connection_string)?;

    let order_id = orders::next_valid_order_id(&mut client)?;
    println!("Next valid order id: {order_id}");

    Ok(())
}
