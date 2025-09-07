#![allow(clippy::uninlined_format_args)]
//! Example demonstrating how to use place_order() with per-order subscriptions
//!
//! This example shows how to:
//! 1. Place orders using place_order() which returns a subscription
//! 2. Monitor order status through the individual order subscription
//! 3. Handle multiple concurrent orders with separate subscriptions

use ibapi::orders::{order_builder, place_order};
use ibapi::prelude::*;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to server version {}", client.server_version());

    // Create a contract for Apple stock
    let contract = Contract {
        symbol: Symbol::from("AAPL"),
        security_type: SecurityType::Stock,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        ..Default::default()
    };

    // Create a limit order to buy 100 shares
    let order = order_builder::limit_order(Action::Buy, 100.0, 150.0);
    let order_id = client.next_order_id();

    println!(
        "Placing order {} for {} {} @ {}",
        order_id,
        order.total_quantity,
        contract.symbol,
        order.limit_price.unwrap()
    );

    // Place the order and get a subscription for this specific order
    let mut order_subscription = place_order(&client, order_id, &contract, &order).await?;
    println!("Order placed successfully, monitoring updates...\n");

    // Monitor updates for this specific order
    while let Some(update) = order_subscription.next().await {
        match update {
            Ok(PlaceOrder::OrderStatus(status)) => {
                println!("Order Status Update:");
                println!("  Order ID: {}", status.order_id);
                println!("  Status: {}", status.status);
                println!("  Filled: {}", status.filled);
                println!("  Remaining: {}", status.remaining);
                println!("  Avg Fill Price: {}", status.average_fill_price);

                // Exit when order is filled or cancelled
                if status.status == "Filled" || status.status == "Cancelled" {
                    println!("\nOrder completed with status: {}", status.status);
                    break;
                }
            }
            Ok(PlaceOrder::OpenOrder(order_data)) => {
                println!("Open Order Update:");
                println!("  Order ID: {}", order_data.order_id);
                println!("  Symbol: {}", order_data.contract.symbol);
                println!("  Action: {:?}", order_data.order.action);
                println!("  Quantity: {}", order_data.order.total_quantity);
                println!("  Status: {}", order_data.order_state.status);
            }
            Ok(PlaceOrder::ExecutionData(exec_data)) => {
                println!("Execution:");
                println!("  Order ID: {}", exec_data.execution.order_id);
                println!("  Symbol: {}", exec_data.contract.symbol);
                println!("  Side: {}", exec_data.execution.side);
                println!("  Shares: {}", exec_data.execution.shares);
                println!("  Price: {}", exec_data.execution.price);
                println!("  Time: {}", exec_data.execution.time);
            }
            Ok(PlaceOrder::CommissionReport(report)) => {
                println!("Commission Report:");
                println!("  Execution ID: {}", report.execution_id);
                println!("  Commission: {} {}", report.commission, report.currency);
            }
            Ok(PlaceOrder::Message(notice)) => {
                println!("Order Message: {} - {}", notice.code, notice.message);

                // Check for error messages that indicate order completion
                if notice.code >= 200 && notice.code < 300 {
                    println!("Order rejected/cancelled");
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error in order subscription: {e}");
                break;
            }
        }
        println!("---");
    }

    println!("\nExample demonstrating concurrent orders...\n");

    // Example of handling multiple orders concurrently
    let order1 = order_builder::limit_order(Action::Buy, 50.0, 149.0);
    let order_id1 = client.next_order_id();

    let order2 = order_builder::limit_order(Action::Sell, 75.0, 151.0);
    let order_id2 = client.next_order_id();

    // Place both orders
    let subscription1 = place_order(&client, order_id1, &contract, &order1).await?;
    println!("Placed order {order_id1}");

    let subscription2 = place_order(&client, order_id2, &contract, &order2).await?;
    println!("Placed order {order_id2}");

    // Monitor both orders concurrently
    let handle1 = tokio::spawn(monitor_order(order_id1, subscription1));
    let handle2 = tokio::spawn(monitor_order(order_id2, subscription2));

    // Wait for both to complete (or timeout after 30 seconds)
    tokio::select! {
        _ = handle1 => println!("Order {} monitoring complete", order_id1),
        _ = handle2 => println!("Order {} monitoring complete", order_id2),
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
            println!("Timeout waiting for orders");
        }
    }

    println!("\nExample complete");
    Ok(())
}

async fn monitor_order(order_id: i32, mut subscription: ibapi::subscriptions::Subscription<PlaceOrder>) {
    println!("Starting monitor for order {order_id}");

    while let Some(update) = subscription.next().await {
        match update {
            Ok(PlaceOrder::OrderStatus(status)) => {
                println!(
                    "[Order {}] Status: {} - Filled: {}/{}",
                    order_id,
                    status.status,
                    status.filled,
                    status.filled + status.remaining
                );

                if status.status == "Filled" || status.status == "Cancelled" {
                    break;
                }
            }
            Ok(PlaceOrder::ExecutionData(exec_data)) => {
                println!(
                    "[Order {}] Executed: {} shares @ {}",
                    order_id, exec_data.execution.shares, exec_data.execution.price
                );
            }
            _ => {} // Ignore other updates for brevity
        }
    }

    println!("Monitor for order {order_id} complete");
}
