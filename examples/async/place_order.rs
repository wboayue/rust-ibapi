//! Example demonstrating how to use place_order() with per-order subscriptions
//! 
//! This example shows how to:
//! 1. Place orders using place_order() which returns a subscription
//! 2. Monitor order status through the individual order subscription
//! 3. Handle multiple concurrent orders with separate subscriptions

use futures::StreamExt;
use ibapi::contracts::{Contract, SecurityType};
use ibapi::orders::{order_builder, place_order, PlaceOrder};
use ibapi::Client;
use std::error::Error;

#[tokio::main] 
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to server version {}", client.server_version());

    // Create a contract for Apple stock
    let mut contract = Contract::default();
    contract.symbol = "AAPL".to_string();
    contract.security_type = SecurityType::Stock;
    contract.exchange = "SMART".to_string();
    contract.currency = "USD".to_string();

    // Create a limit order to buy 100 shares
    let order = order_builder::limit_order("BUY", 100.0, 150.0);
    let order_id = client.next_order_id();
    
    println!("Placing order {} for {} {} @ {}", 
             order_id, order.total_quantity, contract.symbol, order.limit_price.unwrap());
    
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
            Ok(PlaceOrder::ExecutionData(exec)) => {
                println!("Execution:");
                println!("  Order ID: {}", exec.order_id);
                println!("  Symbol: {}", exec.contract.symbol);
                println!("  Side: {}", exec.side);
                println!("  Shares: {}", exec.shares);
                println!("  Price: {}", exec.price);
                println!("  Time: {}", exec.time);
            }
            Ok(PlaceOrder::CommissionReport(report)) => {
                println!("Commission Report:");
                println!("  Execution ID: {}", report.exec_id);
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
                eprintln!("Error in order subscription: {}", e);
                break;
            }
        }
        println!("---");
    }
    
    println!("\nExample demonstrating concurrent orders...\n");
    
    // Example of handling multiple orders concurrently
    let order1 = order_builder::limit_order("BUY", 50.0, 149.0);
    let order_id1 = client.next_order_id();
    
    let order2 = order_builder::limit_order("SELL", 75.0, 151.0);  
    let order_id2 = client.next_order_id();
    
    // Place both orders
    let subscription1 = place_order(&client, order_id1, &contract, &order1).await?;
    println!("Placed order {}", order_id1);
    
    let subscription2 = place_order(&client, order_id2, &contract, &order2).await?;
    println!("Placed order {}", order_id2);
    
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
    println!("Starting monitor for order {}", order_id);
    
    while let Some(update) = subscription.next().await {
        match update {
            Ok(PlaceOrder::OrderStatus(status)) => {
                println!("[Order {}] Status: {} - Filled: {}/{}", 
                         order_id, status.status, status.filled, 
                         status.filled + status.remaining);
                         
                if status.status == "Filled" || status.status == "Cancelled" {
                    break;
                }
            }
            Ok(PlaceOrder::ExecutionData(exec)) => {
                println!("[Order {}] Executed: {} shares @ {}", 
                         order_id, exec.shares, exec.price);
            }
            _ => {} // Ignore other updates for brevity
        }
    }
    
    println!("Monitor for order {} complete", order_id);
}