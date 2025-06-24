//! Example demonstrating how to submit a market order and monitor order updates.
//!
//! This example connects to TWS/IB Gateway, submits a buy order for AAPL stock,
//! and monitors the order update stream for status updates, fills, and commission reports.
//!
//! The `submit_order` method is used to send the order, while `order_update_stream`
//! provides real-time updates about all orders including:
//! - Order status changes
//! - Execution/fill notifications
//! - Commission reports
//! - System messages

use ibapi::{contracts::ContractBuilder, prelude::*};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let symbol = "AAPL";
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    println!("Connected {client:?}");

    let contract = ContractBuilder::stock(symbol, "SMART", "USD").build().expect("invalid contract");
    let order = order_builder::market_order(Action::Buy, 100.0);

    let order_id = client.next_order_id();
    println!("Placing order with ID: {order_id} for {} of {symbol}", order.total_quantity);

    client.submit_order(order_id, &contract, &order).expect("could not submit order");

    // Monitor the order update stream for all order-related events
    // This will loop indefinitely, processing updates as they come in.
    // You would typically run this in a separate thread or use a timeout mechanism to exit gracefully.
    for update in client.order_update_stream()? {
        match update {
            PlaceOrder::OrderStatus(status) => println!("Order Status: {status:?}"),
            PlaceOrder::OpenOrder(open_order) => println!("Open Order: {open_order:?}"),
            PlaceOrder::ExecutionData(execution) => println!("Execution Data: {execution:?}"),
            PlaceOrder::CommissionReport(report) => println!("Commission Report: {report:?}"),
            PlaceOrder::Message(message) => println!("Message: {message:?}"),
        }
    }

    Ok(())
}
