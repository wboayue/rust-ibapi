#![allow(clippy::uninlined_format_args)]
//! Example demonstrating order_update_stream() with the fluent submit path.
//!
//! This example shows how to:
//! 1. Create a global order update stream that receives all order-related events
//! 2. Submit orders via `client.order(&contract).buy(qty).<type>().submit().await`
//!    (fluent fire-and-forget; submit() allocates the order id internally)
//! 3. Monitor order status through the update stream

use futures::StreamExt;
use ibapi::contracts::Contract;
use ibapi::orders::OrderUpdate;
use ibapi::subscriptions::SubscriptionItemStreamExt;
use ibapi::Client;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to server version {}", client.server_version());

    // Create order update stream - this receives ALL order updates
    let order_stream = client.order_update_stream().await?;
    println!("Created order update stream");

    // Spawn a task to monitor all order updates
    let monitor_handle = tokio::spawn(async move {
        println!("Starting order update monitor...");

        let mut order_stream = order_stream.filter_data();
        while let Some(update) = order_stream.next().await {
            match update {
                Ok(OrderUpdate::OrderStatus(status)) => {
                    println!("Order Status Update:");
                    println!("  Order ID: {}", status.order_id);
                    println!("  Status: {}", status.status);
                    println!("  Filled: {}", status.filled);
                    println!("  Remaining: {}", status.remaining);
                    match status.average_fill_price {
                        Some(price) => println!("  Avg Fill Price: {price}"),
                        None => println!("  Avg Fill Price: -"),
                    }
                }
                Ok(OrderUpdate::OpenOrder(order_data)) => {
                    println!("Open Order Update:");
                    println!("  Order ID: {}", order_data.order_id);
                    println!("  Symbol: {}", order_data.contract.symbol);
                    println!("  Action: {:?}", order_data.order.action);
                    println!("  Quantity: {}", order_data.order.total_quantity);
                    println!("  Order Type: {}", order_data.order.order_type);
                    println!("  Status: {}", order_data.order_state.status);
                }
                Ok(OrderUpdate::ExecutionData(exec_data)) => {
                    println!("Execution:");
                    println!("  Order ID: {}", exec_data.execution.order_id);
                    println!("  Symbol: {}", exec_data.contract.symbol);
                    println!("  Side: {}", exec_data.execution.side);
                    println!("  Shares: {}", exec_data.execution.shares);
                    println!("  Price: {}", exec_data.execution.price);
                    println!("  Time: {}", exec_data.execution.time);
                }
                Ok(OrderUpdate::CommissionReport(report)) => {
                    println!("Commission Report:");
                    println!("  Execution ID: {}", report.execution_id);
                    println!("  Commission: {} {}", report.commission, report.currency);
                }
                Err(e) => {
                    eprintln!("Error in order stream: {e:?}");
                    break;
                }
            }
            println!("---");
        }

        println!("Order update monitor stopped");
    });

    // Give the monitor time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create a contract for Apple stock
    let contract = Contract::stock("AAPL").build();

    // Submit a limit buy via the fluent path. `submit()` is fire-and-forget and allocates
    // the order id internally; status flows through the `order_update_stream` above.
    let order_id = client.order(&contract).buy(100).limit(150.0).submit().await?;
    println!("\nSubmitted order: {order_id}");

    // Wait a bit to see order updates
    println!("\nWaiting for order updates...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Submit another order
    let order_id2 = client.order(&contract).sell(50).limit(160.0).submit().await?;
    println!("\nSubmitted order: {order_id2}");

    // Wait for more updates
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Cancel the monitoring task
    monitor_handle.abort();

    println!("\nExample complete");
    Ok(())
}
