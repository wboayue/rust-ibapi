//! Stream Retry example
//!
//! Streams realtime bars across reconnects with `Client::resubscribing`.
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example stream_retry
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::TradingHours;
use ibapi::subscriptions::SubscriptionItem;
use ibapi::TRANSPORT_RECONNECT_CODE;

fn main() {
    env_logger::init();

    let connection_url = "127.0.0.1:4002";
    let client = Client::connect(connection_url, 100).expect("connection to TWS failed!");

    let contract = Contract::stock("AAPL").build();

    // Resubscribes after each reconnect; ends on any other error, or once the
    // client gave up reconnecting (ConnectionFailed) or was shut down (Shutdown).
    let bars = client.resubscribing(|| client.realtime_bars(&contract).trading_hours(TradingHours::Extended).subscribe());

    for item in bars {
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
