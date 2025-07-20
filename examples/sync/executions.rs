//! Executions example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example executions
//! ```

use ibapi::orders::ExecutionFilter;
use ibapi::Client;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let filter = ExecutionFilter {
        client_id: Some(32),
        ..Default::default()
    };
    // filter.account_code = account_code.to_owned();
    // filter.time = time.to_owned();
    // filter.symbol = symbol.to_owned();
    // filter.security_type = security_type.to_owned();
    // filter.exchange = exchange.to_owned();
    // filter.side = side.to_owned();

    let client = Client::connect("127.0.0.1:4002", 100)?;

    let subscription = client.executions(filter)?;
    for execution in &subscription {
        println!("{execution:?}")
    }

    Ok(())
}
