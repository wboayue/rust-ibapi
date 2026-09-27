//! Preset attached orders example
//!
//! Places a limit order and asks TWS to attach a stop-loss and a profit-taker priced from
//! its order presets (TWS Global Configuration → Presets). Unlike `bracket()`, no child
//! prices are sent and TWS creates the children itself.
//!
//! `submit()` is fire-and-forget, so the outcome is read from the order update stream:
//! child `OpenOrder`s pointing at the parent, or error 10355 when no preset is defined, in
//! which case TWS discards the parent too.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example async_preset_attached_orders
//! ```

use std::time::Duration;

use futures::StreamExt;
use ibapi::contracts::Contract;
use ibapi::orders::OrderUpdate;
use ibapi::subscriptions::SubscriptionItem;
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    // Open the stream first so no update for the new orders is missed.
    let mut updates = client.order_update_stream().await?;

    let contract = Contract::stock("AAPL").build();
    let ids = client
        .order(&contract)
        .buy(100)
        .limit(150.0)
        .preset_stop_loss()
        .preset_profit_taker()
        .submit()
        .await?;
    println!("parent {} stop-loss {:?} profit-taker {:?}", ids.parent, ids.stop_loss, ids.profit_taker);

    let watch = async {
        while let Some(item) = updates.next().await {
            match item? {
                SubscriptionItem::Data(OrderUpdate::OpenOrder(o)) => {
                    println!(
                        "open order {} (parent {}): {} {:?}",
                        o.order_id, o.order.parent_id, o.order.order_type, o.order_state.status
                    );
                }
                SubscriptionItem::Data(OrderUpdate::OrderStatus(s)) => println!("status {}: {}", s.order_id, s.status),
                SubscriptionItem::Notice(n) if n.code == 10355 => {
                    println!("no preset defined, parent discarded: {}", n.message);
                    break;
                }
                SubscriptionItem::Notice(n) => println!("notice: {n}"),
                SubscriptionItem::Data(_) => {}
            }
        }
        Ok::<_, ibapi::Error>(())
    };
    // Watch for a few seconds; the children stay working at TWS after this exits.
    if let Ok(result) = tokio::time::timeout(Duration::from_secs(10), watch).await {
        result?;
    }
    Ok(())
}
