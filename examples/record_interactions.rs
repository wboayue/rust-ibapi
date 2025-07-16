//! Records interactions between the Client and TWS server for mock server development
//!
//! This example demonstrates how to capture real interactions with a live TWS/Gateway
//! and save them in YAML format for building mock servers.

use core::str::FromStr;
use std::fs;
use std::path::Path;

use ibapi::messages::parser_registry::{MessageParserRegistry, ParsedField};
use ibapi::messages::*;
use ibapi::prelude::*;
use ibapi::trace;
use serde::{Deserialize, Serialize};

/// Represents a field in a TWS message
#[derive(Debug, Serialize, Deserialize)]
struct Field {
    name: String,
    value: String,
}

/// Represents a complete interaction between client and TWS
#[derive(Debug, Serialize, Deserialize)]
struct InteractionRecord {
    /// Name of the API call (e.g., "server_time")
    name: String,
    /// The request message sent to TWS
    request: MessageRecord,
    /// The response messages received from TWS
    responses: Vec<MessageRecord>,
}

/// Represents a single message (request or response)
#[derive(Debug, Serialize, Deserialize)]
struct MessageRecord {
    /// Raw message as sent/received
    raw: String,
    /// Parsed fields with descriptions
    fields: Vec<Field>,
}

// Sanitization module
mod sanitization {
    use super::Field;

    /// Sanitize sensitive data in a field value
    pub fn sanitize_field_value(value: &str) -> String {
        // Check if this might be an account ID
        if (value.starts_with("DU") || value.starts_with("U")) && value.len() > 5 {
            return "ACCOUNT_ID".to_string();
        }

        // Check for other sensitive patterns
        if value.len() > 20 && value.chars().all(|c| c.is_alphanumeric()) {
            // Might be a token or key
            return format!("{}...", &value[..6]);
        }

        value.to_string()
    }

    /// Sanitize the raw message by replacing sensitive data
    pub fn sanitize_raw_message(raw: &str, fields: &[Field]) -> String {
        let mut sanitized = raw.to_string();

        // Look for account IDs in the raw message
        let parts: Vec<&str> = raw.split('\0').collect();
        for (i, part) in parts.iter().enumerate() {
            if (part.starts_with("DU") || part.starts_with("U")) && part.len() > 5 {
                // This looks like an account ID
                if let Some(field) = fields.get(i) {
                    if field.value == "ACCOUNT_ID" {
                        // Replace in the raw message
                        sanitized = sanitized.replace(part, "ACCOUNT_ID");
                    }
                }
            }
        }

        sanitized
    }
}

use sanitization::{sanitize_field_value, sanitize_raw_message};

// Message parsing functions
fn parse_message_parts(raw: &str) -> Vec<&str> {
    // Don't filter out empty strings in the middle - they represent empty fields
    // But remove the last empty string if present (from trailing \0)
    let mut parts: Vec<&str> = raw.split('\0').collect();
    if parts.last() == Some(&"") {
        parts.pop();
    }
    parts
}

fn parse_request_fields(raw: &str, registry: &MessageParserRegistry) -> Vec<Field> {
    let parts = parse_message_parts(raw);

    if parts.is_empty() {
        return Vec::new();
    }

    match parts.get(0).map(|s| OutgoingMessages::from_str(s)) {
        Some(Ok(msg_type)) => {
            let mut parsed = registry.parse_request(msg_type, &parts);

            // Sanitize account field in requests
            if matches!(msg_type, OutgoingMessages::RequestPnL) {
                for field in &mut parsed {
                    if field.name == "account" {
                        field.value = sanitize_field_value(&field.value);
                    }
                }
            }

            convert_parsed_fields(parsed)
        }
        _ => {
            let parsed = parser_registry::parse_generic_message(&parts);
            convert_parsed_fields(parsed)
        }
    }
}

fn parse_response_fields(raw: &str, registry: &MessageParserRegistry) -> Vec<Field> {
    let parts = parse_message_parts(raw);

    if parts.is_empty() {
        return Vec::new();
    }

    match parts.get(0).map(|s| IncomingMessages::from_str(s)) {
        Some(Ok(msg_type)) => {
            let mut parsed = registry.parse_response(msg_type, &parts);

            // Special handling for sanitization
            match msg_type {
                IncomingMessages::ManagedAccounts => {
                    // Apply sanitization to accounts field
                    for field in &mut parsed {
                        if field.name == "accounts" {
                            field.value = field
                                .value
                                .split(',')
                                .map(|acc| sanitize_field_value(acc.trim()))
                                .collect::<Vec<_>>()
                                .join(",");
                        }
                    }
                }
                IncomingMessages::Position | IncomingMessages::AccountSummary | IncomingMessages::PnL => {
                    // Sanitize account field
                    for field in &mut parsed {
                        if field.name == "account" {
                            field.value = sanitize_field_value(&field.value);
                        }
                    }
                }
                _ => {}
            }

            convert_parsed_fields(parsed)
        }
        _ => {
            let parsed = parser_registry::parse_generic_message(&parts);
            convert_parsed_fields(parsed)
        }
    }
}

