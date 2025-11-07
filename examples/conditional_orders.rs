//! Demonstrates how to use conditional orders with the TWS API.
//!
//! This example shows all 6 types of order conditions supported by Interactive Brokers:
//! 1. Price Condition - Trigger when a security reaches a specific price
//! 2. Time Condition - Trigger at a specific date/time
//! 3. Margin Condition - Trigger based on account margin cushion
//! 4. Execution Condition - Trigger when another order executes
//! 5. Volume Condition - Trigger when volume reaches a threshold
//! 6. Percent Change Condition - Trigger based on price change percentage
//!
//! The example demonstrates both simple conditions and complex multi-condition scenarios
//! using AND/OR logic.
//!
//! # Note
//!
//! This example is designed to compile and demonstrate the API usage patterns.
//! To actually place orders, you would need to:
//! 1. Connect to TWS or IB Gateway
//! 2. Obtain valid contract IDs using the contract_details() API
//! 3. Use a valid order ID from next_order_id()
//! 4. Call client.place_order() with your contract and order
//!
//! # Usage
//!
//! ```bash
//! # Just compile to verify syntax (default features)
//! cargo build --example conditional_orders
//!
//! # Or with sync support
//! cargo build --features sync --example conditional_orders
//! ```

use ibapi::orders::conditions::TriggerMethod;
use ibapi::orders::{
    order_builder, Action, ExecutionCondition, MarginCondition, OrderCondition, PercentChangeCondition, PriceCondition, TimeCondition,
    VolumeCondition,
};

