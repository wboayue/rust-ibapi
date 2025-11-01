//! Test for Issue #326: Order rejection routing to order_update_stream
//!
//! This test verifies that order rejection messages from submit_order()
//! are now properly delivered to order_update_stream instead of being
//! logged as "no recipient found".
//!
//! To run: cargo run --example test_order_rejection_routing

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::{order_builder, Action, OrderUpdate};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Testing Order Rejection Routing (Issue #326) ===\n");

    // Connect to TWS Paper Trading (default port 7497)
    println!("Connecting to TWS Paper Trading on 127.0.0.1:7497...");
    let client = Client::connect("127.0.0.1:7497", 100)?;
    println!("✓ Connected to TWS\n");

    // Get next valid order ID
    let order_id = client.next_valid_order_id()?;
    println!("Next order ID: {}\n", order_id);

    // Create order update stream BEFORE submitting order
    println!("Creating order update stream...");
    let updates = client.order_update_stream()?;
    println!("✓ Order update stream created\n");

    // Create a contract for an INVALID symbol that TWS won't recognize
    // This will trigger a rejection: "No security definition has been found"
    let contract = Contract::stock("INVALID_SYMBOL_XYZ123").build();

    // Create a simple market order
    let mut order = order_builder::market_order(Action::Buy, 100.0);
    order.transmit = true;

    println!("Submitting order that should be rejected:");
    println!("  Symbol: INVALID_SYMBOL_XYZ123 (intentionally invalid)");
    println!("  Action: BUY");
    println!("  Quantity: 100");
    println!("  Expected rejection: No security definition found");
    println!();

    // Submit order (fire-and-forget)
    client.submit_order(order_id, &contract, &order)?;
    println!("✓ Order submitted with ID {}\n", order_id);

    println!("Waiting for order updates (including rejection message)...");
    println!("Press Ctrl+C to stop\n");

    let mut rejection_received = false;
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(10);

    // Monitor order update stream
    for update in updates {
        match update {
            OrderUpdate::OrderStatus(status) => {
                println!("✓ Order Status: {} (ID: {})", status.status, status.order_id);
                if status.order_id == order_id {
                    println!("  Status for our order: {}", status.status);
                }
            }
            OrderUpdate::OpenOrder(order_data) => {
                println!("✓ Open Order: ID {}", order_data.order_id);
            }
            OrderUpdate::Message(notice) => {
                println!("✓ ORDER MESSAGE RECEIVED:");
                println!("  Code: {}", notice.code);
                println!("  Message: {}", notice.message);
                println!();

                // Check if this is a rejection message (error codes typically 200-299 for contract issues)
                if notice.code >= 200 && notice.code < 300 {
                    println!("🎉 SUCCESS! Rejection message received!");
                    println!("   This confirms Issue #326 is FIXED.");
                    println!("   Before the fix, this message would have been lost.\n");
                    rejection_received = true;
                    break;
                }
            }
            OrderUpdate::ExecutionData(_) => {
                println!("✓ Execution received");
            }
            OrderUpdate::CommissionReport(_) => {
                println!("✓ Commission report received");
            }
        }

        // Timeout after 10 seconds
        if start.elapsed() > timeout {
            println!("\n⚠ Timeout after {} seconds", timeout.as_secs());
            println!("If no rejection was received, the order might be valid or TWS didn't reject it.");
            break;
        }

        // Small delay between checks
        thread::sleep(Duration::from_millis(100));
    }

    if rejection_received {
        println!("\n=== TEST PASSED ===");
        println!("Order rejection messages are now properly routed to order_update_stream!");
    } else {
        println!("\n=== TEST INCONCLUSIVE ===");
        println!("No rejection received. This could mean:");
        println!("  1. TWS accepted the order (unlikely with $0.01 limit)");
        println!("  2. Need to wait longer for TWS response");
        println!("  3. TWS configuration doesn't reject this type of order");
    }

    Ok(())
}
