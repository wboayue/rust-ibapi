use anyhow::Ok;
use clap::{arg, Command};

use ibapi::Client;
use ibapi::orders::{self, ExecutionFilter};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("orders")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Find order executions for current daya")
        .arg(arg!(--client_id <CLIENT_ID>).value_parser(clap::value_parser!(i32)))
        .arg(arg!(--account_code <ACCOUNT_CODE>))
        .arg(arg!(--time <TIME>).help("yyyymmdd hh:mm:ss"))
        .arg(arg!(--symbol <SYMBOL>))
        .arg(arg!(--security_type <SECURITY_TYPE>))
        .arg(arg!(--exchange <EXCHANGE>))
        .arg(arg!(--side <SIDE>))
        .arg(arg!(--connection_string <CONNECTION_STRING>).default_value("odin:4002"))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").unwrap();

    let mut filter = ExecutionFilter::default();

    if let Some(client_id) = matches.get_one::<i32>("client_id") {
        filter.client_id = Some(*client_id);
    }

    if let Some(account_code) = matches.get_one::<String>("account_code") {
        filter.account_code = account_code.to_owned();
    }

    if let Some(time) = matches.get_one::<String>("time") {
        filter.time = time.to_owned();
    }

    if let Some(symbol) = matches.get_one::<String>("symbol") {
        filter.symbol = symbol.to_owned();
    }

    if let Some(security_type) = matches.get_one::<String>("security_type") {
        filter.security_type = security_type.to_owned();
    }

    if let Some(exchange) = matches.get_one::<String>("exchange") {
        filter.exchange = exchange.to_owned();
    }

    if let Some(side) = matches.get_one::<String>("side") {
        filter.side = side.to_owned();
    }

    let mut client = Client::connect(connection_string)?;

    let executions = orders::executions(&mut client, filter)?;
    for execution in executions {
        println!("{execution:?}")
    }

    // std::thread::sleep(std::time::Duration::from_secs(5));

    Ok(())
}