// Convert ParsedField to Field
fn convert_parsed_fields(parsed: Vec<ParsedField>) -> Vec<Field> {
    parsed
        .into_iter()
        .map(|pf| Field {
            name: pf.name,
            value: pf.value,
        })
        .collect()
}

/// Interaction recorder
struct InteractionRecorder {
    registry: MessageParserRegistry,
}

impl InteractionRecorder {
    fn new() -> Self {
        Self {
            registry: MessageParserRegistry::new(),
        }
    }

    fn record_interaction<F>(&self, name: &str, operation: F) -> Result<InteractionRecord, Box<dyn std::error::Error>>
    where
        F: FnOnce() -> Result<InteractionRecord, Box<dyn std::error::Error>>,
    {
        println!("Recording {} interaction...", name);

        // Clear any previous interaction
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Execute the operation and get the captured record
        let record = operation()?;

        Ok(record)
    }

    fn create_request_record(&self, raw: String) -> MessageRecord {
        let fields = parse_request_fields(&raw, &self.registry);
        let sanitized_raw = sanitize_raw_message(&raw, &fields);
        MessageRecord { raw: sanitized_raw, fields }
    }

    fn create_response_record(&self, raw: String) -> MessageRecord {
        let fields = parse_response_fields(&raw, &self.registry);
        let sanitized_raw = sanitize_raw_message(&raw, &fields);
        MessageRecord { raw: sanitized_raw, fields }
    }
}

/// Header information for the recording session
#[derive(Debug, Serialize, Deserialize)]
struct RecordingHeader {
    server_version: i32,
    recorded_at: String,
}

/// YAML output wrapper for proper serialization
#[derive(Serialize)]
struct YamlOutput {
    header: RecordingHeader,
    interactions: Vec<InteractionRecord>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable debug logging to activate trace recording
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    println!("Connecting to TWS/Gateway...");
    let client = Client::connect("127.0.0.1:4002", 100)?;
    println!("Connected successfully!");

    let recorder = InteractionRecorder::new();
    let mut interactions = Vec::new();

    // Record server_time interaction
    match recorder.record_interaction("server_time", || {
        let server_time = client.server_time()?;
        println!("Server time: {}", server_time);

        // Capture interaction immediately after the call
        let interaction = trace::last_interaction().ok_or("No interaction captured - ensure debug logging is enabled")?;

        let record = InteractionRecord {
            name: "server_time".to_string(),
            request: recorder.create_request_record(interaction.request),
            responses: interaction.responses.into_iter().map(|r| recorder.create_response_record(r)).collect(),
        };

        Ok(record)
    }) {
        Ok(record) => {
            println!("\nRecorded interaction: {}", record.name);
            println!("Request: {}", record.request.raw);
            println!("Responses: {} message(s)", record.responses.len());
            interactions.push(record);
        }
        Err(e) => eprintln!("Failed to record server_time: {}", e),
    }

    // Record managed_accounts interaction
    match recorder.record_interaction("managed_accounts", || {
        let accounts = client.managed_accounts()?;
        println!("Managed accounts: {:?} (will be sanitized)", accounts);

        // Capture interaction immediately
        let interaction = trace::last_interaction().ok_or("No interaction captured - ensure debug logging is enabled")?;

        let record = InteractionRecord {
            name: "managed_accounts".to_string(),
            request: recorder.create_request_record(interaction.request),
            responses: interaction.responses.into_iter().map(|r| recorder.create_response_record(r)).collect(),
        };

        Ok(record)
    }) {
        Ok(record) => {
            println!("\nRecorded interaction: {}", record.name);
            println!("Request: {}", record.request.raw);
            println!("Responses: {} message(s)", record.responses.len());
            interactions.push(record);
        }
        Err(e) => eprintln!("Failed to record managed_accounts: {}", e),
    }

    // Skip other interactions for now to debug
    // Get the first account for subsequent queries
    let accounts = client.managed_accounts().unwrap_or_default();
    let account = accounts.first().map(|s| s.as_str()).unwrap_or("DU1234567");
    println!("\nUsing account: {} for subsequent queries", account);

