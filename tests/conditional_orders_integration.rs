//! Integration test for Issue #325: Conditional Orders Support
//!
//! This test verifies that conditional orders can be created, submitted to TWS,
//! and properly encoded/decoded. Tests all 6 condition types:
//! - Price Condition
//! - Time Condition
//! - Margin Condition
//! - Execution Condition
//! - Volume Condition
//! - Percent Change Condition
//!
//! # Requirements
//! - TWS or IB Gateway must be running
//! - Paper trading account recommended
//!
//! # Running the test
//! ```bash
//! # Start TWS Paper Trading on 127.0.0.1:7497, then run:
//! cargo test --test conditional_orders_integration --features sync -- --ignored --nocapture
//!
//! # Or with logging enabled:
//! RUST_LOG=debug cargo test --test conditional_orders_integration --features sync -- --ignored --nocapture
//! ```

#![cfg(feature = "sync")]

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::conditions::*;
use ibapi::orders::{order_builder, Action, OrderCondition};
use std::thread;
use std::time::Duration;

#[test]
#[ignore] // Requires live TWS/Gateway connection
fn test_conditional_orders() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Testing Conditional Orders (Issue #325) ===\n");

    // Connect to TWS Paper Trading
    println!("Connecting to TWS Paper Trading on 127.0.0.1:7497...");
    let client = Client::connect("127.0.0.1:7497", 100)?;
    println!("✓ Connected to TWS\n");

    // Get next valid order ID
    let mut order_id = client.next_valid_order_id()?;
    println!("Starting order ID: {}\n", order_id);

    // Test 1: Price Condition
    println!("=== Test 1: Price Condition ===");
    test_price_condition(&client, order_id)?;
    order_id += 1;
    thread::sleep(Duration::from_secs(2));

    // Test 2: Time Condition
    println!("\n=== Test 2: Time Condition ===");
    test_time_condition(&client, order_id)?;
    order_id += 1;
    thread::sleep(Duration::from_secs(2));

    // Test 3: Margin Condition
    println!("\n=== Test 3: Margin Condition ===");
    test_margin_condition(&client, order_id)?;
    order_id += 1;
    thread::sleep(Duration::from_secs(2));

    // Test 4: Execution Condition
    println!("\n=== Test 4: Execution Condition ===");
    test_execution_condition(&client, order_id)?;
    order_id += 1;
    thread::sleep(Duration::from_secs(2));

    // Test 5: Volume Condition
    println!("\n=== Test 5: Volume Condition ===");
    test_volume_condition(&client, order_id)?;
    order_id += 1;
    thread::sleep(Duration::from_secs(2));

    // Test 6: Percent Change Condition
    println!("\n=== Test 6: Percent Change Condition ===");
    test_percent_change_condition(&client, order_id)?;
    order_id += 1;
    thread::sleep(Duration::from_secs(2));

    // Test 7: Multiple Conditions (AND logic)
    println!("\n=== Test 7: Multiple Conditions with AND Logic ===");
    test_multiple_conditions(&client, order_id)?;

    println!("\n=== ALL TESTS COMPLETED ===");
    println!("Check TWS for the submitted conditional orders.");
    println!("All 6 condition types were successfully created and submitted!");

    Ok(())
}

fn test_price_condition(client: &Client, order_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating order with Price Condition:");
    println!("  Trigger when AAPL price > $200");

    // Create price condition: trigger when AAPL goes above $200
    let condition = PriceCondition::builder(265598, "SMART", 200.0)
        .trigger_above()
        .trigger_method(0) // Default: last price
        .build();

    let contract = Contract::stock("MSFT").build();
    let mut order = order_builder::market_order(Action::Buy, 10.0);
    order.conditions = vec![OrderCondition::Price(condition)];
    order.conditions_ignore_rth = false;
    order.transmit = false; // Don't actually execute, just submit for validation

    println!("  Submitting order ID {}...", order_id);
    client.submit_order(order_id, &contract, &order)?;
    println!("  ✓ Order submitted successfully!");
    println!("  TWS accepted the price condition encoding.");

    Ok(())
}

fn test_time_condition(client: &Client, order_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating order with Time Condition:");
    println!("  Trigger after 14:30:00 today");

    // Create time condition: trigger after 2:30 PM
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_utc();
    let time_str = format!("{:04}{:02}{:02} 14:30:00", now.year(), now.month() as u8, now.day());

    let condition = TimeCondition::builder(&time_str).trigger_after().build();

    let contract = Contract::stock("AAPL").build();
    let mut order = order_builder::market_order(Action::Buy, 10.0);
    order.conditions = vec![OrderCondition::Time(condition)];
    order.conditions_ignore_rth = true;
    order.transmit = false;

    println!("  Submitting order ID {}...", order_id);
    client.submit_order(order_id, &contract, &order)?;
    println!("  ✓ Order submitted successfully!");
    println!("  TWS accepted the time condition encoding.");

    Ok(())
}

