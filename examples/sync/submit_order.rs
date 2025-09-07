//! Example demonstrating how to submit multiple market orders and monitor order updates.
//!
//! This example connects to TWS/IB Gateway, starts a background thread to monitor order updates,
//! then submits a series of buy and sell orders for AAPL stock with 1 second delays between orders.
//!
//! The `submit_order` method is used to send orders, while `order_update_stream`
//! provides real-time updates about all orders including:
//! - Order status changes
//! - Execution/fill notifications
//! - Commission reports
//! - System messages

use ibapi::prelude::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let symbol = "AAPL";
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).expect("connection failed"));

    println!("Connected {client:?}");

    // Start background thread to monitor order updates
    let monitor_client = client.clone();
    let _monitor_handle = thread::spawn(move || {
        println!("Starting order monitoring thread...");

        match monitor_client.order_update_stream() {
            Ok(stream) => {
                for update in stream {
                    match update {
                        OrderUpdate::OrderStatus(status) => {
                            println!(
                                "[Monitor] Order {} Status: {} - Filled: {}/{}",
                                status.order_id, status.status, status.filled, status.remaining
                            );
                        }
                        OrderUpdate::OpenOrder(open_order) => {
                            println!(
                                "[Monitor] Open Order {}: {} {} shares of {} @ {}",
                                open_order.order_id,
                                open_order.order.action,
                                open_order.order.total_quantity,
                                open_order.contract.symbol,
                                open_order.order.order_type
                            );
                        }
                        OrderUpdate::ExecutionData(execution) => {
                            println!(
                                "[Monitor] Execution: {} {} shares @ {} on {}",
                                execution.execution.side, execution.execution.shares, execution.execution.price, execution.execution.exchange
                            );
                        }
                        OrderUpdate::CommissionReport(report) => {
                            println!("[Monitor] Commission: ${} for execution {}", report.commission, report.execution_id);
                        }
                        OrderUpdate::Message(message) => {
                            println!("[Monitor] Message: {}", message.message);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error creating order update stream: {e:?}");
            }
        }
    });

    // Give the monitoring thread time to start
    thread::sleep(Duration::from_millis(100));

    let contract = Contract::stock(symbol)
        .on_exchange(ibapi::contracts::Exchange::SMART)
        .in_currency(ibapi::contracts::Currency::USD)
        .build();

    // Place a series of buy and sell orders
    let order_quantities = [
        (Action::Buy, 100.0),
        (Action::Sell, 50.0),
        (Action::Buy, 75.0),
        (Action::Sell, 100.0),
        (Action::Buy, 25.0),
    ];

    for (i, (action, quantity)) in order_quantities.iter().enumerate() {
        let order_id = client.next_order_id();
        let order = order_builder::market_order(*action, *quantity);

        println!(
            "\n[Main] Placing order #{} (ID: {}) - {} {} shares of {}",
            i + 1,
            order_id,
            action,
            quantity,
            symbol
        );

        match client.submit_order(order_id, &contract, &order) {
            Ok(_) => println!("[Main] Order {order_id} submitted successfully"),
            Err(e) => eprintln!("[Main] Failed to submit order {order_id}: {e}"),
        }

        // Wait 1 second between orders
        if i < order_quantities.len() - 1 {
            println!("[Main] Waiting 1 second before next order...");
            thread::sleep(Duration::from_secs(1));
        }
    }

    println!("\n[Main] All orders submitted. Waiting for order updates...");

    // Wait a bit for final order updates to come through
    thread::sleep(Duration::from_secs(10));

    println!("[Main] Shutting down...");

    // Note: In a real application, you would want a more graceful shutdown mechanism
    // The monitoring thread will end when the main thread exits

    Ok(())
}
