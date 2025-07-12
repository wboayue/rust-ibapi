//! Captures raw order response messages from TWS for test data generation

use ibapi::contracts::Contract;
use ibapi::orders::{order_builder, Action, PlaceOrder};
use ibapi::Client;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Enable message recording to capture raw responses
    env::set_var("IBAPI_RECORDING_DIR", "/tmp/order-responses");
    std::fs::create_dir_all("/tmp/order-responses")?;

    let client = Client::connect("127.0.0.1:4002", 100)?;
    println!("Connected to TWS/Gateway");

    // Get next valid order ID
    let order_id = client.next_valid_order_id()?;
    println!("Next valid order ID: {order_id:?}");

    // Create an ES futures order (trades 24/7)
    let mut contract = Contract::default();
    contract.symbol = "ES".to_string();
    contract.security_type = ibapi::contracts::SecurityType::Future;
    contract.exchange = "CME".to_string();
    contract.currency = "USD".to_string();
    contract.local_symbol = "ESU5".to_string(); // September 2025 contract

    let mut order = order_builder::limit_order(Action::Buy, 1.0, 5800.0); // 1 contract at $5800
    order.order_id = order_id;

    println!("\nPlacing order {} for 1 ESU5 contract at limit $5800...", order_id);

    // Place the order and capture responses
    let subscription = client.place_order(order_id, &contract, &order)?;

    println!("\nCapturing order responses:");
    let mut response_count = 0;

    // Collect responses for a few seconds
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 5 {
        if let Some(msg) = subscription.try_next() {
            response_count += 1;
            match msg {
                PlaceOrder::OpenOrder(ref data) => {
                    println!("\n[{}] OpenOrder:", response_count);
                    println!("  Order ID: {}", data.order.order_id);
                    println!("  Symbol: {}", data.contract.symbol);
                    println!("  Action: {:?}", data.order.action);
                    println!("  Quantity: {}", data.order.total_quantity);
                    println!("  Order Type: {}", data.order.order_type);
                }
                PlaceOrder::OrderStatus(ref status) => {
                    println!("\n[{}] OrderStatus:", response_count);
                    println!("  Order ID: {}", status.order_id);
                    println!("  Status: {}", status.status);
                    println!("  Filled: {}", status.filled);
                    println!("  Remaining: {}", status.remaining);
                }
                PlaceOrder::ExecutionData(ref exec) => {
                    println!("\n[{}] ExecutionData:", response_count);
                    println!("  Exec ID: {}", exec.execution.execution_id);
                    println!("  Order ID: {}", exec.execution.order_id);
                    println!("  Symbol: {}", exec.contract.symbol);
                    println!("  Side: {}", exec.execution.side);
                    println!("  Shares: {}", exec.execution.shares);
                    println!("  Price: {}", exec.execution.price);
                }
                PlaceOrder::CommissionReport(ref comm) => {
                    println!("\n[{}] CommissionReport:", response_count);
                    println!("  Exec ID: {}", comm.execution_id);
                    println!("  Commission: {}", comm.commission);
                    println!("  Currency: {}", comm.currency);
                }
                PlaceOrder::Message(ref notice) => {
                    println!("\n[{}] Notice/Error:", response_count);
                    println!("  Code: {}", notice.code);
                    println!("  Message: {}", notice.message);
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("\n\nNow let's cancel the order and capture cancel responses:");

    // Cancel the order
    let cancel_subscription = client.cancel_order(order_id, "")?;

    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 3 {
        if let Some(msg) = cancel_subscription.try_next() {
            response_count += 1;
            match msg {
                ibapi::orders::CancelOrder::OrderStatus(ref status) => {
                    println!("\n[{}] Cancel OrderStatus:", response_count);
                    println!("  Order ID: {}", status.order_id);
                    println!("  Status: {}", status.status);
                }
                ibapi::orders::CancelOrder::Notice(ref notice) => {
                    println!("\n[{}] Cancel Notice:", response_count);
                    println!("  Code: {}", notice.code);
                    println!("  Message: {}", notice.message);
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("\n\nRaw messages saved to: /tmp/order-responses/");
    println!("Check the incoming.log file for exact message formats");

    Ok(())
}