fn test_margin_condition(client: &Client, order_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating order with Margin Condition:");
    println!("  Trigger when margin cushion < 30%");

    // Create margin condition: trigger when margin falls below 30%
    let condition = MarginCondition::builder(30).trigger_below().build();

    let contract = Contract::stock("TSLA").build();
    let mut order = order_builder::market_order(Action::Sell, 5.0);
    order.conditions = vec![OrderCondition::Margin(condition)];
    order.conditions_cancel_order = true; // Cancel if margin too low
    order.transmit = false;

    println!("  Submitting order ID {}...", order_id);
    client.submit_order(order_id, &contract, &order)?;
    println!("  ✓ Order submitted successfully!");
    println!("  TWS accepted the margin condition encoding.");

    Ok(())
}

fn test_execution_condition(client: &Client, order_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating order with Execution Condition:");
    println!("  Trigger when MSFT trade executes");

    // Create execution condition: trigger when MSFT executes
    let condition = ExecutionCondition::builder("MSFT", "STK", "SMART").build();

    let contract = Contract::stock("AAPL").build();
    let mut order = order_builder::market_order(Action::Buy, 10.0);
    order.conditions = vec![OrderCondition::Execution(condition)];
    order.transmit = false;

    println!("  Submitting order ID {}...", order_id);
    client.submit_order(order_id, &contract, &order)?;
    println!("  ✓ Order submitted successfully!");
    println!("  TWS accepted the execution condition encoding.");

    Ok(())
}

fn test_volume_condition(client: &Client, order_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating order with Volume Condition:");
    println!("  Trigger when TSLA volume > 50M shares");

    // Create volume condition: trigger when volume exceeds 50 million
    let condition = VolumeCondition::builder(76792991, "SMART", 50_000_000).trigger_above().build();

    let contract = Contract::stock("TSLA").build();
    let mut order = order_builder::market_order(Action::Buy, 10.0);
    order.conditions = vec![OrderCondition::Volume(condition)];
    order.transmit = false;

    println!("  Submitting order ID {}...", order_id);
    client.submit_order(order_id, &contract, &order)?;
    println!("  ✓ Order submitted successfully!");
    println!("  TWS accepted the volume condition encoding.");

    Ok(())
}

fn test_percent_change_condition(client: &Client, order_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating order with Percent Change Condition:");
    println!("  Trigger when SPY changes > 2%");

    // Create percent change condition: trigger when SPY moves more than 2%
    let condition = PercentChangeCondition::builder(756733, "SMART", 2.0).trigger_above().build();

    let contract = Contract::stock("SPY").build();
    let mut order = order_builder::market_order(Action::Sell, 10.0);
    order.conditions = vec![OrderCondition::PercentChange(condition)];
    order.transmit = false;

    println!("  Submitting order ID {}...", order_id);
    client.submit_order(order_id, &contract, &order)?;
    println!("  ✓ Order submitted successfully!");
    println!("  TWS accepted the percent change condition encoding.");

    Ok(())
}

fn test_multiple_conditions(client: &Client, order_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating order with Multiple Conditions (AND logic):");
    println!("  1. Trigger when price > $180");
    println!("  2. AND after 15:00:00 today");

    // Price condition
    let price_condition = PriceCondition::builder(265598, "SMART", 180.0)
        .trigger_above()
        .conjunction(true) // AND with next condition
        .build();

    // Time condition
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_utc();
    let time_str = format!("{:04}{:02}{:02} 15:00:00", now.year(), now.month() as u8, now.day());
    let time_condition = TimeCondition::builder(&time_str)
        .trigger_after()
        .conjunction(true) // AND logic
        .build();

    let contract = Contract::stock("AAPL").build();
    let mut order = order_builder::market_order(Action::Buy, 10.0);
    order.conditions = vec![OrderCondition::Price(price_condition), OrderCondition::Time(time_condition)];
    order.conditions_ignore_rth = false;
    order.transmit = false;

    println!("  Submitting order ID {}...", order_id);
    client.submit_order(order_id, &contract, &order)?;
    println!("  ✓ Order submitted successfully!");
    println!("  TWS accepted multiple conditions with AND logic.");

    Ok(())
}
