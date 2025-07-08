//! Captures raw responses for order listing operations (open orders, completed orders, executions)

use ibapi::orders::{ExecutionFilter, Executions, Orders};
use ibapi::Client;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Enable message recording
    env::set_var("IBAPI_RECORDING_DIR", "/tmp/order-list-responses");
    std::fs::create_dir_all("/tmp/order-list-responses")?;

    let client = Client::connect("127.0.0.1:4002", 100)?;
    println!("Connected to TWS/Gateway");

    // Test 1: Get open orders
    println!("\n=== Capturing Open Orders ===");
    let mut open_orders = client.open_orders()?;

    let mut count = 0;
    for result in open_orders {
        match result {
            Orders::OrderData(data) => {
                count += 1;
                println!("\n[{}] OrderData:", count);
                println!("  Order ID: {}", data.order.order_id);
                println!("  Symbol: {}", data.contract.symbol);
                println!("  Status: {}", data.order_state.status);
            }
            Orders::OrderStatus(status) => {
                count += 1;
                println!("\n[{}] OrderStatus:", count);
                println!("  Order ID: {}", status.order_id);
                println!("  Status: {}", status.status);
            }
            Orders::Notice(notice) => {
                count += 1;
                println!("\n[{}] Notice:", count);
                println!("  Code: {}", notice.code);
                println!("  Message: {}", notice.message);
            }
        }
    }

    if count == 0 {
        println!("No open orders found");
    }

    // Test 2: Get all open orders
    println!("\n\n=== Capturing All Open Orders ===");
    let mut all_orders = client.all_open_orders()?;

    count = 0;
    for result in all_orders {
        match result {
            Orders::OrderData(data) => {
                count += 1;
                println!("\n[{}] OrderData:", count);
                println!("  Order ID: {}", data.order.order_id);
                println!("  Symbol: {}", data.contract.symbol);
            }
            _ => {}
        }
    }

    // Test 3: Get completed orders (if server version supports it)
    if client.server_version() > 150 {
        // COMPLETED_ORDERS version
        println!("\n\n=== Capturing Completed Orders ===");
        match client.completed_orders(false) {
            Ok(mut completed) => {
                count = 0;
                for result in completed {
                    match result {
                        Orders::OrderData(data) => {
                            count += 1;
                            println!("\n[{}] Completed OrderData:", count);
                            println!("  Order ID: {}", data.order.order_id);
                            println!("  Symbol: {}", data.contract.symbol);
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => println!("Completed orders not supported: {}", e),
        }
    }

    // Test 4: Get executions
    println!("\n\n=== Capturing Executions ===");
    let filter = ExecutionFilter::default();
    let mut executions = client.executions(filter)?;

    count = 0;
    for result in executions {
        match result {
            Executions::ExecutionData(exec) => {
                count += 1;
                println!("\n[{}] ExecutionData:", count);
                println!("  Exec ID: {}", exec.execution.execution_id);
                println!("  Order ID: {}", exec.execution.order_id);
                println!("  Symbol: {}", exec.contract.symbol);
                println!("  Side: {}", exec.execution.side);
                println!("  Shares: {}", exec.execution.shares);
                println!("  Price: {}", exec.execution.price);
            }
            Executions::CommissionReport(comm) => {
                count += 1;
                println!("\n[{}] CommissionReport:", count);
                println!("  Exec ID: {}", comm.execution_id);
                println!("  Commission: {}", comm.commission);
            }
            Executions::Notice(notice) => {
                count += 1;
                println!("\n[{}] Notice:", count);
                println!("  Code: {}", notice.code);
                println!("  Message: {}", notice.message);
            }
        }
    }

    if count == 0 {
        println!("No executions found");
    }

    println!("\n\nRaw messages saved to: /tmp/order-list-responses/");
    println!("Check the incoming.log file for exact message formats");

    Ok(())
}
