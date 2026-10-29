//! Async stream retry example
//!
//! Streams realtime bars across reconnects with `Client::resubscribing`.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example async_stream_retry
//! ```

use futures::StreamExt;
use ibapi::market_data::TradingHours;
use ibapi::prelude::*;
use ibapi::TRANSPORT_RECONNECT_CODE;

#[tokio::main]
async fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL").build();

    // Resubscribes after each reconnect; ends on any other error, or once the
    // client gave up reconnecting (ConnectionFailed) or was shut down (Shutdown).
    let bars = client.resubscribing(|| client.realtime_bars(&contract).trading_hours(TradingHours::Extended).subscribe());
    let mut bars = std::pin::pin!(bars);

    while let Some(item) = bars.next().await {
        match item {
            Ok(SubscriptionItem::Data(bar)) => println!("bar: {bar:?}"),
            Ok(SubscriptionItem::Notice(notice)) if notice.code == TRANSPORT_RECONNECT_CODE => {
                eprintln!("Resubscribed after a reconnect; bars from the outage are missing.")
            }
            Ok(SubscriptionItem::Notice(notice)) => eprintln!("notice: {notice}"),
            Err(e) => eprintln!("stream ended: {e}"),
        }
    }
}
