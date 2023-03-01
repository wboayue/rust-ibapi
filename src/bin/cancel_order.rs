use std::thread;
use std::time::Duration;

use clap::{arg, ArgMatches, Command};
use log::{debug, info};

use ibapi::client::{Client, IBClient};
use ibapi::orders;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // let matches = Command::new("cancel_order")
    //     .version("1.0")
    //     .author("Wil Boayue <wil.boayue@gmail.com>")
    //     .about("Cancels an open order")
    //     .arg(arg!(--connection_string <VALUE>).default_value("odin:4002"))
    //     .arg(arg!(--order_id).value_parser(clap::value_parser!(i32)))
    //     .arg(arg!(--manual_order_cancel_time).default_value(""))
    //     .get_matches();

    // let connection_string = matches.get_one::<String>("connection_string").unwrap();
    // let order_id = matches.get_one::<i32>("order_id").unwrap();
    // let manual_order_cancel_time = matches.get_one::<String>("manual_order_cancel_time").unwrap();

    let connection_string = "odin:4002";
    let order_id = 40;
    let manual_order_cancel_time = "";

    let mut client = IBClient::connect(&connection_string)?;

    let results = orders::cancel_order(&mut client, order_id, &manual_order_cancel_time)?;
    for result in results {
        println!("{result:?}");
    }

    Ok(())
}
