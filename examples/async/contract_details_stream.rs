//! Contract details stream example: read a broad query row by row and stop early.
//!
//! Dropping the subscription before TWS has sent every row sends the native
//! cancel (server 215+). TWS may still send the rest of a result it has
//! already prepared; those rows are discarded.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features async --example async_contract_details_stream
//! ```

use ibapi::prelude::*;

const MAX_ROWS: usize = 10;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).await?;

    // SPY calls for one expiry month: a few hundred rows. Read the first few, then stop.
    let contract = Contract {
        symbol: Symbol::from("SPY"),
        security_type: SecurityType::Option,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        last_trade_date_or_contract_month: "202611".into(),
        right: Some(OptionRight::Call),
        ..Default::default()
    };

    let request = client.contract_details_stream(&contract);
    println!("request id: {}", request.request_id());

    let mut subscription = request.subscribe().await?;
    let mut rows = 0;
    while let Some(item) = subscription.next().await {
        match item? {
            SubscriptionItem::Data(details) => {
                rows += 1;
                println!("{rows:>3}: {} {}", details.contract.local_symbol, details.contract.exchange);
                if rows == MAX_ROWS {
                    println!("stopping after {MAX_ROWS} rows; dropping the subscription sends the native cancel");
                    break;
                }
            }
            SubscriptionItem::Notice(notice) => println!("notice: {notice}"),
        }
    }

    Ok(())
}
