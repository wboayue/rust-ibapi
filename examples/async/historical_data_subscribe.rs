//! Historical bars from a caller-owned request (async).
//!
//! `historical_data(..).subscribe()` sends the request once, under an id known
//! before anything is written, and hands back its frames: the bars, then the
//! window. The caller bounds the wait, and on timeout cancels and learns from
//! `cancel_and_drain` whether TWS is done with the request.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example async_historical_data_subscribe
//! cargo run --example async_historical_data_subscribe -- --symbol MSFT --days 10 --timeout 5
//! ```

use clap::Parser;
use ibapi::market_data::historical::HistoricalDataFrame;
use ibapi::prelude::*;
use time::OffsetDateTime;
use tokio::time::{timeout_at, Duration, Instant};

#[derive(Parser)]
#[command(about = "Fetch hourly bars for a date range through a caller-owned subscription")]
struct Args {
    /// Stock symbol
    #[arg(long, default_value = "AAPL")]
    symbol: String,

    /// Days back from now
    #[arg(long, default_value_t = 5)]
    days: i64,

    /// Seconds to wait for the result before cancelling
    #[arg(long, default_value_t = 30)]
    timeout: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    let client = Client::connect("127.0.0.1:4002", 100).await?;
    let contract = Contract::stock(&args.symbol).build();
    let end = OffsetDateTime::now_utc();
    let start = end - time::Duration::days(args.days);

    let request = client.historical_data(&contract, HistoricalBarSize::Hour).between(start, end);
    println!("request id {} for {} from {start} to {end}", request.request_id(), args.symbol);
    let mut subscription = request.subscribe().await?;

    // `next()` is cancel-safe, so a timed-out wait loses nothing already read.
    let deadline = Instant::now() + Duration::from_secs(args.timeout);
    let mut bars = None;
    loop {
        match timeout_at(deadline, subscription.next()).await {
            Ok(Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::Bars(data))))) => {
                println!("received {} bars", data.bars.len());
                bars = Some(data);
            }
            Ok(Some(Ok(SubscriptionItem::Data(HistoricalDataFrame::End {
                start: served_start,
                end: served_end,
            })))) => {
                println!("TWS served {served_start} to {served_end}");
                let Some(data) = bars.take() else {
                    return Err("window arrived before the bars".into());
                };
                let data = data.trim_to(start, end);
                println!("{} bars in the requested range", data.bars.len());
                for bar in data.bars.iter().take(3) {
                    println!("  {}: O {:.2} H {:.2} L {:.2} C {:.2}", bar.date, bar.open, bar.high, bar.low, bar.close);
                }
                break;
            }
            Ok(Some(Ok(SubscriptionItem::Notice(notice)))) => println!("notice: {notice}"),
            Ok(Some(Err(e))) => return Err(e.into()),
            Ok(None) => return Err("stream closed before the window".into()),
            Err(_elapsed) => {
                println!("no result after {}s; cancelling", args.timeout);
                let outcome = subscription.cancel_and_drain(Instant::now() + Duration::from_secs(5)).await?;
                println!("cancel outcome: {outcome:?}");
                break;
            }
        }
    }

    Ok(())
}