    // Record positions interaction
    match recorder.record_interaction("positions", || {
        let mut position_count = 0;
        let positions = client.positions()?;

        // Consume positions and capture trace before drop
        println!("Starting to consume positions...");
        while let Some(position) = positions.next() {
            if let PositionUpdate::PositionEnd = position {
                break;
            }
            position_count += 1;
            println!("Got position #{}", position_count);
        }
        println!("Finished consuming positions: {}", position_count);

        // Capture interaction before subscription is dropped
        let interaction = trace::last_interaction().ok_or("No interaction captured - ensure debug logging is enabled")?;

        // Drop subscription before creating record to avoid cancel message
        drop(positions);

        let record = InteractionRecord {
            name: "positions".to_string(),
            request: recorder.create_request_record(interaction.request),
            responses: interaction.responses.into_iter().map(|r| recorder.create_response_record(r)).collect(),
        };

        Ok(record)
    }) {
        Ok(record) => {
            println!("\nRecorded interaction: {}", record.name);
            println!("Request: {}", record.request.raw);
            println!("Responses: {} message(s)", record.responses.len());
            interactions.push(record);
        }
        Err(e) => eprintln!("Failed to record positions: {}", e),
    }

    // Record account_summary interaction
    match recorder.record_interaction("account_summary", || {
        use ibapi::accounts::types::AccountGroup;
        let mut summary_count = 0;
        let tags = vec!["NetLiquidation", "TotalCashValue", "GrossPositionValue"];
        let group = AccountGroup("All".to_string());
        let summaries = client.account_summary(&group, &tags)?;

        // Consume all summaries
        while let Some(summary) = summaries.next() {
            if let AccountSummaryResult::End = summary {
                break;
            }
            summary_count += 1;
            println!("Got summary #{}", summary_count);
        }
        println!("Finished consuming summaries: {}", summary_count);

        // Capture interaction before subscription is dropped
        let interaction = trace::last_interaction().ok_or("No interaction captured - ensure debug logging is enabled")?;

        // Drop subscription before creating record
        drop(summaries);

        let record = InteractionRecord {
            name: "account_summary".to_string(),
            request: recorder.create_request_record(interaction.request),
            responses: interaction.responses.into_iter().map(|r| recorder.create_response_record(r)).collect(),
        };

        Ok(record)
    }) {
        Ok(record) => {
            println!("\nRecorded interaction: {}", record.name);
            println!("Request: {}", record.request.raw);
            println!("Responses: {} message(s)", record.responses.len());
            interactions.push(record);
        }
        Err(e) => eprintln!("Failed to record account_summary: {}", e),
    }

    // Record pnl interaction
    match recorder.record_interaction("pnl", || {
        use ibapi::accounts::types::AccountId;
        let mut pnl_updates = 0;
        let account_id = AccountId(account.to_string());
        let pnl_stream = client.pnl(&account_id, None)?;

        // Read a few PnL updates
        for update in pnl_stream.into_iter().take(3) {
            pnl_updates += 1;
            println!("Got PnL update #{}: {:?}", pnl_updates, update);
        }
        println!("Finished consuming PnL updates: {}", pnl_updates);

        // Give it a moment to ensure trace is updated
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Capture interaction before subscription is dropped
        let interaction = trace::last_interaction().ok_or("No interaction captured - ensure debug logging is enabled")?;

        println!("PnL interaction request: {}", interaction.request);
        println!("PnL interaction responses: {} messages", interaction.responses.len());

        let record = InteractionRecord {
            name: "pnl".to_string(),
            request: recorder.create_request_record(interaction.request),
            responses: interaction.responses.into_iter().map(|r| recorder.create_response_record(r)).collect(),
        };

        Ok(record)
    }) {
        Ok(record) => {
            println!("\nRecorded interaction: {}", record.name);
            println!("Request: {}", record.request.raw);
            println!("Responses: {} message(s)", record.responses.len());
            interactions.push(record);
        }
        Err(e) => eprintln!("Failed to record pnl: {}", e),
    }

    // Create header with server version
    let header = RecordingHeader {
        server_version: client.server_version(),
        recorded_at: time::OffsetDateTime::now_utc().to_string(),
    };

    // Save all interactions to YAML file using serde_yaml
    let output_path = Path::new("tws_interactions.yaml");
    let yaml_output = YamlOutput { header, interactions };

    let yaml_content = serde_yaml::to_string(&yaml_output)?;
    fs::write(output_path, &yaml_content)?;

    println!("\nSaved to {}:", output_path.display());
    println!("{}", yaml_content);

    Ok(())
}
