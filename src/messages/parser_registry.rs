//! Message parser registry for decoding TWS protocol messages into structured fields
//!
//! This module provides a registry of parsers that can decode raw TWS messages
//! into human-readable field names and values, useful for debugging, logging,
//! and mock server development.

use super::{IncomingMessages, OutgoingMessages};
use std::collections::HashMap;

/// Represents a parsed field in a TWS message
#[derive(Debug, Clone)]
pub struct ParsedField {
    pub name: String,
    pub value: String,
}

/// Field definition for message parsing
type FieldTransform = Box<dyn Fn(&str) -> String + Send + Sync>;

pub struct FieldDef {
    index: usize,
    name: &'static str,
    transform: Option<FieldTransform>,
}

impl FieldDef {
    pub fn new(index: usize, name: &'static str) -> Self {
        Self {
            index,
            name,
            transform: None,
        }
    }

    pub fn with_transform<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.transform = Some(Box::new(f));
        self
    }
}

/// Message parser trait
pub trait MessageParser: Send + Sync {
    fn parse(&self, parts: &[&str]) -> Vec<ParsedField>;
}

/// Generic field-based parser
pub struct FieldBasedParser {
    fields: Vec<FieldDef>,
}

impl FieldBasedParser {
    pub fn new(fields: Vec<FieldDef>) -> Self {
        Self { fields }
    }
}

impl MessageParser for FieldBasedParser {
    fn parse(&self, parts: &[&str]) -> Vec<ParsedField> {
        let mut result = Vec::new();

        for field_def in &self.fields {
            if let Some(value) = parts.get(field_def.index) {
                let processed_value = if let Some(transform) = &field_def.transform {
                    transform(value)
                } else {
                    value.to_string()
                };

                result.push(ParsedField {
                    name: field_def.name.to_string(),
                    value: processed_value,
                });
            }
        }

        result
    }
}

/// Parser with special handling for timestamp fields
pub struct TimestampParser {
    base_parser: FieldBasedParser,
    timestamp_index: usize,
}

impl TimestampParser {
    pub fn new(base_parser: FieldBasedParser, timestamp_index: usize) -> Self {
        Self {
            base_parser,
            timestamp_index,
        }
    }
}

impl MessageParser for TimestampParser {
    fn parse(&self, parts: &[&str]) -> Vec<ParsedField> {
        let mut fields = self.base_parser.parse(parts);

        // Add parsed timestamp if available
        if let Some(timestamp) = parts.get(self.timestamp_index) {
            if let Ok(ts) = timestamp.parse::<i64>() {
                if let Ok(dt) = time::OffsetDateTime::from_unix_timestamp(ts) {
                    fields.push(ParsedField {
                        name: "timestamp_parsed".to_string(),
                        value: dt.to_string(),
                    });
                }
            }
        }

        fields
    }
}

/// Registry of message parsers
pub struct MessageParserRegistry {
    request_parsers: HashMap<OutgoingMessages, Box<dyn MessageParser>>,
    response_parsers: HashMap<IncomingMessages, Box<dyn MessageParser>>,
}

