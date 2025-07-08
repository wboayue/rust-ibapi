//! Parses recorded TWS messages and formats them for use in tests

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <recording_dir>", args[0]);
        eprintln!("Example: {} /tmp/order-responses", args[0]);
        return Ok(());
    }

    let recording_dir = &args[1];
    let incoming_file = Path::new(recording_dir).join("incoming.log");

    if !incoming_file.exists() {
        eprintln!("No incoming.log file found in {}", recording_dir);
        return Ok(());
    }

    println!("Parsing messages from: {}", incoming_file.display());
    println!("\n// Add these to your test mock responses:\n");

    let file = fs::File::open(&incoming_file)?;
    let reader = BufReader::new(file);

    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Parse the timestamp and message
        if let Some(pos) = line.find("] ") {
            let message = &line[pos + 2..];
            
            // Convert null bytes to pipes for test format
            let formatted = message.replace('\0', "|");
            
            // Try to identify the message type
            let parts: Vec<&str> = formatted.split('|').collect();
            if !parts.is_empty() {
                let msg_type = identify_message_type(parts[0]);
                println!("// {} - Line {}", msg_type, i + 1);
                println!("\"{}\".to_string(),", formatted);
                println!();
            }
        }
    }

    Ok(())
}

fn identify_message_type(code: &str) -> &'static str {
    match code {
        "3" => "OrderStatus",
        "4" => "ErrorMessage",
        "5" => "OpenOrder",
        "8" => "NextValidId",
        "9" => "NextValidId",
        "11" => "ExecutionData",
        "53" => "OpenOrderEnd",
        "55" => "ExecutionDataEnd",
        "59" => "CommissionReport",
        "82" => "CompletedOrder",
        "83" => "CompletedOrdersEnd",
        _ => "Unknown",
    }
}