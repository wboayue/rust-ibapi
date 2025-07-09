//! Example demonstrating how to use order_update_stream() with submit_order()
//!
//! This example shows how to:
//! 1. Create a global order update stream that receives all order-related events
//! 2. Submit orders using the fire-and-forget submit_order() method
//! 3. Monitor order status through the update stream

use futures::StreamExt;
use ibapi::contracts::{Contract, SecurityType};
use ibapi::orders::{order_builder, order_update_stream, submit_order, Action, OrderUpdate};
use ibapi::Client;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to server version {}", client.server_version());

    // Create order update stream - this receives ALL order updates
    let mut order_stream = order_update_stream(&client).await?;
    println!("Created order update stream");

    // Spawn a task to monitor all order updates
    let monitor_handle = tokio::spawn(async move {
        println!("Starting order update monitor...");

        while let Some(update) = order_stream.next().await {
            match update {
                Ok(OrderUpdate::OrderStatus(status)) => {
                    println!("Order Status Update:");
                    println!("  Order ID: {}", status.order_id);
                    println!("  Status: {}", status.status);
                    println!("  Filled: {}", status.filled);
                    println!("  Remaining: {}", status.remaining);
                    println!("  Avg Fill Price: {}", status.average_fill_price);
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
                Ok(OrderUpdate::Message(notice)) => {
                    println!("Order Message: {} - {}", notice.code, notice.message);
                }
                Err(e) => {
                    eprintln!("Error in order stream: {}", e);
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
    let mut contract = Contract::default();
    contract.symbol = "AAPL".to_string();
    contract.security_type = SecurityType::Stock;
    contract.exchange = "SMART".to_string();
    contract.currency = "USD".to_string();

    // Create a limit order to buy 100 shares
    let order = order_builder::limit_order(Action::Buy, 100.0, 150.0);

    // Submit the order using fire-and-forget method
    let order_id = client.next_order_id();
    println!(
        "\nSubmitting order {} for {} {} @ {}",
        order_id,
        order.total_quantity,
        contract.symbol,
        order.limit_price.unwrap()
    );

    submit_order(&client, order_id, &contract, &order).await?;
    println!("Order submitted successfully");

    // Wait a bit to see order updates
    println!("\nWaiting for order updates...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Submit another order
    let order_id2 = client.next_order_id();
    let order2 = order_builder::limit_order(Action::Sell, 50.0, 160.0);

    println!(
        "\nSubmitting order {} for {} {} @ {}",
        order_id2,
        order2.total_quantity,
        contract.symbol,
        order2.limit_price.unwrap()
    );

    submit_order(&client, order_id2, &contract, &order2).await?;
    println!("Order submitted successfully");

    // Wait for more updates
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Cancel the monitoring task
    monitor_handle.abort();

    println!("\nExample complete");
    Ok(())
}