impl MessageParserRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            request_parsers: HashMap::new(),
            response_parsers: HashMap::new(),
        };

        registry.register_default_parsers();
        registry
    }

    fn register_default_parsers(&mut self) {
        // Register request parsers
        // RequestCurrentTime: message_type (49) + version (1)
        self.request_parsers.insert(
            OutgoingMessages::RequestCurrentTime,
            Box::new(FieldBasedParser::new(vec![FieldDef::new(0, "message_type"), FieldDef::new(1, "version")])),
        );

        // RequestManagedAccounts: message_type (17) + version (1)
        self.request_parsers.insert(
            OutgoingMessages::RequestManagedAccounts,
            Box::new(FieldBasedParser::new(vec![FieldDef::new(0, "message_type"), FieldDef::new(1, "version")])),
        );

        // RequestPositions: message_type (61) + version (1)
        self.request_parsers.insert(
            OutgoingMessages::RequestPositions,
            Box::new(FieldBasedParser::new(vec![FieldDef::new(0, "message_type"), FieldDef::new(1, "version")])),
        );

        // CancelPositions: message_type (64) + version (1)
        self.request_parsers.insert(
            OutgoingMessages::CancelPositions,
            Box::new(FieldBasedParser::new(vec![FieldDef::new(0, "message_type"), FieldDef::new(1, "version")])),
        );

        // RequestAccountSummary: message_type (62) + version (1) + request_id + group + tags
        self.request_parsers.insert(
            OutgoingMessages::RequestAccountSummary,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "version"),
                FieldDef::new(2, "request_id"),
                FieldDef::new(3, "group"),
                FieldDef::new(4, "tags"),
            ])),
        );

        // CancelAccountSummary: message_type (63) + version (1) + request_id
        self.request_parsers.insert(
            OutgoingMessages::CancelAccountSummary,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "version"),
                FieldDef::new(2, "request_id"),
            ])),
        );

        // RequestPnL: message_type (92) + request_id + account + model_code (empty)
        self.request_parsers.insert(
            OutgoingMessages::RequestPnL,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "request_id"),
                FieldDef::new(2, "account"),
                FieldDef::new(3, "model_code"),
            ])),
        );

        // CancelPnL: message_type (93) + request_id
        self.request_parsers.insert(
            OutgoingMessages::CancelPnL,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "request_id"),
            ])),
        );

        // RequestPnLSingle: message_type (94) + request_id + account + model_code + contract_id
        self.request_parsers.insert(
            OutgoingMessages::RequestPnLSingle,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "request_id"),
                FieldDef::new(2, "account"),
                FieldDef::new(3, "model_code"),
                FieldDef::new(4, "contract_id"),
            ])),
        );

        // CancelPnLSingle: message_type (95) + request_id
        self.request_parsers.insert(
            OutgoingMessages::CancelPnLSingle,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "request_id"),
            ])),
        );

        // Register response parsers
        // CurrentTime: message_type (49) + version (1) + timestamp
        self.response_parsers.insert(
            IncomingMessages::CurrentTime,
            Box::new(TimestampParser::new(
                FieldBasedParser::new(vec![
                    FieldDef::new(0, "message_type"),
                    FieldDef::new(1, "version"),
                    FieldDef::new(2, "timestamp"),
                ]),
                2, // timestamp_index
            )),
        );

        // Error: message_type (4) + version (2) + request_id + error_code + error_message
        self.response_parsers.insert(
            IncomingMessages::Error,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "version"),
                FieldDef::new(2, "request_id"),
                FieldDef::new(3, "error_code"),
                FieldDef::new(4, "error_message"),
            ])),
        );

        // ManagedAccounts: message_type (15) + version (1) + accounts (comma-separated)
        self.response_parsers.insert(
            IncomingMessages::ManagedAccounts,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "version"),
                FieldDef::new(2, "accounts"),
            ])),
        );

        // Position: message_type (61) + version + account + contract fields...
        self.response_parsers.insert(
            IncomingMessages::Position,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "version"),
                FieldDef::new(2, "account"),
                FieldDef::new(3, "contract_id"),
                FieldDef::new(4, "symbol"),
                FieldDef::new(5, "security_type"),
                FieldDef::new(6, "last_trade_date_or_contract_month"),
                FieldDef::new(7, "strike"),
                FieldDef::new(8, "right"),
                FieldDef::new(9, "multiplier"),
                FieldDef::new(10, "exchange"),
                FieldDef::new(11, "currency"),
                FieldDef::new(12, "local_symbol"),
                FieldDef::new(13, "trading_class"),
                FieldDef::new(14, "position"),
                FieldDef::new(15, "average_cost"),
            ])),
        );

        // PositionEnd: message_type (62) + version
        self.response_parsers.insert(
            IncomingMessages::PositionEnd,
            Box::new(FieldBasedParser::new(vec![FieldDef::new(0, "message_type"), FieldDef::new(1, "version")])),
        );

        // AccountSummary: message_type (63) + version + request_id + account + tag + value + currency
        self.response_parsers.insert(
            IncomingMessages::AccountSummary,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "version"),
                FieldDef::new(2, "request_id"),
                FieldDef::new(3, "account"),
                FieldDef::new(4, "tag"),
                FieldDef::new(5, "value"),
                FieldDef::new(6, "currency"),
            ])),
        );

        // AccountSummaryEnd: message_type (64) + version + request_id
        self.response_parsers.insert(
            IncomingMessages::AccountSummaryEnd,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "version"),
                FieldDef::new(2, "request_id"),
            ])),
        );

        // PnL: message_type (94) + request_id + daily_pnl + unrealized_pnl (optional) + realized_pnl (optional)
        self.response_parsers.insert(
            IncomingMessages::PnL,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "request_id"),
                FieldDef::new(2, "daily_pnl"),
                FieldDef::new(3, "unrealized_pnl"),
                FieldDef::new(4, "realized_pnl"),
            ])),
        );

        // PnLSingle: message_type (95) + request_id + position + daily_pnl + unrealized_pnl (optional) + realized_pnl (optional) + value
        self.response_parsers.insert(
            IncomingMessages::PnLSingle,
            Box::new(FieldBasedParser::new(vec![
                FieldDef::new(0, "message_type"),
                FieldDef::new(1, "request_id"),
                FieldDef::new(2, "position"),
                FieldDef::new(3, "daily_pnl"),
                FieldDef::new(4, "unrealized_pnl"),
                FieldDef::new(5, "realized_pnl"),
                FieldDef::new(6, "value"),
            ])),
        );
    }

    pub fn parse_request(&self, msg_type: OutgoingMessages, parts: &[&str]) -> Vec<ParsedField> {
        if let Some(parser) = self.request_parsers.get(&msg_type) {
            parser.parse(parts)
        } else {
            parse_generic_message(parts)
        }
    }

    pub fn parse_response(&self, msg_type: IncomingMessages, parts: &[&str]) -> Vec<ParsedField> {
        if let Some(parser) = self.response_parsers.get(&msg_type) {
            parser.parse(parts)
        } else {
            parse_generic_message(parts)
        }
    }

    /// Register a custom request parser
    pub fn register_request_parser(&mut self, msg_type: OutgoingMessages, parser: Box<dyn MessageParser>) {
        self.request_parsers.insert(msg_type, parser);
    }

    /// Register a custom response parser
    pub fn register_response_parser(&mut self, msg_type: IncomingMessages, parser: Box<dyn MessageParser>) {
        self.response_parsers.insert(msg_type, parser);
    }
}

impl Default for MessageParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a message generically when no specific parser is available
pub fn parse_generic_message(parts: &[&str]) -> Vec<ParsedField> {
    let mut fields = Vec::new();

    // First field is always message type
    if let Some(msg_type) = parts.first() {
        fields.push(ParsedField {
            name: "message_type".to_string(),
            value: msg_type.to_string(),
        });
    }

    // Remaining fields are generic
    for (i, part) in parts.iter().skip(1).enumerate() {
        // Skip the last empty part if it exists (from trailing \0)
        if i == parts.len() - 2 && part.is_empty() {
            continue;
        }
        fields.push(ParsedField {
            name: format!("field_{}", i + 2),
            value: part.to_string(),
        });
    }

    fields
}
