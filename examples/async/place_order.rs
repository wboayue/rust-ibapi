#![allow(clippy::uninlined_format_args)]
//! Example demonstrating the canonical fluent order path with concurrent monitoring.
//!
//! - `client.order(&contract).buy(qty).limit(price).submit().await` is the canonical
//!   single-call submit. `submit()` allocates the order id internally and uses
//!   fire-and-forget delivery.
//! - All-order monitoring flows through `client.order_update_stream()`; this example
//!   spawns a single background task that reads from it.

use ibapi::prelude::*;
use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS.
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to server version {}", client.server_version());

    // Spawn a background task to monitor all order updates before submitting.
    let monitor_client = client.clone();
    let monitor_handle = tokio::spawn(async move {
        let mut stream = match monitor_client.order_update_stream().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to open order update stream: {e:?}");
                return;
            }
        };
        while let Some(update) = (&mut stream).filter_data().next().await {
            match update {
                Ok(OrderUpdate::OrderStatus(status)) => {
                    println!(
                        "[Monitor] order {} status: {} filled {}/{}",
                        status.order_id,
                        status.status,
                        status.filled,
                        status.filled + status.remaining
                    );
                }
                Ok(OrderUpdate::OpenOrder(o)) => {
                    println!(
                        "[Monitor] open order {}: {} {} of {}",
                        o.order_id, o.order.action, o.order.total_quantity, o.contract.symbol
                    );
                }
                Ok(OrderUpdate::ExecutionData(e)) => {
                    println!(
                        "[Monitor] execution: order {} {} {} @ {}",
                        e.execution.order_id, e.execution.side, e.execution.shares, e.execution.price
                    );
                }
                Ok(OrderUpdate::CommissionReport(r)) => {
                    println!("[Monitor] commission: ${} for {}", r.commission, r.execution_id);
                }
                Err(e) => {
                    eprintln!("[Monitor] error: {e:?}");
                    break;
                }
            }
        }
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let contract = Contract::stock("AAPL").build();

    // Submit a limit buy via the fluent path.
    let order_id = client.order(&contract).buy(100).limit(150.0).submit().await?;
    println!("Submitted order: {order_id}");

    // Submit two more orders concurrently using try_join!.
    let (order_id1, order_id2) = tokio::try_join!(
        client.order(&contract).buy(50).limit(149.0).submit(),
        client.order(&contract).sell(75).limit(151.0).submit(),
    )?;
    println!("Submitted orders: {order_id1}, {order_id2}");

    // Let the monitor task observe updates for up to 30 seconds, then abort it.
    let abort = monitor_handle.abort_handle();
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(30), monitor_handle).await;
    abort.abort();

    println!("\nExample complete");
    Ok(())
}
