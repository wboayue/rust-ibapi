//! Contract details stream example: read a broad query row by row and stop early.
//!
//! Dropping the subscription before TWS has sent every row sends the native
//! cancel (server 215+). TWS keeps sending the rest anyway; those rows are
//! discarded.
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example contract_details_stream
//! ```

use ibapi::client::blocking::Client;
use ibapi::prelude::*;

const MAX_ROWS: usize = 10;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100)?;

    // SPY calls for one expiry month two months out: hundreds of rows. Read the first few, then stop.
    let contract = Contract {
        symbol: Symbol::from("SPY"),
        security_type: SecurityType::Option,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        last_trade_date_or_contract_month: contract_month_from_now(2),
        right: Some(OptionRight::Call),
        ..Default::default()
    };

    let request = client.contract_details_stream(&contract);
    println!("request id: {}", request.request_id());

    let subscription = request.subscribe()?;
    let mut rows = 0;
    while let Some(item) = subscription.next() {
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

/// The contract month `months` ahead of the current UTC month, as `YYYYMM`.
fn contract_month_from_now(months: i32) -> String {
    let today = time::OffsetDateTime::now_utc();
    let index = today.year() * 12 + (today.month() as i32 - 1) + months;
    format!("{:04}{:02}", index / 12, index % 12 + 1)
}