fn main() {
    println!("=== Conditional Orders Example ===\n");

    // Example 1: Price Condition
    // Trigger a buy order when AAPL price goes above $150
    println!("1. PRICE CONDITION");
    println!("   Scenario: Buy MSFT when price exceeds $350");

    let price_condition = PriceCondition::builder(
        272093,  // MSFT contract ID (example - use contract_details() to get real ID)
        "SMART", // Smart routing
    )
    .greater_than(350.0) // Trigger price
    .trigger_method(TriggerMethod::Last) // Use last price
    .build();

    let mut order = order_builder::market_order(Action::Buy, 100.0);
    order.conditions = vec![OrderCondition::Price(price_condition)];

    println!("   Order: Buy 100 shares when MSFT last price > $350.00");
    println!("   Condition: Price > 350.0, method=Last, exchange=SMART\n");

    // Example 2: Time Condition
    // Trigger an order after 2:30 PM Eastern Time
    println!("2. TIME CONDITION");
    println!("   Scenario: Submit order after 2:30 PM ET");

    let time_condition = TimeCondition::builder().greater_than("20251230 14:30:00 US/Eastern").build();

    let mut order = order_builder::limit_order(Action::Sell, 50.0, 155.0);
    order.conditions = vec![OrderCondition::Time(time_condition)];

    println!("   Order: Sell 50 shares at limit $155.00");
    println!("   Condition: Activate after 2:30 PM ET on Dec 30, 2025\n");

    // Example 3: Margin Condition
    // Trigger a protective order when margin cushion falls below 30%
    println!("3. MARGIN CONDITION");
    println!("   Scenario: Close position if margin cushion drops below 30%");

    let margin_condition = MarginCondition::builder()
        .less_than(30) // 30% threshold
        .build();

    let mut order = order_builder::market_order(Action::Sell, 200.0);
    order.conditions = vec![OrderCondition::Margin(margin_condition)];
    order.tif = ibapi::orders::TimeInForce::GoodTilCanceled; // Good-til-canceled

    println!("   Order: Sell 200 shares at market");
    println!("   Condition: Margin cushion < 30%");
    println!("   Purpose: Risk management / margin call protection\n");

    // Example 4: Execution Condition
    // Place a hedge order when an initial order executes
    println!("4. EXECUTION CONDITION");
    println!("   Scenario: Buy protective puts after buying stock");

    let execution_condition = ExecutionCondition::builder(
        "TSLA",  // Symbol
        "STK",   // Stock security type
        "SMART", // Exchange
    )
    .build();

    // This would be an order for put options to hedge the TSLA stock position
    let mut order = order_builder::limit_order(Action::Buy, 1.0, 5.50);
    order.conditions = vec![OrderCondition::Execution(execution_condition)];

    println!("   Order: Buy 1 put option contract at $5.50");
    println!("   Condition: Triggers when any TSLA stock order executes");
    println!("   Purpose: Automatic hedge/protection\n");

    // Example 5: Volume Condition
    // Enter position after sufficient liquidity is established
    println!("5. VOLUME CONDITION");
    println!("   Scenario: Trade after volume indicates sufficient liquidity");

    let volume_condition = VolumeCondition::builder(
        76792991, // TSLA contract ID (example)
        "SMART",  // Exchange
    )
    .greater_than(50_000_000) // 50 million shares
    .build();

    let mut order = order_builder::limit_order(Action::Buy, 100.0, 245.0);
    order.conditions = vec![OrderCondition::Volume(volume_condition)];

    println!("   Order: Buy 100 shares at limit $245.00");
    println!("   Condition: TSLA volume > 50,000,000 shares");
    println!("   Purpose: Ensure liquidity before trading\n");

    // Example 6: Percent Change Condition
    // Momentum trading - enter when significant move occurs
    println!("6. PERCENT CHANGE CONDITION");
    println!("   Scenario: Momentum trade when SPY moves significantly");

    let percent_change_condition = PercentChangeCondition::builder(
        756733,  // SPY contract ID (example)
        "SMART", // Exchange
    )
    .greater_than(2.0) // 2% change
    .build();

    let mut order = order_builder::limit_order(Action::Buy, 50.0, 452.0);
    order.conditions = vec![OrderCondition::PercentChange(percent_change_condition)];

    println!("   Order: Buy 50 shares at limit $452.00");
    println!("   Condition: SPY price change > 2% from open");
    println!("   Purpose: Momentum/breakout trading\n");

    // Example 7: Complex Scenario - Multiple Conditions with AND logic
    // All conditions must be true
    println!("7. COMPLEX SCENARIO - AND CONDITIONS");
    println!("   Scenario: Trade only when multiple conditions align");

    let price_cond = PriceCondition::builder(265598, "SMART")
        .greater_than(150.0)
        .conjunction(true) // AND with next condition
        .build();

    let volume_cond = VolumeCondition::builder(265598, "SMART")
        .greater_than(80_000_000)
        .conjunction(true) // AND with next condition
        .build();

    let time_cond = TimeCondition::builder()
        .greater_than("20251230 10:00:00 US/Eastern")
        .conjunction(true) // Last condition in AND chain
        .build();

    let mut order = order_builder::limit_order(Action::Buy, 100.0, 151.0);
    order.conditions = vec![
        OrderCondition::Price(price_cond),
        OrderCondition::Volume(volume_cond),
        OrderCondition::Time(time_cond),
    ];
    order.conditions_ignore_rth = false; // Only during regular trading hours

    println!("   Order: Buy 100 shares at limit $151.00");
    println!("   Conditions (ALL must be true):");
    println!("     - AAPL price > $150.00");
    println!("     - AAPL volume > 80,000,000");
    println!("     - Time after 10:00 AM ET");
    println!("   Purpose: High-confidence trade setup\n");

    // Example 8: Complex Scenario - Multiple Conditions with OR logic
    // Any condition can trigger
    println!("8. COMPLEX SCENARIO - OR CONDITIONS");
    println!("   Scenario: Close position if any risk threshold is breached");

    let margin_cond = MarginCondition::builder()
        .less_than(25)
        .conjunction(false) // OR with next condition
        .build();

    let price_cond = PriceCondition::builder(265598, "SMART")
        .less_than(140.0)
        .conjunction(false) // OR with next condition
        .build();

    let time_cond = TimeCondition::builder()
        .greater_than("20251230 15:55:00 US/Eastern")
        .conjunction(false) // Last condition in OR chain
        .build();

    let mut order = order_builder::market_order(Action::Sell, 100.0);
    order.conditions = vec![
        OrderCondition::Margin(margin_cond),
        OrderCondition::Price(price_cond),
        OrderCondition::Time(time_cond),
    ];
    order.conditions_cancel_order = false; // Activate (not cancel) when triggered
    order.tif = ibapi::orders::TimeInForce::GoodTilCanceled;

    println!("   Order: Sell 100 shares at market");
    println!("   Conditions (ANY can trigger):");
    println!("     - Margin cushion < 25%");
    println!("     - AAPL price < $140.00 (stop loss)");
    println!("     - After 3:55 PM ET (end of day exit)");
    println!("   Purpose: Risk management with multiple exit triggers\n");

    // Example 9: Cancel Order on Condition
    // Use conditions to automatically cancel an order
    println!("9. CANCEL ORDER ON CONDITION");
    println!("   Scenario: Cancel order if not filled by specific time");

    let time_cond = TimeCondition::builder().greater_than("20251230 15:30:00 US/Eastern").build();

    let mut order = order_builder::limit_order(Action::Buy, 100.0, 149.0);
    order.conditions = vec![OrderCondition::Time(time_cond)];
    order.conditions_cancel_order = true; // Cancel (not activate) when triggered
    order.tif = ibapi::orders::TimeInForce::Day;

    println!("   Order: Buy 100 shares at limit $149.00");
    println!("   Condition: Cancel after 3:30 PM ET if not filled");
    println!("   Purpose: Time-limited opportunity\n");

    // Example 10: Real-world Pairs Trading Strategy
    println!("10. REAL-WORLD STRATEGY - PAIRS TRADING");
    println!("    Scenario: Trade spread when correlation breaks");

    // Buy the laggard when the leader has moved significantly
    let leader_condition = PercentChangeCondition::builder(756733, "SMART")
        .greater_than(1.5)
        .conjunction(true)
        .build();

    let laggard_condition = PercentChangeCondition::builder(265598, "SMART").less_than(0.5).conjunction(true).build();

    let mut order = order_builder::limit_order(Action::Buy, 100.0, 150.0);
    order.conditions = vec![
        OrderCondition::PercentChange(leader_condition),
        OrderCondition::PercentChange(laggard_condition),
    ];

    println!("    Order: Buy 100 shares of AAPL at limit $150.00");
    println!("    Conditions:");
    println!("      - SPY (leader) up > 1.5% from open");
    println!("      - AAPL (laggard) up < 0.5% from open");
    println!("    Strategy: Buy laggard expecting it to catch up to market\n");

    println!("=== End of Examples ===");
    println!();
    println!("Key Points:");
    println!("- Each condition has a trigger direction (above/below, after/before)");
    println!("- Conditions can be chained with AND (conjunction=true) or OR (conjunction=false)");
    println!("- Use conditions_cancel_order to cancel instead of activate");
    println!("- Use conditions_ignore_rth to include after-hours monitoring");
    println!("- Contract IDs must be obtained via contract_details() API");
    println!();
    println!("To actually place these orders:");
    println!("1. Connect to TWS/Gateway: client.connect(...)");
    println!("2. Get valid contract IDs: client.contract_details(...)");
    println!("3. Get order ID: client.next_order_id()");
    println!("4. Place order: client.place_order(order_id, &contract, &order)");
}
