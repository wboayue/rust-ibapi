#![allow(clippy::uninlined_format_args)]
//! Async histogram data example
//!
//! This example demonstrates how to retrieve histogram data (price distribution)
//! for a contract using the async API.
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_histogram_data
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the contract and period as desired

use std::sync::Arc;

use ibapi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway (port 4002) or TWS (port 7497)
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    // Test different contracts and periods
    let test_cases = vec![
        (
            "AAPL",
            Contract::stock("AAPL").build(),
            HistoricalBarSize::Week,
            TradingHours::Regular,
            "1 week RTH",
        ),
        (
            "SPY",
            Contract::stock("SPY").build(),
            HistoricalBarSize::Day,
            TradingHours::Regular,
            "1 day RTH",
        ),
        (
            "TSLA",
            Contract::stock("TSLA").build(),
            HistoricalBarSize::Week,
            TradingHours::Extended,
            "1 week all hours",
        ),
    ];

    for (symbol, contract, period, trading_hours, description) in test_cases {
        println!("\n{symbol} Histogram ({description}):");
        println!("Period: {period:?}, Trading hours: {trading_hours:?}");

        match client.histogram_data(&contract, trading_hours, period).await {
            Ok(histogram) => {
                if histogram.is_empty() {
                    println!("No histogram data available");
                    continue;
                }

                // Calculate statistics
                let total_count: f64 = histogram.iter().filter_map(|e| e.size).sum();
                let min_price = histogram.iter().map(|e| e.price).fold(f64::INFINITY, f64::min);
                let max_price = histogram.iter().map(|e| e.price).fold(f64::NEG_INFINITY, f64::max);

                // Calculate weighted average price
                let weighted_sum: f64 = histogram.iter().filter_map(|e| e.size.map(|s| e.price * s)).sum();
                let weighted_avg = weighted_sum / total_count;

                println!("\nPrice Distribution:");
                println!("Price     | Count    | Percentage | Bar");
                println!("----------|----------|------------|{}", "-".repeat(50));

                // Find max count for bar chart scaling
                // `max_by_key` needs `Ord`, which f64 doesn't implement — fold instead.
                let max_count = histogram.iter().filter_map(|e| e.size).fold(1.0_f64, f64::max);

                // Sort by price for better display
                let mut sorted_histogram = histogram.clone();
                sorted_histogram.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

                // Display top and bottom 10 price levels
                let display_count = 10;
                let total_entries = sorted_histogram.len();

                if total_entries <= display_count * 2 {
                    // Show all entries if small enough
                    for entry in &sorted_histogram {
                        print_histogram_entry(entry, total_count, max_count);
                    }
                } else {
                    // Show top and bottom entries with separator
                    println!("Top {display_count} price levels:");
                    for entry in sorted_histogram.iter().rev().take(display_count).rev() {
                        print_histogram_entry(entry, total_count, max_count);
                    }

                    println!("... ({} entries omitted) ...", total_entries - display_count * 2);

                    println!("Bottom {display_count} price levels:");
                    for entry in sorted_histogram.iter().take(display_count) {
                        print_histogram_entry(entry, total_count, max_count);
                    }
                }

                // Display statistics
                println!("\nStatistics:");
                println!("  Total observations: {total_count:.0}");
                println!("  Price range: ${min_price:.2} - ${max_price:.2}");
                println!("  Price levels: {}", histogram.len());
                println!("  Weighted average: ${weighted_avg:.2}");

                // Find mode (most frequent price)
                // Entries with no size reported are skipped rather than counted as zero.
                let mode = histogram.iter().filter_map(|e| e.size.map(|s| (e, s))).max_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((mode_entry, mode_size)) = mode {
                    let mode_pct = (mode_size / total_count) * 100.0;
                    println!("  Mode: ${:.2} ({mode_size:.0} occurrences, {mode_pct:.1}%)", mode_entry.price);
                }
            }
            Err(e) => {
                println!("Error: {e}");
            }
        }

        // Small delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    println!("\nExample completed!");
    Ok(())
}

fn print_histogram_entry(entry: &ibapi::market_data::historical::HistogramEntry, total_count: f64, max_count: f64) {
    // `None` means TWS reported no size for this bucket; it contributes nothing.
    let size = entry.size.unwrap_or(0.0);
    let percentage = (size / total_count) * 100.0;
    let bar_length = ((size / max_count) * 50.0) as usize;
    let bar = "█".repeat(bar_length);

    println!("${:8.2} | {size:8.0} | {percentage:9.2}% | {bar}", entry.price);
}
