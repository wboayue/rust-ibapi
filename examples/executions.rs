use ibapi::orders::ExecutionFilter;
use ibapi::Client;

fn main() -> anyhow::Result<()> {
    let mut filter = ExecutionFilter::default();

    filter.client_id = Some(32);
    // filter.account_code = account_code.to_owned();
    // filter.time = time.to_owned();
    // filter.symbol = symbol.to_owned();
    // filter.security_type = security_type.to_owned();
    // filter.exchange = exchange.to_owned();
    // filter.side = side.to_owned();

    let client = Client::connect("localhost:4002")?;

    let executions = client.executions(filter)?;
    for execution in executions {
        println!("{execution:?}")
    }

    Ok(())
}
