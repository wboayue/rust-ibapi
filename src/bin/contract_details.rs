use clap::{arg, Command};
use log::{debug, info};

use ibapi::client::{Client, IBClient};
use ibapi::contracts::{self, Contract};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("find_contract_details")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Finds contract details")
        .arg(arg!(--connection_string <VALUE>).default_value("odin:4002"))
        .arg(arg!(--stock <VALUE>).required(true))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let stock_symbol = matches.get_one::<String>("stock").expect("stock symbol is required");

    let mut client = IBClient::connect(connection_string)?;

    info!("connected {client:?}");

    println!("server_version: {}", client.server_version());
    println!("server_time: {}", client.server_time());
    println!("managed_accounts: {}", client.managed_accounts());
    println!("next_order_id: {}", client.next_order_id());

    let mut contract = Contract::stock(stock_symbol);
    contract.currency = "USD".to_string();
    debug!("contract template: {contract:?}");

    let results = contracts::contract_details(&mut client, &contract)?;
    for contract in &results {
        println!("contract: {contract:?}");
    }

    Ok(())
}
